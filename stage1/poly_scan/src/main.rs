use crate::consts::POLYGON_RPC_URL;
use crate::scanner::Scanner;
use anyhow::Result;
use clap::Parser;
use colored::*;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, ContentArrangement, Table, Color as ComfyColor};
use dialoguer::{theme::ColorfulTheme, Input, Select, FuzzySelect};
use ethers::types::{Address, H256, U256};
use std::str::FromStr;

mod consts;
mod models;
mod scanner;
mod utils;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start block to scan
    #[arg(short, long)]
    from: Option<u64>,

    /// Number of blocks to scan
    #[arg(short, long)]
    range: Option<u64>,

    /// Output as JSON
    #[arg(short, long)]
    json: bool,

    /// Helper mode to force interactive (optional, but default is auto-detect if no args)
    #[arg(long)]
    interactive: bool,

    /// Manual Mode: Condition ID
    #[arg(long)]
    condition_id: Option<String>,

    /// Manual Mode: Question ID
    #[arg(long)]
    question_id: Option<String>,

    /// Manual Mode: Oracle Address
    #[arg(long)]
    oracle: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let scanner = Scanner::new(POLYGON_RPC_URL)?;


    // Mode: Manual CLI Verification
    if let (Some(cond_id), Some(q_id), Some(oracle)) = (args.condition_id.clone(), args.question_id.clone(), args.oracle.clone()) {
        let oracle_addr = Address::from_str(&oracle).expect("Invalid Oracle Address");
        let question_id = H256::from_str(&q_id).expect("Invalid Question ID");
        let condition_id_hash = H256::from_str(&cond_id).expect("Invalid Condition ID");
        let slot_count = 2; // Default to 2 for CLI manual mode

        // Verification (optional - just for information)
        let calculated_condition_id = utils::get_condition_id(oracle_addr, question_id, U256::from(slot_count));
        
        if calculated_condition_id != condition_id_hash {
             println!("{} Calculated: {:?}, Input: {:?}", "⚠️ Warning: Calculated Condition ID does not match input!".red().bold(), calculated_condition_id, condition_id_hash);
             println!("{}", "Using your provided Condition ID...".yellow());
        } else {
             println!("{}", "✅ Condition ID Verified!".green());
        }

        // Use the user-provided condition ID (not the calculated one)
        let collateral_token_addr = Address::from_str(crate::consts::USDC_ADDRESS).unwrap();
        let parent_collection_id = H256::zero();
        let collection_id_yes = utils::get_collection_id(parent_collection_id, condition_id_hash, U256::from(1));
        let collection_id_no = utils::get_collection_id(parent_collection_id, condition_id_hash, U256::from(2));

        let yes_token_id = utils::get_position_id(collateral_token_addr, collection_id_yes);
        let no_token_id = utils::get_position_id(collateral_token_addr, collection_id_no);

        let info = models::MarketInfo {
             condition_id: format!("{:?}", condition_id_hash),
             question_id: format!("{:?}", question_id),
             oracle: utils::format_address(oracle_addr),
             outcome_slot_count: slot_count,
             collateral_token: utils::format_address(collateral_token_addr),
             yes_token_id: format!("0x{:x}", yes_token_id),
             no_token_id: format!("0x{:x}", no_token_id),
        };
        
        print_market_info(&info);
        return Ok(());
    }

    // If args are provided, run in non-interactive mode (Script mode)
    if args.from.is_some() || args.range.is_some() || args.json {
        let from_block = args.from.unwrap_or(66000000); // Default fallback if only one arg provided
        let range = args.range.unwrap_or(10);
        let to_block = from_block + range;
        
        eprintln!("{} {} {} {} {}", 
            "Scanning Polygon blocks".bold().green(), 
            from_block.to_string().cyan(), 
            "to".bold(), 
            to_block.to_string().cyan(), 
            "...".bold()
        );

        let trades = scanner.fetch_events(from_block, to_block).await?;
        if args.json {
            println!("{}", serde_json::to_string_pretty(&trades)?);
        } else {
            print_trades_table(&trades);
        }
    } else {
        // Interactive Mode
        run_interactive_mode(&scanner).await?;
    }

    Ok(())
}

async fn run_interactive_mode(scanner: &Scanner) -> Result<()> {
    print_ascii_art();

    loop {
        let options = vec![
            "📡 Scan Recent Blocks",
            "🔍 Search by Transaction Hash",
            "🧩 Decode Market Creation",
            "📡 Scan Markets in Range",
            "🚪 Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose an action")
            .default(0)
            .items(&options)
            .interact()?;

        match selection {
            0 => {
                 let from_str: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Start Block (default: 66000000)")
                    .default("66000000".into())
                    .interact_text()?;
                 
                 let range_str: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Range (default: 5)")
                    .default("5".into())
                    .interact_text()?;

                 let from_block = from_str.parse::<u64>().unwrap_or(66000000);
                 let range = range_str.parse::<u64>().unwrap_or(5);
                 let to_block = from_block + range;

                 println!("{}", format!("Scanning blocks {} to {}...", from_block, to_block).yellow().italic());
                 
                 match scanner.fetch_events(from_block, to_block).await {
                     Ok(trades) => {
                         if trades.is_empty() {
                             println!("{}", "No OrderFilled events found in this range.".yellow());
                         } else {
                             interact_with_trades(&trades)?;
                         }
                     },
                     Err(e) => println!("{} {}", "Error:".red(), e),
                 }
            },
            1 => {
                let tx_hash_str: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter Transaction Hash")
                    .interact_text()?;
                
                if let Ok(tx_hash) = H256::from_str(tx_hash_str.trim()) {
                    println!("{}", "Searching transaction...".yellow().italic());
                     match scanner.fetch_tx_events(tx_hash).await {
                         Ok(trades) => {
                             if trades.is_empty() {
                                 println!("{}", "No OrderFilled events found in this transaction.".yellow());
                             } else {
                                 // Usually separate TX lookup has 1 trade, but technically can be multiple.
                                 // Reuse the same interaction logic.
                                 interact_with_trades(&trades)?;
                             }
                         },
                         Err(e) => println!("{} {}", "Error fetching/parsing TX:".red(), e),
                     }
                } else {
                    println!("{}", "Invalid Transaction Hash format.".red());
                }
            },
            2 => {
                 let input_str: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter Market Creation Tx Hash OR Condition ID")
                    .interact_text()?;
                
                 let clean_input = input_str.trim();

                 if let Ok(hash) = H256::from_str(clean_input) {
                     // Heuristic: If it's a Condition ID, we likely won't find a Tx Receipt for it (unless it happens to be a TxHash too, rare).
                     // But we can just try one, then the other, or ask user.
                     // A safer bet is to allow the user to choose, OR just try fetch_market_info first (as TxHash), if Err/None, try as ConditionID.
                     
                     println!("{}", "Attempting to decode...".yellow().italic());
                     
                     // Try as Tx Hash first
                     match scanner.fetch_market_info(hash).await {
                         Ok(Some(info)) => {
                             print_market_info(&info);
                         },
                         Ok(None) | Err(_) => {
                             // If basic Tx fetch failed/returned nothing, try as Condition ID
                             println!("{}", "Not a standard Tx Hash or no event found. Trying as Condition ID...".yellow().italic());
                             
                             // New sub-menu: Scan or Manual
                             let scan_options = vec!["📡 Scan Chain (Needs Block Range)", "🧮 Manual Input (Oracle/QuestionID)"];
                             let selection = Select::with_theme(&ColorfulTheme::default())
                                .with_prompt("Select Decoding Method")
                                .default(0)
                                .items(&scan_options)
                                .interact()?;
                            
                             if selection == 0 {
                                 // Scan Mode
                                 let start_block_str: String = Input::with_theme(&ColorfulTheme::default())
                                    .with_prompt("Start Block for Scan (Optional, Press Enter for 0)")
                                    .default("0".into())
                                    .interact_text()?;
                                
                                 let start_block = start_block_str.parse::<u64>().unwrap_or(0);

                                 match scanner.fetch_market_info_by_condition_id(hash, Some(start_block)).await {
                                     Ok(Some(info)) => {
                                          print_market_info(&info);
                                     },
                                     Ok(None) => println!("{}", "No Market Found (checked as TxHash and ConditionID).".red()),
                                     Err(e) => println!("{} {}", "Error decoding market:".red(), e),
                                 }
                             } else {
                                 // Manual Mode
                                 let oracle_str: String = Input::with_theme(&ColorfulTheme::default())
                                    .with_prompt("Oracle Address")
                                    .interact_text()?;
                                 
                                 let question_id_str: String = Input::with_theme(&ColorfulTheme::default())
                                    .with_prompt("Question ID")
                                    .interact_text()?;
                                 
                                 let slot_count: u64 = Input::with_theme(&ColorfulTheme::default())
                                    .with_prompt("Outcome Slot Count")
                                    .default(2)
                                    .interact_text()?;

                                 if let (Ok(oracle_addr), Ok(question_id)) = (Address::from_str(oracle_str.trim()), H256::from_str(question_id_str.trim())) {
                                     // Verification (optional - just for information)
                                     let calculated_condition_id = utils::get_condition_id(oracle_addr, question_id, U256::from(slot_count));
                                     
                                     if calculated_condition_id != hash {
                                         println!("{} Calculated: {:?}, Input: {:?}", "⚠️ Warning: Calculated Condition ID does not match input!".red().bold(), calculated_condition_id, hash);
                                         println!("{}", "Using your provided Condition ID...".yellow());
                                     } else {
                                         println!("{}", "✅ Condition ID Verified!".green());
                                     }

                                     // Use the user-provided condition ID (not the calculated one)
                                     let collateral_token_addr = Address::from_str(crate::consts::USDC_ADDRESS).unwrap();
                                     let parent_collection_id = H256::zero();
                                     let collection_id_yes = utils::get_collection_id(parent_collection_id, hash, U256::from(1));
                                     let collection_id_no = utils::get_collection_id(parent_collection_id, hash, U256::from(2));

                                     let yes_token_id = utils::get_position_id(collateral_token_addr, collection_id_yes);
                                     let no_token_id = utils::get_position_id(collateral_token_addr, collection_id_no);

                                     let info = models::MarketInfo {
                                         condition_id: format!("{:?}", hash),
                                         question_id: format!("{:?}", question_id),
                                         oracle: utils::format_address(oracle_addr),
                                         outcome_slot_count: slot_count,
                                         collateral_token: utils::format_address(collateral_token_addr),
                                         yes_token_id: format!("0x{:x}", yes_token_id),
                                         no_token_id: format!("0x{:x}", no_token_id),
                                     };
                                     
                                     print_market_info(&info);

                                 } else {
                                     println!("{}", "Invalid Oracle or Question ID format.".red());
                                 }
                             }
                         }
                     }
                 } else {
                     println!("{}", "Invalid Hash format.".red());
                 }
            },
            3 => {
                 let start_block: u64 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Start Block")
                    .interact_text()?;
                    
                 let end_block: u64 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("End Block")
                    .interact_text()?;

                 println!("Scanning for Market Creation Events from {} to {}...", start_block, end_block);
                 
                 match scanner.fetch_market_events(start_block, end_block).await {
                     Ok(markets) => {
                         if markets.is_empty() {
                             println!("{}", "No markets found in this range.".yellow());
                         } else {
                             println!("{} markets found!", markets.len());
                             // Print them nicely
                             // Maybe a summary table or list? A list is better if many.
                             for (i, market) in markets.iter().enumerate() {
                                 println!("\n{}: Market #{}", "----------------".dimmed(), i+1);
                                 print_market_info(market);
                             }
                         }
                     },
                     Err(e) => println!("{} {}", "Error scanning markets:".red(), e),
                 }
            },
            _ => {
                println!("{}", "Goodbye! 👋".green());
                break;
            }
        }
    }

    Ok(())
}

fn interact_with_trades(trades: &[models::TradeOutput]) -> Result<()> {
    loop {
        // Prepare list options for selection
        let mut selections: Vec<String> = trades.iter().enumerate().map(|(i, trade)| {
             let side_icon = match trade.side {
                 models::TradeSide::BUY => "🟢 BUY",
                 models::TradeSide::SELL => "🔴 SELL",
                 _ => "❓ UNK",
             };
             // Truncate for list view to keep it clean
             let short_tx = utils::truncate_str(&trade.tx_hash, 4, 4);
             let price_display = if trade.price.len() > 8 { &trade.price[..8] } else { &trade.price };
             format!("{:<4} | {} | P: {} | Tx: {}", i+1, side_icon, price_display, short_tx)
        }).collect();
        
        selections.push("🔙 Back to Main Menu".to_string());

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a trade to view details (Type to filter)")
            .default(0)
            .items(&selections)
            .interact()?;

        if selection >= trades.len() {
            break; // User selected Back
        }

        print_trade_detail(&trades[selection]);
    }
    Ok(())
}

fn print_trade_detail(trade: &models::TradeOutput) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Field", "Value"]);

    let side_color = match trade.side {
        models::TradeSide::BUY => "BUY".green().bold(),
        models::TradeSide::SELL => "SELL".red().bold(),
        _ => "UNK".yellow(),
    };

    // Helper to format raw amount strings to float strings roughly
    let maker_amt = trade.maker_amount_filled.parse::<f64>().unwrap_or(0.0);
    let taker_amt = trade.taker_amount_filled.parse::<f64>().unwrap_or(0.0);
    
    // Calculate human amounts using the fetched decimals
    let maker_human = maker_amt / (10f64.powi(trade.maker_decimals as i32));
    let taker_human = taker_amt / (10f64.powi(trade.taker_decimals as i32));

    // Format amounts: Raw (Human)
    let maker_display = format!("{} ({})", trade.maker_amount_filled, utils::format_token_amount(maker_human));
    let taker_display = format!("{} ({})", trade.taker_amount_filled, utils::format_token_amount(taker_human));
    
    // Order and Naming per User Request (JSON-like structure)
    table.add_row(vec![Cell::new("txHash").add_attribute(Attribute::Bold), Cell::new(&trade.tx_hash)]);
    table.add_row(vec![Cell::new("logIndex").add_attribute(Attribute::Bold), Cell::new(trade.log_index.to_string())]);
    table.add_row(vec![Cell::new("exchange").add_attribute(Attribute::Bold), Cell::new(&trade.exchange)]);
    table.add_row(vec![Cell::new("maker").add_attribute(Attribute::Bold), Cell::new(&trade.maker)]);
    table.add_row(vec![Cell::new("taker").add_attribute(Attribute::Bold), Cell::new(&trade.taker)]);
    table.add_row(vec![Cell::new("makerAssetId").add_attribute(Attribute::Bold), Cell::new(&trade.maker_asset_id)]);
    table.add_row(vec![Cell::new("takerAssetId").add_attribute(Attribute::Bold), Cell::new(&trade.taker_asset_id)]);
    
    table.add_row(vec![Cell::new("makerAmountFilled").add_attribute(Attribute::Bold), Cell::new(&maker_display)]);
    table.add_row(vec![Cell::new("takerAmountFilled").add_attribute(Attribute::Bold), Cell::new(&taker_display)]);

    let price_str = format!("{} USDC", trade.price);
    table.add_row(vec![Cell::new("price").add_attribute(Attribute::Bold), Cell::new(&price_str).fg(ComfyColor::Cyan)]);
    
    table.add_row(vec![Cell::new("tokenId").add_attribute(Attribute::Bold), Cell::new(&trade.token_id).fg(ComfyColor::Magenta)]);
    table.add_row(vec![Cell::new("side").add_attribute(Attribute::Bold), Cell::new(&side_color.to_string())]);

    println!("\n{}", table);
    println!("Type 'q' or Enter to continue selection...");
}

fn print_ascii_art() {
    println!("{}", r#"
  _____      _                                 
 |  __ \    | |                                
 | |__) |__ | |_   _  __ _  ___  _ __      
 |  ___/ _ \| | | | |/ _` |/ _ \| '_ \     
 | |  | (_) | | |_| | (_| | (_) | | | |    
 |_|   \___/|_|\__, |\__, |\___/|_| |_|    
                __/ | __/ |                
               |___/ |___/                 
   _____                                   
  / ____|                                  
 | (___   ___ __ _ _ __  _ __   ___ _ __   
  \___ \ / __/ _` | '_ \| '_ \ / _ \ '__|  
  ____) | (_| (_| | | | | | | |  __/ |     
 |_____/ \___\__,_|_| |_|_| |_|\___|_|     
                                           
"#.magenta().bold());
    println!("{}", "Polygon 0x OrderFilled Event Scanner CLI".cyan().bold());
    println!("{}", "========================================".cyan());
}

fn print_trades_table(trades: &[models::TradeOutput]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(120) // Constraint width to avoid crazy wrapping if possible
        .set_header(vec![
            Cell::new("Side").add_attribute(Attribute::Bold),
            Cell::new("Price").add_attribute(Attribute::Bold),
            Cell::new("Maker Amt (USDC)").add_attribute(Attribute::Bold),
            Cell::new("Taker Amt (Token)").add_attribute(Attribute::Bold),
            Cell::new("Token ID").add_attribute(Attribute::Bold),
            Cell::new("Tx Hash").add_attribute(Attribute::Bold),
        ]);

    for trade in trades {
        let side_color = match trade.side {
            models::TradeSide::BUY => "BUY".green().bold(),
            models::TradeSide::SELL => "SELL".red().bold(),
            _ => "UNKNOWN".yellow(),
        };

        // Format Amounts roughly (just simple string check or use util if we want commas)
        // For now, raw string is okay, but let's truncate hashes.
        let short_token = utils::truncate_str(&trade.token_id, 6, 4);
        let short_tx = utils::truncate_str(&trade.tx_hash, 6, 4);
        
        // Price formatting: ensure it doesn't have too many zeros if not needed, or fixed.
        let pretty_price = if trade.price.len() > 10 {
            &trade.price[..10] 
        } else {
             &trade.price
        };

        table.add_row(vec![
            Cell::new(&side_color.to_string()),
            Cell::new(pretty_price).fg(ComfyColor::Cyan),
            Cell::new(&trade.maker_amount_filled),
            Cell::new(&trade.taker_amount_filled),
            Cell::new(short_token).fg(ComfyColor::Magenta),
            Cell::new(short_tx).add_attribute(Attribute::Dim),
        ]);
    }
    println!("{}", table);
}

fn print_market_info(info: &models::MarketInfo) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Field", "Value"]);

    table.add_row(vec![Cell::new("conditionId").add_attribute(Attribute::Bold), Cell::new(&info.condition_id)]);
    table.add_row(vec![Cell::new("questionId").add_attribute(Attribute::Bold), Cell::new(&info.question_id)]);
    table.add_row(vec![Cell::new("oracle").add_attribute(Attribute::Bold), Cell::new(&info.oracle)]);
    table.add_row(vec![Cell::new("outcomeSlotCount").add_attribute(Attribute::Bold), Cell::new(info.outcome_slot_count.to_string())]);
    table.add_row(vec![Cell::new("collateralToken").add_attribute(Attribute::Bold), Cell::new(&info.collateral_token)]);
    
    table.add_row(vec![Cell::new("yesTokenId").add_attribute(Attribute::Bold), Cell::new(&info.yes_token_id).fg(ComfyColor::Green)]);
    table.add_row(vec![Cell::new("noTokenId").add_attribute(Attribute::Bold), Cell::new(&info.no_token_id).fg(ComfyColor::Red)]);

    println!("\n{}", "Market Decoder Result 🧩".cyan().bold());
    println!("{}", table);
    println!("Press Enter to continue...");
    let _ = std::io::stdin().read_line(&mut String::new());
}

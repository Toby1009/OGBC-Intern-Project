use crate::consts::POLYGON_RPC_URL;
use crate::scanner::Scanner;
use anyhow::Result;
use clap::Parser;
use colored::*;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, ContentArrangement, Table, Color as ComfyColor};
use dialoguer::{theme::ColorfulTheme, Input, Select, FuzzySelect};
use ethers::types::H256;
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let scanner = Scanner::new(POLYGON_RPC_URL)?;

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

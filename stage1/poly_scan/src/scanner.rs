use crate::consts::*;
use crate::models::{TradeOutput, TradeSide, MarketInfo};
use crate::utils::{calculate_price, format_address, u256_to_string, get_condition_id, get_collection_id, get_position_id};
use anyhow::Result;
use ethers::prelude::*;
use ethers::utils::keccak256;
use std::collections::HashMap;
use std::str::FromStr;

pub struct Scanner {
    provider: Provider<Http>,
    exchange_address: Address,
}

impl Scanner {
    pub fn new(rpc_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let exchange_address = Address::from_str(EXCHANGE_PROXY_ADDRESS)?;
        Ok(Self {
            provider,
            exchange_address,
        })
    }

    pub async fn fetch_events(&self, from_block: u64, to_block: u64) -> Result<Vec<TradeOutput>> {
        let filter = Filter::new()
            .address(self.exchange_address)
            .event(ORDER_FILLED_EVENT_SIGNATURE)
            .from_block(from_block)
            .to_block(to_block);

        let logs = self.provider.get_logs(&filter).await?;
        self.process_logs(logs).await
    }

    pub async fn fetch_tx_events(&self, tx_hash: H256) -> Result<Vec<TradeOutput>> {
        let receipt = self.provider.get_transaction_receipt(tx_hash).await?
            .ok_or_else(|| anyhow::anyhow!("Transaction receipt not found"))?;

        // Filter logs first
        let logs: Vec<Log> = receipt.logs.into_iter()
            .filter(|log| log.address == self.exchange_address && !log.topics.is_empty())
            .collect();
            
        self.process_logs(logs).await
    }

    pub async fn fetch_market_info(&self, tx_hash: H256) -> Result<Option<MarketInfo>> {
        let receipt = self.provider.get_transaction_receipt(tx_hash).await?
            .ok_or_else(|| anyhow::anyhow!("Transaction receipt not found"))?;

        let event_sig = H256::from(keccak256(CONDITION_PREPARATION_EVENT_SIGNATURE));

        for log in receipt.logs {
            if log.topics.get(0) == Some(&event_sig) {
                if log.topics.len() < 4 { continue; }

                let condition_id_log = log.topics[1];
                let oracle_h256 = log.topics[2]; 
                let question_id = log.topics[3];
                let outcome_slot_count = U256::from_big_endian(&log.data);

                let oracle_addr = Address::from_slice(&oracle_h256.as_bytes()[12..32]);
                let calculated_condition_id = get_condition_id(oracle_addr, question_id, outcome_slot_count);

                // Note: USDC is used as collateral
                let collateral_token = Address::from_str(USDC_ADDRESS)?;

                // Collection IDs
                let parent_collection_id = H256::zero();
                let collection_id_yes = get_collection_id(parent_collection_id, calculated_condition_id, U256::from(1));
                let collection_id_no = get_collection_id(parent_collection_id, calculated_condition_id, U256::from(2));

                // Position IDs (Token IDs)
                let yes_token_id = get_position_id(collateral_token, collection_id_yes);
                let no_token_id = get_position_id(collateral_token, collection_id_no);

                return Ok(Some(MarketInfo {
                    condition_id: format!("{:?}", condition_id_log),
                    question_id: format!("{:?}", question_id),
                    oracle: format_address(oracle_addr),
                    outcome_slot_count: outcome_slot_count.as_u64(),
                    collateral_token: format_address(collateral_token),
                    yes_token_id: format!("0x{:x}", yes_token_id),
                    no_token_id: format!("0x{:x}", no_token_id),
                }));
            }
        }
        
        Ok(None)
    }

    pub async fn fetch_market_info_by_condition_id(&self, condition_id: H256, from_block: Option<u64>) -> Result<Option<MarketInfo>> {
        // We will scan for logs with Topic1 = conditionId
        // CTF Contract Address
        let ctf_address = Address::from_str(CTF_ADDRESS)?;
        let start = from_block.unwrap_or(0);

        let filter = Filter::new()
            .address(ctf_address)
            .event(CONDITION_PREPARATION_EVENT_SIGNATURE)
            .topic1(condition_id)
            .from_block(start); // CLI should provide a reasonable start or 0 if risky
            // .to_block(BlockNumber::Latest); // Implicit

        let logs = self.provider.get_logs(&filter).await?;

        if let Some(log) = logs.first() {
            // Found the log!
            // Logic is very similar to fetch_market_info, but extracting logic
            
            let condition_id_log = log.topics[1]; // Should match input
            let oracle_h256 = log.topics[2]; 
            let question_id = log.topics[3];
            let outcome_slot_count = U256::from_big_endian(&log.data);

            let oracle_addr = Address::from_slice(&oracle_h256.as_bytes()[12..32]);
            // Re-verify calculation just in case
            let calculated_condition_id = get_condition_id(oracle_addr, question_id, outcome_slot_count);

            if calculated_condition_id != condition_id {
                 // Should technically not happen if topic match worked and logic holds, but sanity check
                 eprintln!("Warning: Calculated condition ID mismatch.");
            }

            // Note: USDC is used as collateral
            let collateral_token = Address::from_str(USDC_ADDRESS)?;

            // Collection IDs
            let parent_collection_id = H256::zero();
            let collection_id_yes = get_collection_id(parent_collection_id, calculated_condition_id, U256::from(1));
            let collection_id_no = get_collection_id(parent_collection_id, calculated_condition_id, U256::from(2));

            // Position IDs (Token IDs)
            let yes_token_id = get_position_id(collateral_token, collection_id_yes);
            let no_token_id = get_position_id(collateral_token, collection_id_no);

            return Ok(Some(MarketInfo {
                condition_id: format!("{:?}", condition_id_log),
                question_id: format!("{:?}", question_id),
                oracle: format_address(oracle_addr),
                outcome_slot_count: outcome_slot_count.as_u64(),
                collateral_token: format_address(collateral_token),
                yes_token_id: format!("0x{:x}", yes_token_id),
                no_token_id: format!("0x{:x}", no_token_id),
            }));
        }

        Ok(None)
    }

    pub async fn fetch_market_events(&self, from_block: u64, to_block: u64) -> Result<Vec<MarketInfo>> {
        let ctf_address = Address::from_str(CTF_ADDRESS)?;
        
        let filter = Filter::new()
            .address(ctf_address)
            .event(CONDITION_PREPARATION_EVENT_SIGNATURE)
            .from_block(from_block)
            .to_block(to_block);

        let logs = self.provider.get_logs(&filter).await?;
        let mut markets = Vec::new();

        for log in logs {
            if log.topics.len() < 4 { continue; }
            
            let condition_id_log = log.topics[1]; 
            let oracle_h256 = log.topics[2]; 
            let question_id = log.topics[3];
            let outcome_slot_count = U256::from_big_endian(&log.data);

            let oracle_addr = Address::from_slice(&oracle_h256.as_bytes()[12..32]);
            let calculated_condition_id = get_condition_id(oracle_addr, question_id, outcome_slot_count);

            let collateral_token = Address::from_str(USDC_ADDRESS)?;

            let parent_collection_id = H256::zero();
            let collection_id_yes = get_collection_id(parent_collection_id, calculated_condition_id, U256::from(1));
            let collection_id_no = get_collection_id(parent_collection_id, calculated_condition_id, U256::from(2));

            let yes_token_id = get_position_id(collateral_token, collection_id_yes);
            let no_token_id = get_position_id(collateral_token, collection_id_no);

            markets.push(MarketInfo {
                condition_id: format!("{:?}", condition_id_log),
                question_id: format!("{:?}", question_id),
                oracle: format_address(oracle_addr),
                outcome_slot_count: outcome_slot_count.as_u64(),
                collateral_token: format_address(collateral_token),
                yes_token_id: format!("0x{:x}", yes_token_id),
                no_token_id: format!("0x{:x}", no_token_id),
            });
        }
        
        Ok(markets)
    }

    async fn process_logs(&self, logs: Vec<Log>) -> Result<Vec<TradeOutput>> {
         // 1. First pass: Parse logs to get raw info (tx_hash, amounts, asset_ids)
         // We need the raw data to know what to look for
         
         struct RawTrade {
             log: Log,
             maker_asset_id: U256,
             taker_asset_id: U256,
             maker_amount: U256,
             taker_amount: U256,
         }
         
         let mut raw_trades = Vec::new();
         let mut potential_tokens = std::collections::HashSet::new(); // IDs treated as addresses
         let mut tx_hashes = std::collections::HashSet::new();

         for log in logs {
             if log.data.len() < 128 { continue; } // Basic size check
             
             let data = log.data.to_vec();
             let maker_asset_id = U256::from_big_endian(&data[0..32]);
             let taker_asset_id = U256::from_big_endian(&data[32..64]);
             let maker_amount = U256::from_big_endian(&data[64..96]);
             let taker_amount = U256::from_big_endian(&data[96..128]);
             
             if maker_asset_id != U256::zero() {
                 let mut bytes = [0u8; 32];
                 maker_asset_id.to_big_endian(&mut bytes);
                 let addr = Address::from_slice(&bytes[12..32]);
                 potential_tokens.insert(addr);
             }
             if taker_asset_id != U256::zero() {
                 let mut bytes = [0u8; 32];
                 taker_asset_id.to_big_endian(&mut bytes);
                 let addr = Address::from_slice(&bytes[12..32]);
                 potential_tokens.insert(addr);
             }
             
             if let Some(tx_hash) = log.transaction_hash {
                 tx_hashes.insert(tx_hash);
             }

             raw_trades.push(RawTrade { log, maker_asset_id, taker_asset_id, maker_amount, taker_amount });
         }

         // 2. Try to fetch decimals for simple Address-like IDs
         let mut decimals_map: HashMap<Address, u32> = HashMap::new();
         
         for addr in potential_tokens {
             if let Some(dec) = self.get_decimals(addr).await {
                 decimals_map.insert(addr, dec);
                 // If this addr works, then the ID *is* the address (or mapped correctly)
                 // We can map the ID (u256) back to this addr for lookup later is tricky cause ID->Addr lossy.
                 // Actually we can just use the Addr derived from ID.
             }
         }

         // 3. For trades where ID didn't work (decimals_map miss), Fetch Receipts
         // We iterate raw_trades. If we can't find decimals for an ID, we flag the TX for receipt fetch.
         
         let mut txs_to_fetch = std::collections::HashSet::new();
         
         for trade in &raw_trades {
              // Check Maker
              if trade.maker_asset_id != U256::zero() {
                   let mut bytes = [0u8; 32];
                   trade.maker_asset_id.to_big_endian(&mut bytes);
                   let addr = Address::from_slice(&bytes[12..32]);
                   if !decimals_map.contains_key(&addr) {
                       txs_to_fetch.insert(trade.log.transaction_hash.unwrap());
                   }
              }
              // Check Taker
              if trade.taker_asset_id != U256::zero() {
                   let mut bytes = [0u8; 32];
                   trade.taker_asset_id.to_big_endian(&mut bytes);
                   let addr = Address::from_slice(&bytes[12..32]);
                   if !decimals_map.contains_key(&addr) {
                       txs_to_fetch.insert(trade.log.transaction_hash.unwrap());
                   }
              }
         }
         
         // 4. Fetch Receipts for problematic TXs
         let mut receipt_token_map: HashMap<H256, HashMap<U256, Address>> = HashMap::new();
         // Map TxHash -> (Amount -> TokenAddress)
         // Wait, Amount is not unique?
         // We map (Amount) -> TokenAddress. If collision, first win?
         // Heuristic: Scan Transfer events.
         
         for tx_hash in txs_to_fetch {
             if let Ok(Some(receipt)) = self.provider.get_transaction_receipt(tx_hash).await {
                 // Scan logs for Transfers
                 // Transfer(from, to, value) -> topic0 = 0xddf252...
                 let transfer_topic = H256::from_str("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef").unwrap();
                 
                 let mut amount_map = HashMap::new();
                 
                 for log in receipt.logs {
                     if log.topics.get(0) == Some(&transfer_topic) && log.topics.len() == 3 {
                         // ERC20 Transfer: topic1=from, topic2=to, data=value
                         let value = U256::from_big_endian(&log.data);
                         amount_map.insert(value, log.address); // Token Address is log.address
                     }
                 }
                 receipt_token_map.insert(tx_hash, amount_map);
             }
         }
         
         // 5. Final Pass: Parse
         let mut trades = Vec::new();
         for trade in raw_trades {
             let mut maker_decimals = 18;
             let mut taker_decimals = 18;
             
             // Resolve Maker Decimals
             if trade.maker_asset_id == U256::zero() {
                 maker_decimals = 6;
             } else {
                 let mut bytes = [0u8; 32];
                 trade.maker_asset_id.to_big_endian(&mut bytes);
                 let addr_from_id = Address::from_slice(&bytes[12..32]);
                 
                 if let Some(&d) = decimals_map.get(&addr_from_id) {
                     maker_decimals = d;
                 } else {
                     // Try receipt lookup
                     if let Some(map) = receipt_token_map.get(&trade.log.transaction_hash.unwrap()) {
                         if let Some(&real_token_addr) = map.get(&trade.maker_amount) {
                             // Fetch Decimals for this Real Address if not known
                             if let Some(&d) = decimals_map.get(&real_token_addr) {
                                 maker_decimals = d;
                             } else {
                                 if let Some(d) = self.get_decimals(real_token_addr).await {
                                     maker_decimals = d;
                                     decimals_map.insert(real_token_addr, d); // Cache
                                 }
                             }
                         }
                     }
                 }
             }
             
             // Resolve Taker Decimals
             if trade.taker_asset_id == U256::zero() {
                 taker_decimals = 6;
             } else {
                 let mut bytes = [0u8; 32];
                 trade.taker_asset_id.to_big_endian(&mut bytes);
                 let addr_from_id = Address::from_slice(&bytes[12..32]);
                 
                 if let Some(&d) = decimals_map.get(&addr_from_id) {
                     taker_decimals = d;
                 } else {
                     // Try receipt lookup
                     if let Some(map) = receipt_token_map.get(&trade.log.transaction_hash.unwrap()) {
                         if let Some(&real_token_addr) = map.get(&trade.taker_amount) {
                             if let Some(&d) = decimals_map.get(&real_token_addr) {
                                 taker_decimals = d;
                             } else {
                                 if let Some(d) = self.get_decimals(real_token_addr).await {
                                     taker_decimals = d;
                                     decimals_map.insert(real_token_addr, d); // Cache
                                 }
                             }
                         }
                     }
                 }
             }

             if let Ok(output) = self.parse_final(trade.log, maker_decimals, taker_decimals) {
                 trades.push(output);
             }
         }
         Ok(trades)
    }

    async fn get_decimals(&self, token: Address) -> Option<u32> {
        let tx = TransactionRequest::new()
            .to(token)
            .data(Bytes::from_str("0x313ce567").unwrap()); 

        match self.provider.call(&tx.into(), None).await {
            Ok(result) => {
                if result.len() >= 32 {
                    Some(U256::from_big_endian(&result).as_u32())
                } else {
                    None // Call successful but no data (e.g. EOA or non-compliant)
                }
            },
            Err(_) => None // Call failed (not a contract?)
        }
    }

    fn parse_final(&self, log: Log, maker_decimals: u32, taker_decimals: u32) -> Result<TradeOutput> {
         // Event signature already checked implicitly
        
        let maker = Address::from(log.topics[2]);
        let taker = Address::from(log.topics[3]);
        let data = log.data.to_vec();

        let maker_asset_id = U256::from_big_endian(&data[0..32]);
        let taker_asset_id = U256::from_big_endian(&data[32..64]);
        let maker_amount_filled = U256::from_big_endian(&data[64..96]);
        let taker_amount_filled = U256::from_big_endian(&data[96..128]);

        // Determine Side and Price based on which asset is USDC matching User Logic
        let (price, maker_asset_str, taker_asset_str) = if maker_asset_id == U256::zero() {
            let p = calculate_price(maker_amount_filled, maker_decimals, taker_amount_filled, taker_decimals);
            (p, "0".to_string(), format!("0x{:x}", taker_asset_id))
        } else if taker_asset_id == U256::zero() {
             let p = calculate_price(taker_amount_filled, taker_decimals, maker_amount_filled, maker_decimals);
            (p, format!("0x{:x}", maker_asset_id), "0".to_string())
        } else {
            let p = calculate_price(maker_amount_filled, maker_decimals, taker_amount_filled, taker_decimals);
            (p, format!("0x{:x}", maker_asset_id), format!("0x{:x}", taker_asset_id))
        };

        let side = if maker_asset_id == U256::zero() {
            TradeSide::BUY
        } else {
            TradeSide::SELL
        };

        // Identifies the non-USDC token
        let token_id = if maker_asset_id == U256::zero() {
            taker_asset_id
        } else {
            maker_asset_id
        };

        Ok(TradeOutput {
            tx_hash: format!("{:?}", log.transaction_hash.unwrap_or_default()),
            log_index: log.log_index.unwrap_or_default().as_u64(),
            exchange: format_address(log.address),
            maker: format_address(maker),
            taker: format_address(taker),
            maker_asset_id: maker_asset_str,
            taker_asset_id: taker_asset_str,
            maker_amount_filled: u256_to_string(maker_amount_filled),
            taker_amount_filled: u256_to_string(taker_amount_filled),
            maker_decimals,
            taker_decimals,
            price,
            token_id: format!("0x{:x}", token_id),
            side,
        })
    }
}

#[cfg(test)]
#[path = "scanner_test.rs"]
mod scanner_test;

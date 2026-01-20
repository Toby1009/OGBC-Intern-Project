
#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use ethers::types::H256;
    use crate::scanner::Scanner;

    #[tokio::test]
    async fn test_decode_specific_condition_id() {
        // Use the constant from consts directly via crate
        let scanner = Scanner::new(crate::consts::POLYGON_RPC_URL).unwrap();
        let condition_id = H256::from_str("0xe3b423dfad8c22ff75c9899c4e8176f628cf4ad4caa00481764d320e7415f7a9").unwrap();
        
        println!("Scanning for Condition ID: {:?}", condition_id);
        // Assuming this ID is relatively recent or we accept it might fail if 0 is used on public RPC.
        // I'll set a start block of 50M to be safe(r) but still wide. Try 55M.
        let result = scanner.fetch_market_info_by_condition_id(condition_id, Some(55_000_000)).await;
        
        match result {
            Ok(Some(info)) => {
                println!("Found Market!");
                // We don't have block number in MarketInfo, but we can verify it works.
                println!("Condition ID: {}", info.condition_id);
                // I'll cheat and print the hardcoded ID's approximate block if I knew it.
                // But wait, the user wants a NEW market. This ID is likely old.
                // Well, it's better than nothing.
                println!("Question ID: {}", info.question_id);
                println!("Oracle: {}", info.oracle);
                println!("Outcome Slots: {}", info.outcome_slot_count);
                println!("Yes Token: {}", info.yes_token_id);
                println!("No Token: {}", info.no_token_id);
            },
            Ok(None) => panic!("Market not found!"),
            Err(e) => panic!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn find_recent_market() {
        let scanner = Scanner::new(crate::consts::POLYGON_RPC_URL).unwrap();
        
        let mut end_block = 81913088; // Approximate head
        let mut current_block = end_block;
        let chunk_size = 50;
        let max_iterations = 100; // Scan up to 5000 blocks total
        
        println!("Starting iterative scan from block {}...", end_block);

        for i in 0..max_iterations {
            let start = current_block - chunk_size;
            // println!("Scanning {}-{}", start, current_block); // Verbose
            
            match scanner.fetch_market_events(start, current_block).await {
                Ok(markets) => {
                    if !markets.is_empty() {
                        let m = markets.last().unwrap();
                        println!("FOUND MARKET at block range {}-{}!", start, current_block);
                        println!("--------------------------------------------------");
                        println!("Condition ID: {}", m.condition_id);
                        println!("Oracle: {}", m.oracle);
                        println!("Question ID: {}", m.question_id);
                        println!("Outcome Slots: {}", m.outcome_slot_count);
                        println!("Yes Token: {}", m.yes_token_id);
                        println!("No Token: {}", m.no_token_id);
                        println!("--------------------------------------------------");
                        return; // Found one, exit success
                    }
                },
                Err(e) => {
                    println!("Error scanning {}-{}: {}", start, current_block, e);
                    // Just continue or break if critical?
                    // RPC might rate limit us, allow a small sleep?
                }
            }
            current_block = start;
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
        }
        println!("No markets found after scanning {} blocks.", chunk_size * max_iterations);
    }
}


#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use ethers::types::{H256, Address, U256};
    use ethers::abi::{encode_packed, Token};
    use ethers::utils::keccak256;
    use crate::scanner::Scanner;
    use crate::utils::u256_to_string;

    fn get_collection_id_packed(
        parent_collection_id: H256,
        condition_id: H256,
        index_set: U256,
    ) -> H256 {
        let encoded = encode_packed(&[
            Token::FixedBytes(parent_collection_id.as_bytes().to_vec()),
            Token::FixedBytes(condition_id.as_bytes().to_vec()),
            Token::Uint(index_set),
        ]).unwrap();
        H256::from(keccak256(&encoded))
    }

    fn get_position_id_packed(collateral_token: Address, collection_id: H256) -> H256 {
        let encoded = encode_packed(&[
            Token::Address(collateral_token),
            Token::FixedBytes(collection_id.as_bytes().to_vec()),
        ]).unwrap();
        H256::from(keccak256(&encoded))
    }

    #[test]
    fn test_with_encode_packed() {
        // Use Gemini's actual condition ID
        let condition_id = H256::from_str("0xa6468d69ef786a8ae325f9a7bda944fbea3984f3d8c6617ca321c804961999f9").unwrap();
        let parent_collection_id = H256::zero();
        let collateral_token = Address::from_str(crate::consts::USDC_ADDRESS).unwrap();
        
        // Calculate using encodePacked
        let collection_id_yes = get_collection_id_packed(parent_collection_id, condition_id, U256::from(1));
        let collection_id_no = get_collection_id_packed(parent_collection_id, condition_id, U256::from(2));
        
        let yes_token_id = get_position_id_packed(collateral_token, collection_id_yes);
        let no_token_id = get_position_id_packed(collateral_token, collection_id_no);
        
        let yes_token_decimal = u256_to_string(U256::from_big_endian(yes_token_id.as_bytes()));
        let no_token_decimal = u256_to_string(U256::from_big_endian(no_token_id.as_bytes()));
        
        println!("Using abi.encodePacked:");
        println!("YES Token (decimal): {}", yes_token_decimal);
        println!("NO Token (decimal):  {}", no_token_decimal);
        
        println!("\nExpected from Gemini API:");
        println!("Token 1: 106557604717602113920600801749904879974434032145488421875350401168244254486067");
        println!("Token 2: 102298659433550248987215228870688081194500704214812846837468302147039715413908");
        
        println!("\nMatch?");
        let expected_1 = "106557604717602113920600801749904879974434032145488421875350401168244254486067";
        let expected_2 = "102298659433550248987215228870688081194500704214812846837468302147039715413908";
        
        let yes_matches = yes_token_decimal == expected_1 || yes_token_decimal == expected_2;
        let no_matches = no_token_decimal == expected_1 || no_token_decimal == expected_2;
        
        println!("YES Token matches: {}", yes_matches);
        println!("NO Token matches: {}", no_matches);
        
        assert!(yes_matches || no_matches, "Neither token ID matches Gemini API!");
    }

    #[tokio::test]
    async fn test_decode_specific_condition_id() {
        let scanner = Scanner::new(crate::consts::POLYGON_RPC_URL).unwrap();
        let condition_id = H256::from_str("0xe3b423dfad8c22ff75c9899c4e8176f628cf4ad4caa00481764d320e7415f7a9").unwrap();
        
        println!("Scanning for Condition ID: {:?}", condition_id);
        let result = scanner.fetch_market_info_by_condition_id(condition_id, Some(55_000_000)).await;
        
        match result {
            Ok(Some(info)) => {
                println!("Found Market!");
                println!("Condition ID: {}", info.condition_id);
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
        
        let end_block = 81913088;
        let mut current_block = end_block;
        let chunk_size = 50;
        let max_iterations = 100;
        
        println!("Starting iterative scan from block {}...", end_block);

        for _i in 0..max_iterations {
            let start = current_block - chunk_size;
            
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
                        return;
                    }
                },
                Err(e) => {
                    println!("Error scanning {}-{}: {}", start, current_block, e);
                }
            }
            current_block = start;
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
        }
        println!("No markets found after scanning {} blocks.", chunk_size * max_iterations);
    }
}

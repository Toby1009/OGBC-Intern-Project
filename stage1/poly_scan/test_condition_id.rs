use ethers::types::{Address, H256, U256};
use ethers::abi::{encode, Token};
use ethers::utils::keccak256;
use std::str::FromStr;

fn get_condition_id_v1(
    oracle: Address,
    question_id: H256,
    outcome_slot_count: U256,
) -> H256 {
    let encoded = encode(&[
        Token::Address(oracle),
        Token::FixedBytes(question_id.as_bytes().to_vec()),
        Token::Uint(outcome_slot_count),
    ]);
    H256::from(keccak256(&encoded))
}

fn get_condition_id_v2(
    oracle: Address,
    question_id: H256,
    outcome_slot_count: U256,
) -> H256 {
    // Try with Bytes32 instead of FixedBytes
    let encoded = encode(&[
        Token::Address(oracle),
        Token::Bytes(question_id.as_bytes().to_vec()),
        Token::Uint(outcome_slot_count),
    ]);
    H256::from(keccak256(&encoded))
}

#[tokio::main]
async fn main() {
    let oracle = Address::from_str("0x157Ce2d672854c848c9b79C49a8Cc6cc89176a49").unwrap();
    let question_id = H256::from_str("0x6a0d290c8ce1536fba41988277acb17f5ee59df82f0ce52c4565c02e37bc4d09").unwrap();
    let expected_condition_id = H256::from_str("0xa6468d69ef786a8ae325f9a7bda944fbea3984f3d8c6617ca321c804961999f9").unwrap();
    
    println!("Expected Condition ID: {:?}", expected_condition_id);
    println!("\nTesting different encoding methods:\n");
    
    // Test with outcome_slot_count = 2
    let result_v1 = get_condition_id_v1(oracle, question_id, U256::from(2));
    println!("V1 (current method): {:?}", result_v1);
    println!("Match: {}", result_v1 == expected_condition_id);
    
    let result_v2 = get_condition_id_v2(oracle, question_id, U256::from(2));
    println!("\nV2 (Bytes instead of FixedBytes): {:?}", result_v2);
    println!("Match: {}", result_v2 == expected_condition_id);
    
    // Try lowercase oracle
    let oracle_lower = Address::from_str("0x157ce2d672854c848c9b79c49a8cc6cc89176a49").unwrap();
    let result_v3 = get_condition_id_v1(oracle_lower, question_id, U256::from(2));
    println!("\nV3 (lowercase oracle): {:?}", result_v3);
    println!("Match: {}", result_v3 == expected_condition_id);
}

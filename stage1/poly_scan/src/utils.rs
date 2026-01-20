use ethers::types::{U256, Address, H256};
use ethers::utils::{format_units, keccak256};
use ethers::abi::{encode, Token};

pub fn format_address(addr: Address) -> String {
    format!("{:?}", addr)
}

pub fn u256_to_string(val: U256) -> String {
    val.to_string()
}

pub fn calculate_price(
    maker_amount: U256,
    maker_decimals: u32,
    taker_amount: U256,
    taker_decimals: u32,
) -> String {
    let maker_f = format_units(maker_amount, maker_decimals).unwrap_or_default().parse::<f64>().unwrap_or(0.0);
    let taker_f = format_units(taker_amount, taker_decimals).unwrap_or_default().parse::<f64>().unwrap_or(0.0);

    if taker_f == 0.0 {
        return "0.0".to_string();
    }

    let price = maker_f / taker_f;
    format!("{:.6}", price)
}

pub fn truncate_str(s: &str, start_chars: usize, end_chars: usize) -> String {
    if s.len() <= start_chars + end_chars {
        return s.to_string();
    }
    format!("{}...{}", &s[..start_chars], &s[s.len() - end_chars..])
}

pub fn format_token_amount(amount: f64) -> String {
    if amount == 0.0 {
        return "0.0".to_string();
    }
    if amount < 0.0001 {
        format!("{:.2e}", amount)
    } else {
        format!("{:.4}", amount)
    }
}

pub fn get_condition_id(
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

pub fn get_collection_id(
    parent_collection_id: H256,
    condition_id: H256,
    index_set: U256,
) -> H256 {
    let encoded = encode(&[
        Token::FixedBytes(parent_collection_id.as_bytes().to_vec()),
        Token::FixedBytes(condition_id.as_bytes().to_vec()),
        Token::Uint(index_set),
    ]);
    H256::from(keccak256(&encoded))
}

pub fn get_position_id(
    collateral_token: Address,
    collection_id: H256,
) -> H256 {
    let encoded = encode(&[
        Token::Address(collateral_token),
        Token::Uint(U256::from_big_endian(collection_id.as_bytes())),
    ]);
    H256::from(keccak256(&encoded))
}

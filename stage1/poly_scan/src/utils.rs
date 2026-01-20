use ethers::types::{U256, Address};
use ethers::utils::format_units;

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

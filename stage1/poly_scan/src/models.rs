use serde::Serialize;


#[derive(Serialize, Debug)]
#[allow(dead_code)]
pub enum TradeSide {
    BUY,
    SELL,
    UNKNOWN,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TradeOutput {
    pub tx_hash: String,
    pub log_index: u64,
    pub exchange: String,
    pub maker: String,
    pub taker: String,
    pub maker_asset_id: String,
    pub taker_asset_id: String,
    pub maker_amount_filled: String,
    pub taker_amount_filled: String,
    pub maker_decimals: u32,
    pub taker_decimals: u32,
    pub price: String,
    pub token_id: String,
    pub side: TradeSide,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MarketInfo {
    pub condition_id: String,
    pub question_id: String,
    pub oracle: String,
    pub outcome_slot_count: u64,
    pub collateral_token: String,
    pub yes_token_id: String,
    pub no_token_id: String,
}

use ethers::types::{Address, H256, U256};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct OrderFilledLog {
    pub order_hash: H256,
    pub maker: Address,
    pub taker: Address,
    pub maker_asset_id: U256,
    pub taker_asset_id: U256,
    pub maker_amount_filled: U256,
    pub taker_amount_filled: U256,
    pub fee: U256,
    // Metadata
    pub tx_hash: H256,
    pub log_index: U256,
    pub block_number: u64,
}

#[derive(Serialize, Debug)]
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

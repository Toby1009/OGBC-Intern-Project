pub const POLYGON_RPC_URL: &str = "https://polygon-rpc.com";
pub const EXCHANGE_PROXY_ADDRESS: &str = "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E";

// event OrderFilled(bytes32 indexed orderHash, address indexed maker, address indexed taker, uint256 makerAssetId, uint256 takerAssetId, uint256 makerAmountFilled, uint256 takerAmountFilled, uint256 fee);
pub const ORDER_FILLED_EVENT_SIGNATURE: &str = "OrderFilled(bytes32,address,address,uint256,uint256,uint256,uint256,uint256)";

// Common Token Addresses on Polygon (Optional mapping for quick lookup if needed)
pub const USDC_ADDRESS: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"; // USDC.e
pub const NATIVE_TOKEN_DECIMALS: u32 = 18;

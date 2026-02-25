use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, OnceLock};

/// Global shared reqwest::Client — 所有 provider 共用同一個連接池
static SHARED_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Cached provider info list — 避免每次 info() 都重新分配
static PROVIDER_INFO_CACHE: LazyLock<Vec<ProviderInfo>> = LazyLock::new(build_all_provider_info);

/// Cached provider info map — O(1) 查找
pub static PROVIDER_INFO_MAP: LazyLock<HashMap<String, ProviderInfo>> = LazyLock::new(|| {
    PROVIDER_INFO_CACHE.iter().map(|p| (p.id.clone(), p.clone())).collect()
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetData {
    pub symbol: String,
    pub price: f64,
    pub currency: String,
    pub change_24h: Option<f64>,
    pub change_percent_24h: Option<f64>,
    pub high_24h: Option<f64>,
    pub low_24h: Option<f64>,
    pub volume: Option<f64>,
    pub market_cap: Option<f64>,
    pub last_updated: i64,
    pub provider_id: String,
    pub extra: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub requires_api_key: bool,
    pub requires_api_secret: bool,
    pub supports_websocket: bool,
    pub optional_api_key: bool,
    pub free_tier_info: String,
    pub symbol_format: String,
    pub supported_fields: Vec<String>,
    /// Default refresh interval (ms) when using free/no-key mode
    pub free_interval: i64,
    /// Default refresh interval (ms) when using API key mode
    pub key_interval: i64,
}

#[async_trait::async_trait]
pub trait DataProvider: Send + Sync {
    #[allow(dead_code)]
    fn info(&self) -> ProviderInfo;
    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String>;
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        // Default fallback: 逐一查詢（各 provider 應覆寫此方法以使用批量/並行）
        let mut results = Vec::new();
        for symbol in symbols {
            match self.fetch_price(symbol).await {
                Ok(data) => results.push(data),
                Err(e) => eprintln!("Error fetching {}: {}", symbol, e),
            }
        }
        Ok(results)
    }
}

/// Shared HTTP client — 全局單例，所有 provider 共用同一個連接池和 TCP 連接
pub fn shared_client() -> reqwest::Client {
    SHARED_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("StockenBoard/1.0")
            .pool_max_idle_per_host(10)
            .build()
            .unwrap_or_default()
    }).clone()
}

/// Helper to build AssetData with defaults
pub struct AssetDataBuilder {
    data: AssetData,
    extra: HashMap<String, serde_json::Value>,
}

impl AssetDataBuilder {
    pub fn new(symbol: &str, provider_id: &str) -> Self {
        Self {
            data: AssetData {
                symbol: symbol.to_string(),
                price: 0.0,
                currency: "USD".to_string(),
                change_24h: None,
                change_percent_24h: None,
                high_24h: None,
                low_24h: None,
                volume: None,
                market_cap: None,
                last_updated: chrono::Utc::now().timestamp_millis(),
                provider_id: provider_id.to_string(),
                extra: None,
            },
            extra: HashMap::new(),
        }
    }

    pub fn price(mut self, p: f64) -> Self { self.data.price = p; self }
    pub fn currency(mut self, c: &str) -> Self { self.data.currency = c.to_string(); self }
    pub fn change_24h(mut self, v: Option<f64>) -> Self { self.data.change_24h = v; self }
    pub fn change_percent_24h(mut self, v: Option<f64>) -> Self { self.data.change_percent_24h = v; self }
    pub fn high_24h(mut self, v: Option<f64>) -> Self { self.data.high_24h = v; self }
    pub fn low_24h(mut self, v: Option<f64>) -> Self { self.data.low_24h = v; self }
    pub fn volume(mut self, v: Option<f64>) -> Self { self.data.volume = v; self }
    pub fn market_cap(mut self, v: Option<f64>) -> Self { self.data.market_cap = v; self }

    pub fn extra_f64(mut self, key: &str, val: Option<f64>) -> Self {
        if let Some(v) = val { self.extra.insert(key.to_string(), serde_json::json!(v)); }
        self
    }
    pub fn extra_i64(mut self, key: &str, val: Option<i64>) -> Self {
        if let Some(v) = val { self.extra.insert(key.to_string(), serde_json::json!(v)); }
        self
    }
    pub fn extra_str(mut self, key: &str, val: Option<&str>) -> Self {
        if let Some(v) = val { self.extra.insert(key.to_string(), serde_json::json!(v)); }
        self
    }

    pub fn build(mut self) -> AssetData {
        self.data.extra = if self.extra.is_empty() { None } else { Some(self.extra) };
        self.data
    }
}

/// Pool lookup result for DEX providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexPoolInfo {
    pub token0_address: String,
    pub token0_symbol: String,
    pub token1_address: String,
    pub token1_symbol: String,
}

/// Trait for DEX providers that can look up pool token info
#[async_trait::async_trait]
pub trait DexPoolLookup: Send + Sync {
    async fn lookup_pool(&self, pool_address: &str) -> Result<DexPoolInfo, String>;
}

/// WebSocket message types for real-time data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsTickerUpdate {
    pub symbol: String,
    pub provider_id: String,
    pub data: AssetData,
}

/// Trait for providers that support WebSocket streaming
#[async_trait::async_trait]
pub trait WebSocketProvider: Send + Sync {
    /// Subscribe to real-time updates for given symbols.
    /// Returns a JoinHandle for the WS connection task so it can be aborted on cleanup.
    async fn subscribe(
        &self,
        symbols: Vec<String>,
        sender: Arc<tokio::sync::broadcast::Sender<WsTickerUpdate>>,
    ) -> Result<tokio::task::JoinHandle<()>, String>;
}

// All Provider static info
// free_interval = 免費版默認刷新間隔(ms), key_interval = 有API Key時默認刷新間隔(ms)
pub fn get_all_provider_info() -> Vec<ProviderInfo> {
    PROVIDER_INFO_CACHE.clone()
}

/// O(1) 查找單個 provider info
#[allow(dead_code)]
pub fn get_provider_info(id: &str) -> Option<ProviderInfo> {
    PROVIDER_INFO_MAP.get(id).cloned()
}

fn build_all_provider_info() -> Vec<ProviderInfo> {
    vec![
        // Crypto                                                                                    free_iv  key_iv
        pi("binance", "Binance", "crypto", false, false, true,
           "Free unlimited (1200 weight/min)", "BTCUSDT, ETHUSDT",
           &["price","change_24h","high_24h","low_24h","volume"],                                    5000,    5000),
        pi("coinbase", "Coinbase", "crypto", false, false, true,
           "Free unlimited", "BTC-USD, ETH-USD",
           &["price","volume"],                                                                      5000,    5000),
        pi("coingecko", "CoinGecko", "crypto", false, false, false,
           "Free 10-30 calls/min; w/ key 30/min", "bitcoin, ethereum",
           &["price","change_24h","volume","market_cap"],                                            60000,   20000),
        pi("coinmarketcap", "CoinMarketCap", "crypto", true, false, false,
           "Free 10k credits/mo, 10 calls/min", "BTC, ETH",
           &["price","change_24h","volume","market_cap"],                                            60000,   30000),
        pi("cryptocompare", "CryptoCompare", "crypto", false, false, true,
           "Free tier; w/ key 100k calls/mo", "BTC, ETH",
           &["price","change_24h","high_24h","low_24h","volume","market_cap"],                       30000,   10000),
        // Stock
        pi("yahoo", "Yahoo Finance", "stock", false, false, false,
           "Unofficial API (cookie+crumb)", "AAPL, GOOGL, TSLA",
           &["price","change_24h","high_24h","low_24h","volume"],                                    15000,   15000),
        pi("marketstack", "Marketstack", "stock", true, false, false,
           "Free 100 req/mo; paid unlimited", "AAPL, MSFT",
           &["price","high_24h","low_24h","volume"],                                                 600000,  60000),
        pi("eodhd", "EODHD", "stock", true, false, false,
           "Free 20 calls/day; paid unlimited", "AAPL.US, TSLA.US",
           &["price","change_24h","high_24h","low_24h","volume"],                                    300000,  30000),
        pi("mboum", "Mboum", "stock", true, false, false,
           "Limited free (Bearer token)", "AAPL, MSFT",
           &["price","change_24h","volume"],                                                         60000,   15000),
        // Both
        pi("alpaca", "Alpaca", "both", true, true, true,
           "200 calls/min, real-time data", "AAPL, BTC/USD",
           &["price","change_24h","high_24h","low_24h","volume"],                                    60000,   5000),
        pi("finnhub", "Finnhub", "both", true, false, true,
           "Free 60 calls/min, no daily limit", "AAPL, BINANCE:BTCUSDT",
           &["price","change_24h","high_24h","low_24h"],                                             60000,   10000),
        pi("alphavantage", "Alpha Vantage", "both", true, false, false,
           "Free 25 calls/day; paid more", "AAPL, BTC",
           &["price","change_24h","high_24h","low_24h","volume"],                                    180000,  60000),
        pi("polygon", "Polygon.io", "both", true, false, true,
           "Free 5 calls/min; paid unlimited", "AAPL, X:BTCUSD",
           &["price","change_24h","high_24h","low_24h","volume"],                                    60000,   15000),
        pi("tiingo", "Tiingo", "both", true, false, false,
           "Free 500 req/mo; paid more", "AAPL, btcusd",
           &["price","change_24h","high_24h","low_24h","volume"],                                    120000,  30000),
        pi("fmp", "Financial Modeling Prep", "both", true, false, false,
           "Free 250 calls/day; paid more", "AAPL, BTCUSD",
           &["price","change_24h","high_24h","low_24h","volume","market_cap"],                       360000,  30000),
        pi("twelvedata", "Twelve Data", "both", true, false, true,
           "Free 800 calls/day, 8/min; paid more", "AAPL, BTC/USD",
           &["price","change_24h","high_24h","low_24h","volume"],                                    15000,   8000),
        // Prediction
        pi("polymarket", "Polymarket", "prediction", false, false, true,
           "Free unlimited reads", "condition_id",
           &["price","volume"],                                                                      5000,    5000),
        pi("bitquery", "Bitquery", "prediction", true, false, false,
           "Free tier (OAuth token)", "contract_address",
           &["price","volume"],                                                                      30000,   15000),
        // New Crypto Exchanges
        pi("kraken", "Kraken", "crypto", false, false, false,
           "Free unlimited (public API)", "XBTUSD, ETHUSD",
           &["price","change_24h","high_24h","low_24h","volume"],                                    5000,    5000),
        pi("bybit", "Bybit", "crypto", false, false, false,
           "Free 120 req/s (public API)", "BTCUSDT, ETHUSDT",
           &["price","change_24h","high_24h","low_24h","volume"],                                    5000,    5000),
        pi("kucoin", "KuCoin", "crypto", false, false, false,
           "Free unlimited (public API)", "BTC-USDT, ETH-USDT",
           &["price","change_24h","high_24h","low_24h","volume"],                                    5000,    5000),
        pi("okx", "OKX", "crypto", false, false, false,
           "Free 20 req/2s (public API)", "BTC-USDT, ETH-USDT",
           &["price","change_24h","high_24h","low_24h","volume"],                                    5000,    5000),
        pi("gateio", "Gate.io", "crypto", false, false, false,
           "Free 900 req/s (public API)", "BTC_USDT, ETH_USDT",
           &["price","change_24h","high_24h","low_24h","volume"],                                    5000,    5000),
        pi("bitfinex", "Bitfinex", "crypto", false, false, false,
           "Free 90 req/min (public API)", "tBTCUSD, tETHUSD",
           &["price","change_24h","high_24h","low_24h","volume"],                                    10000,   10000),
        pi("htx", "HTX (Huobi)", "crypto", false, false, false,
           "Free 100 req/s (public API)", "btcusdt, ethusdt",
           &["price","change_24h","high_24h","low_24h","volume"],                                    5000,    5000),
        pi("mexc", "MEXC", "crypto", false, false, false,
           "Free 20 req/s (public API)", "BTCUSDT, ETHUSDT",
           &["price","change_24h","high_24h","low_24h","volume"],                                    5000,    5000),
        // Aggregators
        pi("coinpaprika", "CoinPaprika", "crypto", false, false, false,
           "Free unlimited (public API)", "btc-bitcoin, eth-ethereum",
           &["price","change_24h","volume","market_cap"],                                            30000,   30000),
        pi("coinapi", "CoinAPI", "both", true, false, false,
           "Free $25 credits; 100 data points/credit", "BTC, ETH, AAPL",
           &["price"],                                                                               60000,   30000),
        // Stock/Global
        pi("fcsapi", "FCS API", "both", true, false, false,
           "Free 500 req/mo; paid 10k+/mo, 30+ markets", "AAPL, MSFT, 2330.TW",
           &["price","change_24h","high_24h","low_24h","volume"],                                    120000,  30000),
        // DEX Aggregators
        pi("jupiter", "Jupiter", "dex", true, false, false,
           "API Key required (portal.jup.ag free); Solana DEX aggregator", "SOL, JUP, BONK, WIF, mint_address",
           &["price","change_24h"],                                                                  10000,   5000),
        pi("okx_dex", "OKX DEX", "dex", true, false, false,
           "API Key required (OKX Web3 Portal free); multi-chain DEX aggregator", "ETH, SOL, BNB, eth:0x..., sol:mint",
           &["price"],                                                                               15000,   10000),
        // DEX Pool Providers (for DEX aggregator page)
        pi("raydium", "Raydium", "dex", true, false, false,
           "API Key required; Solana DEX AMM", "pool:tokenFrom:tokenTo",
           &["price"],                                                                               10000,   5000),
        pi("subgraph", "Subgraph (Uniswap/Sushi/Pancake)", "dex", true, false, false,
           "API Key required (The Graph); EVM DEX aggregator", "protocol:pool:tokenFrom:tokenTo",
           &["price"],                                                                               15000,   10000),
    ]
}

/// Normalize a crypto symbol from any common format to a base+quote pair.
/// Returns (base, quote) e.g. ("BTC", "USD")
pub fn parse_crypto_symbol(symbol: &str) -> (String, String) {
    let s = symbol.to_uppercase();
    // "BTC-USD" -> ("BTC", "USD")
    if let Some((base, quote)) = s.split_once('-') {
        return (base.to_string(), quote.to_string());
    }
    // "BTC/USD" -> ("BTC", "USD")
    if let Some((base, quote)) = s.split_once('/') {
        return (base.to_string(), quote.to_string());
    }
    // "BTCUSDT" -> ("BTC", "USDT")
    for suffix in &["USDT", "USDC", "BUSD", "USD", "EUR", "GBP", "BTC", "ETH", "BNB"] {
        if s.len() > suffix.len() && s.ends_with(suffix) {
            let base = &s[..s.len() - suffix.len()];
            return (base.to_string(), suffix.to_string());
        }
    }
    // Fallback: assume it's just a base symbol
    (s, "USD".to_string())
}

/// Convert symbol to Binance format: BTCUSDT
pub fn to_binance_symbol(symbol: &str) -> String {
    let (base, quote) = parse_crypto_symbol(symbol);
    let q = if quote == "USD" { "USDT" } else { &quote };
    format!("{}{}", base, q)
}

/// Convert symbol to Coinbase format: BTC-USD
pub fn to_coinbase_symbol(symbol: &str) -> String {
    let (base, quote) = parse_crypto_symbol(symbol);
    let q = if quote == "USDT" { "USD" } else { &quote };
    format!("{}-{}", base, q)
}

/// Convert symbol to CoinGecko format: bitcoin, ethereum
/// This is a best-effort mapping for common coins
pub fn to_coingecko_id(symbol: &str) -> String {
    let (base, _) = parse_crypto_symbol(symbol);
    match base.as_str() {
        "BTC" => "bitcoin".to_string(),
        "ETH" => "ethereum".to_string(),
        "BNB" => "binancecoin".to_string(),
        "SOL" => "solana".to_string(),
        "XRP" => "ripple".to_string(),
        "ADA" => "cardano".to_string(),
        "DOGE" => "dogecoin".to_string(),
        "DOT" => "polkadot".to_string(),
        "AVAX" => "avalanche-2".to_string(),
        "MATIC" | "POL" => "matic-network".to_string(),
        "LINK" => "chainlink".to_string(),
        "UNI" => "uniswap".to_string(),
        "ATOM" => "cosmos".to_string(),
        "LTC" => "litecoin".to_string(),
        "SHIB" => "shiba-inu".to_string(),
        "TRX" => "tron".to_string(),
        "NEAR" => "near".to_string(),
        "APT" => "aptos".to_string(),
        "ARB" => "arbitrum".to_string(),
        "OP" => "optimism".to_string(),
        "SUI" => "sui".to_string(),
        "PEPE" => "pepe".to_string(),
        "FIL" => "filecoin".to_string(),
        "AAVE" => "aave".to_string(),
        "MKR" => "maker".to_string(),
        _ => symbol.to_lowercase(), // fallback: user might already pass coingecko id
    }
}

/// Convert symbol to CoinMarketCap / CryptoCompare format: BTC, ETH
pub fn to_base_symbol(symbol: &str) -> String {
    let (base, _) = parse_crypto_symbol(symbol);
    base
}

fn pi(id: &str, name: &str, ptype: &str, key: bool, secret: bool, ws: bool,
      free: &str, fmt: &str, fields: &[&str], free_iv: i64, key_iv: i64) -> ProviderInfo {
    // Providers that work without key but benefit from having one
    let opt_key = matches!(id, "coingecko" | "cryptocompare");
    ProviderInfo {
        id: id.to_string(), name: name.to_string(), provider_type: ptype.to_string(),
        requires_api_key: key, requires_api_secret: secret, supports_websocket: ws,
        optional_api_key: opt_key,
        free_tier_info: free.to_string(), symbol_format: fmt.to_string(),
        supported_fields: fields.iter().map(|s| s.to_string()).collect(),
        free_interval: free_iv,
        key_interval: key_iv,
    }
}

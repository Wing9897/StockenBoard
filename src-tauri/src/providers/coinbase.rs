use super::traits::*;

pub struct CoinbaseProvider {
    client: reqwest::Client,
}

impl CoinbaseProvider {
    pub fn new() -> Self {
        Self { client: shared_client() }
    }
}

#[async_trait::async_trait]
impl DataProvider for CoinbaseProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("coinbase").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        // Auto-convert: BTCUSDT -> BTC-USD, BTC/USD -> BTC-USD
        let pair = to_coinbase_symbol(symbol);
        let url = format!("https://api.coinbase.com/v2/prices/{}/spot", pair);
        let data: serde_json::Value = self.client.get(&url)
            .send().await.map_err(|e| format!("Coinbase 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Coinbase API 錯誤: {}。格式: BTC-USD", e))?
            .json().await.map_err(|e| format!("Coinbase 解析失敗: {}", e))?;

        let price = data["data"]["amount"].as_str()
            .and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        let currency = data["data"]["currency"].as_str().unwrap_or("USD");

        Ok(AssetDataBuilder::new(symbol, "coinbase")
            .price(price)
            .currency(currency)
            .build())
    }
}

use super::traits::*;

pub struct PolymarketProvider {
    client: reqwest::Client,
}

impl PolymarketProvider {
    pub fn new() -> Self {
        Self { client: shared_client() }
    }
}

#[async_trait::async_trait]
impl DataProvider for PolymarketProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("polymarket").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        // symbol = condition_id for the market
        let data: serde_json::Value = self.client
            .get(format!("https://clob.polymarket.com/markets/{}", symbol))
            .send().await.map_err(|e| format!("Polymarket 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Polymarket API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Polymarket 解析失敗: {}", e))?;

        let price = data["outcome_prices"].as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let volume = data["volume"].as_str().and_then(|s| s.parse::<f64>().ok());

        let outcomes = data["outcomes"].as_array()
            .map(|arr| arr.iter().filter_map(|o| o.as_str()).collect::<Vec<_>>().join(", "));

        Ok(AssetDataBuilder::new(symbol, "polymarket")
            .price(price)
            .currency("PROB")
            .volume(volume)
            .extra_str("問題", data["question"].as_str())
            .extra_str("結束日期", data["end_date_iso"].as_str())
            .extra_str("選項", outcomes.as_deref())
            .build())
    }
}

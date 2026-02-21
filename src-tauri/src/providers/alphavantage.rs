use super::traits::*;

pub struct AlphaVantageProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl AlphaVantageProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { client: shared_client(), api_key }
    }
}

#[async_trait::async_trait]
impl DataProvider for AlphaVantageProvider {
    fn info(&self) -> ProviderInfo {
        get_all_provider_info().into_iter().find(|p| p.id == "alphavantage").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("Alpha Vantage 需要 API Key")?;

        let data: serde_json::Value = self.client
            .get(format!("https://www.alphavantage.co/query?function=GLOBAL_QUOTE&symbol={}&apikey={}", symbol, api_key))
            .send().await.map_err(|e| format!("AlphaVantage 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("AlphaVantage API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("AlphaVantage 解析失敗: {}", e))?;

        // Check for rate limit message
        if data["Note"].is_string() || data["Information"].is_string() {
            return Err("Alpha Vantage 已達到速率限制 (25 calls/day)".to_string());
        }

        let q = &data["Global Quote"];
        if q.is_null() || q["05. price"].is_null() {
            return Err(format!("AlphaVantage 找不到: {}", symbol));
        }

        let parse = |key: &str| q[key].as_str().and_then(|s| s.parse::<f64>().ok());
        let pct = q["10. change percent"].as_str()
            .and_then(|s| s.trim_end_matches('%').parse::<f64>().ok());

        Ok(AssetDataBuilder::new(symbol, "alphavantage")
            .price(parse("05. price").unwrap_or(0.0))
            .change_24h(parse("09. change"))
            .change_percent_24h(pct)
            .high_24h(parse("03. high"))
            .low_24h(parse("04. low"))
            .volume(parse("06. volume"))
            .extra_f64("開盤價", parse("02. open"))
            .extra_f64("前收盤價", parse("08. previous close"))
            .build())
    }
}

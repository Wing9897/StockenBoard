use super::traits::*;

pub struct AlphaVantageProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl AlphaVantageProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: shared_client(),
            api_key,
        }
    }
}

#[async_trait::async_trait]
impl DataProvider for AlphaVantageProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("alphavantage").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("Alpha Vantage 需要 API Key")?;

        let data: serde_json::Value = self
            .client
            .get(format!(
                "https://www.alphavantage.co/query?function=GLOBAL_QUOTE&symbol={}&apikey={}",
                symbol, api_key
            ))
            .send()
            .await
            .map_err(|e| format!("AlphaVantage 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("AlphaVantage API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("AlphaVantage 解析失敗: {}", e))?;

        // Check for rate limit message
        if data["Note"].is_string() || data["Information"].is_string() {
            return Err("Alpha Vantage 已達到速率限制 (25 calls/day)".to_string());
        }

        let q = &data["Global Quote"];
        if q.is_null() || q["05. price"].is_null() {
            return Err(format!("AlphaVantage 找不到: {}", symbol));
        }

        let parse = |key: &str| q[key].as_str().and_then(|s| s.parse::<f64>().ok());
        let pct = q["10. change percent"]
            .as_str()
            .and_then(|s| s.trim_end_matches('%').parse::<f64>().ok());

        Ok(AssetDataBuilder::new(symbol, "alphavantage")
            .price(parse("05. price").unwrap_or(0.0))
            .change_24h(parse("09. change"))
            .change_percent_24h(pct)
            .high_24h(parse("03. high"))
            .low_24h(parse("04. low"))
            .volume(parse("06. volume"))
            .extra_f64("open_price", parse("02. open"))
            .extra_f64("prev_close", parse("08. previous close"))
            .build())
    }

    /// 限流並行查詢 — Alpha Vantage 沒有批量 endpoint（注意免費版 25 calls/day）
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        let api_key = self
            .api_key
            .as_ref()
            .ok_or("Alpha Vantage 需要 API Key")?
            .clone();
        let client = self.client.clone();

        use futures::stream::{self, StreamExt};
        let results: Vec<_> = stream::iter(symbols.to_vec())
            .map(|sym| {
                let c = client.clone();
                let key = api_key.clone();
                async move {
                    let data: serde_json::Value = c
                        .get(format!("https://www.alphavantage.co/query?function=GLOBAL_QUOTE&symbol={}&apikey={}", sym, key))
                        .send().await.map_err(|e| format!("AlphaVantage: {}", e))?
                        .json().await.map_err(|e| format!("AlphaVantage: {}", e))?;
                    if data["Note"].is_string() || data["Information"].is_string() {
                        return Err("Alpha Vantage 已達到速率限制".to_string());
                    }
                    let q = &data["Global Quote"];
                    if q.is_null() || q["05. price"].is_null() {
                        return Err(format!("AlphaVantage 找不到: {}", sym));
                    }
                    let parse = |key: &str| q[key].as_str().and_then(|s| s.parse::<f64>().ok());
                    let pct = q["10. change percent"].as_str()
                        .and_then(|s| s.trim_end_matches('%').parse::<f64>().ok());
                    Ok(AssetDataBuilder::new(&sym, "alphavantage")
                        .price(parse("05. price").unwrap_or(0.0))
                        .change_24h(parse("09. change"))
                        .change_percent_24h(pct)
                        .high_24h(parse("03. high"))
                        .low_24h(parse("04. low"))
                        .volume(parse("06. volume"))
                        .extra_f64("open_price", parse("02. open"))
                        .extra_f64("prev_close", parse("08. previous close"))
                        .build())
                }
            })
            .buffer_unordered(2)
            .collect()
            .await;

        let mut out = Vec::new();
        for r in results {
            match r {
                Ok(data) => out.push(data),
                Err(e) => eprintln!("AlphaVantage 跳過: {}", e),
            }
        }
        Ok(out)
    }
}

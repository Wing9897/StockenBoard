use super::traits::*;
use std::collections::HashMap;

pub struct EODHDProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl EODHDProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { client: shared_client(), api_key }
    }

    fn parse_eod(symbol: &str, data: &serde_json::Value) -> AssetData {
        AssetDataBuilder::new(symbol, "eodhd")
            .price(data["close"].as_f64().unwrap_or(0.0))
            .change_24h(data["change"].as_f64())
            .change_percent_24h(data["change_p"].as_f64())
            .high_24h(data["high"].as_f64())
            .low_24h(data["low"].as_f64())
            .volume(data["volume"].as_f64())
            .extra_f64("open_price", data["open"].as_f64())
            .extra_f64("prev_close", data["previousClose"].as_f64())
            .build()
    }
}

#[async_trait::async_trait]
impl DataProvider for EODHDProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("eodhd").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("EODHD 需要 API Key")?;

        let data: serde_json::Value = self.client
            .get(format!("https://eodhd.com/api/real-time/{}?api_token={}&fmt=json", symbol, api_key))
            .send().await.map_err(|e| format!("EODHD 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("EODHD API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("EODHD 解析失敗: {}", e))?;

        Ok(Self::parse_eod(symbol, &data))
    }

    /// 批量查詢 — s=AAPL.US,MSFT.US
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        let api_key = self.api_key.as_ref().ok_or("EODHD 需要 API Key")?;
        let extra = symbols[1..].join(",");

        // EODHD batch: first symbol in path, rest in s= param
        let url = format!(
            "https://eodhd.com/api/real-time/{}?api_token={}&fmt=json&s={}",
            symbols[0], api_key, extra
        );

        let arr: Vec<serde_json::Value> = self.client
            .get(&url)
            .send().await.map_err(|e| format!("EODHD 批量連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("EODHD API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("EODHD 批量解析失敗: {}", e))?;

        let response_map: HashMap<String, &serde_json::Value> = arr.iter()
            .filter_map(|v| v["code"].as_str().map(|s| (s.to_uppercase(), v)))
            .collect();

        let mut results = Vec::new();
        for sym in symbols {
            // EODHD code 可能是 AAPL.US 或 AAPL
            let key = sym.to_uppercase();
            if let Some(data) = response_map.get(&key) {
                results.push(Self::parse_eod(sym, data));
            } else {
                // 嘗試不帶交易所後綴
                let base = key.split('.').next().unwrap_or(&key);
                if let Some(data) = response_map.get(base) {
                    results.push(Self::parse_eod(sym, data));
                }
            }
        }
        Ok(results)
    }
}

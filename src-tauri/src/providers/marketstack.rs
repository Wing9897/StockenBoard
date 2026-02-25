use super::traits::*;
use std::collections::HashMap;

pub struct MarketstackProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl MarketstackProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { client: shared_client(), api_key }
    }

    fn parse_eod(symbol: &str, eod: &serde_json::Value) -> AssetData {
        let price = eod["close"].as_f64().unwrap_or(0.0);
        let open = eod["open"].as_f64().unwrap_or(price);
        let change = price - open;
        let pct = if open > 0.0 { (change / open) * 100.0 } else { 0.0 };

        AssetDataBuilder::new(symbol, "marketstack")
            .price(price)
            .change_24h(Some(change))
            .change_percent_24h(Some(pct))
            .high_24h(eod["high"].as_f64())
            .low_24h(eod["low"].as_f64())
            .volume(eod["volume"].as_f64())
            .extra_f64("open_price", eod["open"].as_f64())
            .extra_str("exchange", eod["exchange"].as_str())
            .build()
    }
}

#[async_trait::async_trait]
impl DataProvider for MarketstackProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("marketstack").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("Marketstack 需要 API Key")?;

        let data: serde_json::Value = self.client
            .get(format!("http://api.marketstack.com/v1/eod/latest?access_key={}&symbols={}", api_key, symbol))
            .send().await.map_err(|e| format!("Marketstack 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Marketstack API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Marketstack 解析失敗: {}", e))?;

        if let Some(err) = data["error"].as_object() {
            let msg = err.get("message").and_then(|v| v.as_str()).unwrap_or("未知錯誤");
            return Err(format!("Marketstack: {}", msg));
        }

        let eod = &data["data"][0];
        if eod.is_null() {
            return Err(format!("Marketstack 找不到: {}", symbol));
        }
        Ok(Self::parse_eod(symbol, eod))
    }

    /// 批量查詢 — symbols=AAPL,MSFT
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        let api_key = self.api_key.as_ref().ok_or("Marketstack 需要 API Key")?;
        let syms = symbols.join(",");

        let data: serde_json::Value = self.client
            .get(format!("http://api.marketstack.com/v1/eod/latest?access_key={}&symbols={}", api_key, syms))
            .send().await.map_err(|e| format!("Marketstack 批量連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Marketstack API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Marketstack 批量解析失敗: {}", e))?;

        if let Some(err) = data["error"].as_object() {
            let msg = err.get("message").and_then(|v| v.as_str()).unwrap_or("未知錯誤");
            return Err(format!("Marketstack: {}", msg));
        }

        let arr = data["data"].as_array().ok_or("Marketstack 批量回應格式錯誤")?;
        // 建立 symbol -> eod 查找表（取每個 symbol 最新的一筆）
        let mut latest: HashMap<String, &serde_json::Value> = HashMap::new();
        for eod in arr {
            if let Some(sym) = eod["symbol"].as_str() {
                latest.entry(sym.to_uppercase()).or_insert(eod);
            }
        }

        let mut results = Vec::new();
        for sym in symbols {
            if let Some(eod) = latest.get(&sym.to_uppercase()) {
                results.push(Self::parse_eod(sym, eod));
            }
        }
        Ok(results)
    }
}

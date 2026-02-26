use super::traits::*;
use std::collections::HashMap;

pub struct MboumProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl MboumProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { client: shared_client(), api_key }
    }

    fn parse_quote(symbol: &str, q: &serde_json::Value) -> AssetData {
        let price = q["regularMarketPrice"].as_f64().unwrap_or(0.0);
        let market_state = q["marketState"].as_str();
        let pre_price = q["preMarketPrice"].as_f64();
        let post_price = q["postMarketPrice"].as_f64();

        let mut builder = AssetDataBuilder::new(symbol, "mboum")
            .price(price)
            .currency(q["currency"].as_str().unwrap_or("USD"))
            .change_24h(q["regularMarketChange"].as_f64())
            .change_percent_24h(q["regularMarketChangePercent"].as_f64())
            .high_24h(q["regularMarketDayHigh"].as_f64())
            .low_24h(q["regularMarketDayLow"].as_f64())
            .volume(q["regularMarketVolume"].as_f64())
            .market_cap(q["marketCap"].as_f64())
            .extra_f64("open_price", q["regularMarketOpen"].as_f64())
            .extra_f64("prev_close", q["regularMarketPreviousClose"].as_f64())
            .extra_f64("52w_high", q["fiftyTwoWeekHigh"].as_f64())
            .extra_f64("52w_low", q["fiftyTwoWeekLow"].as_f64())
            .extra_str("name", q["shortName"].as_str())
            .extra_str("market_session", market_state);

        // 盤前數據
        if let Some(pp) = pre_price {
            builder = builder.extra_f64("pre_market_price", Some(pp));
            let pre_change = pp - price;
            let pre_pct = if price > 0.0 { (pre_change / price) * 100.0 } else { 0.0 };
            builder = builder.extra_f64("pre_market_change", Some(pre_change));
            builder = builder.extra_f64("pre_market_change_pct", Some(pre_pct));
        }

        // 盤後數據
        if let Some(pp) = post_price {
            builder = builder.extra_f64("post_market_price", Some(pp));
            let post_change = pp - price;
            let post_pct = if price > 0.0 { (post_change / price) * 100.0 } else { 0.0 };
            builder = builder.extra_f64("post_market_change", Some(post_change));
            builder = builder.extra_f64("post_market_change_pct", Some(post_pct));
        }

        builder.build()
    }
}

#[async_trait::async_trait]
impl DataProvider for MboumProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("mboum").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("Mboum 需要 API Key")?;

        let data: serde_json::Value = self.client
            .get(format!("https://api.mboum.com/v1/markets/stock/quotes?ticker={}", symbol))
            .header("Authorization", format!("Bearer {}", api_key))
            .send().await.map_err(|e| format!("Mboum 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Mboum API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Mboum 解析失敗: {}", e))?;

        let q = &data["body"][0];
        if q.is_null() {
            return Err(format!("Mboum 找不到: {}", symbol));
        }
        Ok(Self::parse_quote(symbol, q))
    }

    /// 批量查詢 — ticker=AAPL,MSFT
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        let api_key = self.api_key.as_ref().ok_or("Mboum 需要 API Key")?;
        let syms = symbols.join(",");

        let data: serde_json::Value = self.client
            .get(format!("https://api.mboum.com/v1/markets/stock/quotes?ticker={}", syms))
            .header("Authorization", format!("Bearer {}", api_key))
            .send().await.map_err(|e| format!("Mboum 批量連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Mboum API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Mboum 批量解析失敗: {}", e))?;

        let arr = data["body"].as_array().ok_or("Mboum 批量回應格式錯誤")?;
        let response_map: HashMap<String, &serde_json::Value> = arr.iter()
            .filter_map(|v| v["symbol"].as_str().map(|s| (s.to_uppercase(), v)))
            .collect();

        let mut results = Vec::new();
        for sym in symbols {
            if let Some(q) = response_map.get(&sym.to_uppercase()) {
                results.push(Self::parse_quote(sym, q));
            }
        }
        Ok(results)
    }
}

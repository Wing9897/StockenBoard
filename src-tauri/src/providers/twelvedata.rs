use super::traits::*;

pub struct TwelveDataProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl TwelveDataProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { client: shared_client(), api_key }
    }

    fn to_td_symbol(symbol: &str) -> String {
        let s = symbol.to_uppercase();
        let looks_crypto = s.ends_with("USDT") || s.ends_with("USD") || s.contains('-');
        if looks_crypto && !s.contains('/') {
            let (base, quote) = parse_crypto_symbol(symbol);
            let q = if quote == "USDT" { "USD" } else { &quote };
            format!("{}/{}", base, q)
        } else {
            symbol.to_string()
        }
    }

    fn parse_quote(symbol: &str, data: &serde_json::Value) -> Result<AssetData, String> {
        if data["code"].is_number() {
            let msg = data["message"].as_str().unwrap_or("未知錯誤");
            return Err(format!("TwelveData: {}", msg));
        }
        let parse = |key: &str| data[key].as_str().and_then(|s| s.parse::<f64>().ok());
        let is_extended = data["is_extended_hours"].as_bool().unwrap_or(false);

        let mut builder = AssetDataBuilder::new(symbol, "twelvedata")
            .price(parse("close").unwrap_or(0.0))
            .currency(data["currency"].as_str().unwrap_or("USD"))
            .change_24h(parse("change"))
            .change_percent_24h(parse("percent_change"))
            .high_24h(parse("high"))
            .low_24h(parse("low"))
            .volume(parse("volume"))
            .extra_f64("open_price", parse("open"))
            .extra_f64("prev_close", parse("previous_close"))
            .extra_f64("52w_high", data["fifty_two_week"]["high"].as_str().and_then(|s| s.parse().ok()))
            .extra_f64("52w_low", data["fifty_two_week"]["low"].as_str().and_then(|s| s.parse().ok()));

        // 市場狀態 — 根據 is_extended_hours 判斷
        if data.get("is_extended_hours").is_some() {
            if is_extended {
                // Extended hours — 無法區分盤前盤後，統一標記
                builder = builder.extra_str("market_session", Some("POST"));
            } else {
                builder = builder.extra_str("market_session", Some("REGULAR"));
            }
        }

        Ok(builder.build())
    }
}

#[async_trait::async_trait]
impl DataProvider for TwelveDataProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("twelvedata").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("Twelve Data 需要 API Key")?;
        let api_symbol = Self::to_td_symbol(symbol);

        let data: serde_json::Value = self.client
            .get(format!("https://api.twelvedata.com/quote?symbol={}&prepost=true&apikey={}", api_symbol, api_key))
            .send().await.map_err(|e| format!("TwelveData 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("TwelveData API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("TwelveData 解析失敗: {}", e))?;

        Self::parse_quote(symbol, &data)
    }

    /// 批量查詢 — symbol=AAPL,BTC/USD
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        let api_key = self.api_key.as_ref().ok_or("Twelve Data 需要 API Key")?;
        let mappings: Vec<(String, String)> = symbols.iter()
            .map(|s| (s.clone(), Self::to_td_symbol(s)))
            .collect();
        let td_syms: Vec<&str> = mappings.iter().map(|(_, t)| t.as_str()).collect();
        let syms_str = td_syms.join(",");

        let data: serde_json::Value = self.client
            .get(format!("https://api.twelvedata.com/quote?symbol={}&prepost=true&apikey={}", syms_str, api_key))
            .send().await.map_err(|e| format!("TwelveData 批量連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("TwelveData API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("TwelveData 批量解析失敗: {}", e))?;

        let mut results = Vec::new();
        // TwelveData: 單個返回 object，多個返回 { "AAPL": {...}, "BTC/USD": {...} }
        if mappings.len() > 1 {
            for (original, td_sym) in &mappings {
                let item = &data[td_sym];
                if !item.is_null() {
                    match Self::parse_quote(original, item) {
                        Ok(asset) => results.push(asset),
                        Err(e) => eprintln!("TwelveData 批量跳過 {}: {}", original, e),
                    }
                }
            }
        } else {
            match Self::parse_quote(&mappings[0].0, &data) {
                Ok(asset) => results.push(asset),
                Err(e) => eprintln!("TwelveData 跳過 {}: {}", mappings[0].0, e),
            }
        }
        Ok(results)
    }
}

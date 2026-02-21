use super::traits::*;

pub struct PolygonProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl PolygonProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { client: shared_client(), api_key }
    }
}

#[async_trait::async_trait]
impl DataProvider for PolygonProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("polygon").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("Polygon.io 需要 API Key")?;

        // Auto-convert crypto symbols: BTCUSDT -> X:BTCUSD, BTC-USD -> X:BTCUSD
        let api_symbol = if symbol.starts_with("X:") || symbol.starts_with("O:") || symbol.starts_with("C:") {
            symbol.to_string()
        } else {
            let s = symbol.to_uppercase();
            let looks_crypto = s.ends_with("USDT") || s.ends_with("USD")
                || s.contains('-') || s.contains('/');
            if looks_crypto {
                let (base, quote) = parse_crypto_symbol(symbol);
                let q = if quote == "USDT" { "USD" } else { &quote };
                format!("X:{}{}", base, q)
            } else {
                symbol.to_string()
            }
        };

        let data: serde_json::Value = self.client
            .get(format!("https://api.polygon.io/v2/aggs/ticker/{}/prev?apiKey={}", api_symbol, api_key))
            .send().await.map_err(|e| format!("Polygon 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Polygon API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Polygon 解析失敗: {}", e))?;

        let r = &data["results"][0];
        if r.is_null() {
            return Err(format!("Polygon 找不到: {}。股票用 AAPL，加密用 X:BTCUSD", symbol));
        }

        let price = r["c"].as_f64().unwrap_or(0.0);
        let open = r["o"].as_f64().unwrap_or(price);
        let change = price - open;
        let pct = if open > 0.0 { (change / open) * 100.0 } else { 0.0 };

        Ok(AssetDataBuilder::new(symbol, "polygon")
            .price(price)
            .change_24h(Some(change))
            .change_percent_24h(Some(pct))
            .high_24h(r["h"].as_f64())
            .low_24h(r["l"].as_f64())
            .volume(r["v"].as_f64())
            .extra_f64("開盤價", r["o"].as_f64())
            .extra_f64("加權平均價", r["vw"].as_f64())
            .extra_i64("交易次數", r["n"].as_i64())
            .build())
    }
}

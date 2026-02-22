use super::traits::*;

pub struct FinnhubProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl FinnhubProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { client: shared_client(), api_key }
    }
}

#[async_trait::async_trait]
impl DataProvider for FinnhubProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("finnhub").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("Finnhub 需要 API Key")?;

        // Auto-convert crypto symbols: BTCUSDT -> BINANCE:BTCUSDT, BTC-USD -> BINANCE:BTCUSDT
        let api_symbol = if symbol.contains(':') {
            // Already in exchange:symbol format
            symbol.to_string()
        } else {
            let s = symbol.to_uppercase();
            let looks_crypto = s.ends_with("USDT") || s.ends_with("USD")
                || s.contains('-') || s.contains('/');
            if looks_crypto {
                let binance_sym = to_binance_symbol(symbol);
                format!("BINANCE:{}", binance_sym)
            } else {
                symbol.to_string()
            }
        };

        let data: serde_json::Value = self.client
            .get(format!("https://finnhub.io/api/v1/quote?symbol={}&token={}", api_symbol, api_key))
            .send().await.map_err(|e| format!("Finnhub 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Finnhub API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Finnhub 解析失敗: {}", e))?;

        // Finnhub returns c=0 for invalid symbols
        let price = data["c"].as_f64().unwrap_or(0.0);
        if price == 0.0 {
            return Err(format!("Finnhub 找不到: {}。股票用 AAPL，加密用 BINANCE:BTCUSDT", symbol));
        }

        Ok(AssetDataBuilder::new(symbol, "finnhub")
            .price(price)
            .change_24h(data["d"].as_f64())
            .change_percent_24h(data["dp"].as_f64())
            .high_24h(data["h"].as_f64())
            .low_24h(data["l"].as_f64())
            .extra_f64("開盤價", data["o"].as_f64())
            .extra_f64("前收盤價", data["pc"].as_f64())
            .build())
    }

    /// 限流並行查詢 — Finnhub 沒有批量 endpoint，限制同時 3 個 request
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        let api_key = self.api_key.as_ref().ok_or("Finnhub 需要 API Key")?.clone();
        let client = self.client.clone();

        use futures::stream::{self, StreamExt};
        let results: Vec<_> = stream::iter(symbols.to_vec())
            .map(|sym| {
                let c = client.clone();
                let key = api_key.clone();
                async move {
                    let api_symbol = if sym.contains(':') {
                        sym.clone()
                    } else {
                        let s = sym.to_uppercase();
                        let looks_crypto = s.ends_with("USDT") || s.ends_with("USD")
                            || s.contains('-') || s.contains('/');
                        if looks_crypto { format!("BINANCE:{}", to_binance_symbol(&sym)) } else { sym.clone() }
                    };
                    let data: serde_json::Value = c
                        .get(format!("https://finnhub.io/api/v1/quote?symbol={}&token={}", api_symbol, key))
                        .send().await.map_err(|e| format!("Finnhub: {}", e))?
                        .json().await.map_err(|e| format!("Finnhub: {}", e))?;
                    let price = data["c"].as_f64().unwrap_or(0.0);
                    if price == 0.0 { return Err(format!("Finnhub 找不到: {}", sym)); }
                    Ok(AssetDataBuilder::new(&sym, "finnhub")
                        .price(price)
                        .change_24h(data["d"].as_f64())
                        .change_percent_24h(data["dp"].as_f64())
                        .high_24h(data["h"].as_f64())
                        .low_24h(data["l"].as_f64())
                        .extra_f64("開盤價", data["o"].as_f64())
                        .extra_f64("前收盤價", data["pc"].as_f64())
                        .build())
                }
            })
            .buffer_unordered(3)
            .collect()
            .await;

        let mut out = Vec::new();
        for r in results {
            match r {
                Ok(data) => out.push(data),
                Err(e) => eprintln!("Finnhub 跳過: {}", e),
            }
        }
        Ok(out)
    }
}

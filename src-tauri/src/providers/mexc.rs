use super::traits::*;

pub struct MexcProvider {
    client: reqwest::Client,
}

impl MexcProvider {
    pub fn new() -> Self {
        Self {
            client: shared_client(),
        }
    }
}

/// Convert to MEXC format: BTCUSDT
fn to_mexc_symbol(symbol: &str) -> String {
    let (base, quote) = parse_crypto_symbol(symbol);
    let q = if quote == "USD" { "USDT" } else { &quote };
    format!("{}{}", base, q)
}

fn parse_mexc_ticker(symbol: &str, item: &serde_json::Value) -> AssetData {
    let pf = |k: &str| {
        item[k]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| item[k].as_f64())
    };
    AssetDataBuilder::new(symbol, "mexc")
        .price(pf("lastPrice").unwrap_or(0.0))
        .currency("USDT")
        .change_24h(pf("priceChange"))
        .change_percent_24h(pf("priceChangePercent"))
        .high_24h(pf("highPrice"))
        .low_24h(pf("lowPrice"))
        .volume(pf("volume"))
        .extra_f64("quote_volume", pf("quoteVolume"))
        .build()
}

#[async_trait::async_trait]
impl DataProvider for MexcProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("mexc").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let sym = to_mexc_symbol(symbol);
        let url = format!("https://api.mexc.com/api/v3/ticker/24hr?symbol={}", sym);
        let data: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("MEXC 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("MEXC API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("MEXC 解析失敗: {}", e))?;

        Ok(parse_mexc_ticker(symbol, &data))
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        // MEXC returns all tickers when no symbol specified
        let url = "https://api.mexc.com/api/v3/ticker/24hr";
        let arr: Vec<serde_json::Value> = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("MEXC 批量連接失敗: {}", e))?
            .json()
            .await
            .map_err(|e| format!("MEXC 批量解析失敗: {}", e))?;

        let mut map = std::collections::HashMap::new();
        for item in &arr {
            if let Some(s) = item["symbol"].as_str() {
                map.insert(s.to_string(), item);
            }
        }

        let mut out = Vec::new();
        for sym in symbols {
            let mexc_sym = to_mexc_symbol(sym);
            if let Some(item) = map.get(&mexc_sym) {
                out.push(parse_mexc_ticker(sym, item));
            }
        }
        Ok(out)
    }
}

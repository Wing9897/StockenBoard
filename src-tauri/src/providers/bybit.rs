use super::traits::*;

pub struct BybitProvider {
    client: reqwest::Client,
}

impl BybitProvider {
    pub fn new() -> Self {
        Self { client: shared_client() }
    }
}

/// Convert to Bybit spot format: BTCUSDT
fn to_bybit_symbol(symbol: &str) -> String {
    let (base, quote) = parse_crypto_symbol(symbol);
    let q = if quote == "USD" { "USDT" } else { &quote };
    format!("{}{}", base, q)
}

fn parse_bybit_ticker(symbol: &str, item: &serde_json::Value) -> AssetData {
    let pf = |k: &str| item[k].as_str().and_then(|s| s.parse::<f64>().ok());
    let last = pf("lastPrice").unwrap_or(0.0);
    let prev = pf("prevPrice24h").unwrap_or(0.0);
    let change = if prev > 0.0 { Some(last - prev) } else { None };
    AssetDataBuilder::new(symbol, "bybit")
        .price(last)
        .currency("USDT")
        .change_24h(change)
        .change_percent_24h(pf("price24hPcnt").map(|p| p * 100.0))
        .high_24h(pf("highPrice24h"))
        .low_24h(pf("lowPrice24h"))
        .volume(pf("volume24h"))
        .extra_f64("成交額", pf("turnover24h"))
        .build()
}

#[async_trait::async_trait]
impl DataProvider for BybitProvider {
    fn info(&self) -> ProviderInfo { get_provider_info("bybit").unwrap() }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let sym = to_bybit_symbol(symbol);
        let url = format!("https://api.bybit.com/v5/market/tickers?category=spot&symbol={}", sym);
        let data: serde_json::Value = self.client.get(&url)
            .send().await.map_err(|e| format!("Bybit 連接失敗: {}", e))?
            .json().await.map_err(|e| format!("Bybit 解析失敗: {}", e))?;

        let item = data["result"]["list"].as_array()
            .and_then(|a| a.first())
            .ok_or("Bybit: 找不到交易對數據")?;

        Ok(parse_bybit_ticker(symbol, item))
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        // Bybit doesn't support multi-symbol query, fetch all spot tickers
        let url = "https://api.bybit.com/v5/market/tickers?category=spot";
        let data: serde_json::Value = self.client.get(url)
            .send().await.map_err(|e| format!("Bybit 批量連接失敗: {}", e))?
            .json().await.map_err(|e| format!("Bybit 批量解析失敗: {}", e))?;

        let list = data["result"]["list"].as_array().ok_or("Bybit: 無結果")?;
        let mut map = std::collections::HashMap::new();
        for item in list {
            if let Some(s) = item["symbol"].as_str() { map.insert(s.to_string(), item); }
        }

        let mut out = Vec::new();
        for sym in symbols {
            let bybit_sym = to_bybit_symbol(sym);
            if let Some(item) = map.get(&bybit_sym) {
                out.push(parse_bybit_ticker(sym, item));
            }
        }
        Ok(out)
    }
}

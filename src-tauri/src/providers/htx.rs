use super::traits::*;

pub struct HtxProvider {
    client: reqwest::Client,
}

impl HtxProvider {
    pub fn new() -> Self {
        Self { client: shared_client() }
    }
}

/// Convert to HTX format: btcusdt (lowercase)
fn to_htx_symbol(symbol: &str) -> String {
    let (base, quote) = parse_crypto_symbol(symbol);
    let q = if quote == "USD" { "USDT" } else { &quote };
    format!("{}{}", base, q).to_lowercase()
}

fn parse_htx_ticker(symbol: &str, tick: &serde_json::Value) -> AssetData {
    let pf = |k: &str| tick[k].as_f64();
    let close = pf("close").unwrap_or(0.0);
    let open = pf("open").unwrap_or(0.0);
    let change = if open > 0.0 { Some(close - open) } else { None };
    let change_pct = if open > 0.0 { Some((close - open) / open * 100.0) } else { None };

    AssetDataBuilder::new(symbol, "htx")
        .price(close).currency("USDT")
        .change_24h(change).change_percent_24h(change_pct)
        .high_24h(pf("high")).low_24h(pf("low"))
        .volume(pf("amount"))
        .extra_f64("成交額", pf("vol"))
        .build()
}

#[async_trait::async_trait]
impl DataProvider for HtxProvider {
    fn info(&self) -> ProviderInfo { get_provider_info("htx").unwrap() }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let pair = to_htx_symbol(symbol);
        let url = format!("https://api.huobi.pro/market/detail/merged?symbol={}", pair);
        let data: serde_json::Value = self.client.get(&url)
            .send().await.map_err(|e| format!("HTX 連接失敗: {}", e))?
            .json().await.map_err(|e| format!("HTX 解析失敗: {}", e))?;

        if data["status"].as_str() != Some("ok") {
            return Err(format!("HTX: {}", data["err-msg"].as_str().unwrap_or("未知錯誤")));
        }
        Ok(parse_htx_ticker(symbol, &data["tick"]))
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        // HTX /market/tickers returns all tickers
        let url = "https://api.huobi.pro/market/tickers";
        let data: serde_json::Value = self.client.get(url)
            .send().await.map_err(|e| format!("HTX 批量連接失敗: {}", e))?
            .json().await.map_err(|e| format!("HTX 批量解析失敗: {}", e))?;

        let tickers = data["data"].as_array().ok_or("HTX: 無結果")?;
        let mut map = std::collections::HashMap::new();
        for t in tickers {
            if let Some(s) = t["symbol"].as_str() { map.insert(s.to_string(), t); }
        }

        let mut out = Vec::new();
        for sym in symbols {
            let htx_sym = to_htx_symbol(sym);
            if let Some(t) = map.get(&htx_sym) {
                out.push(parse_htx_ticker(sym, t));
            }
        }
        Ok(out)
    }
}

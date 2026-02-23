use super::traits::*;

pub struct OkxProvider {
    client: reqwest::Client,
}

impl OkxProvider {
    pub fn new() -> Self {
        Self { client: shared_client() }
    }
}

/// Convert to OKX format: BTC-USDT
fn to_okx_symbol(symbol: &str) -> String {
    let (base, quote) = parse_crypto_symbol(symbol);
    let q = if quote == "USD" { "USDT" } else { &quote };
    format!("{}-{}", base, q)
}

fn parse_okx_ticker(symbol: &str, item: &serde_json::Value) -> AssetData {
    let pf = |k: &str| item[k].as_str().and_then(|s| s.parse::<f64>().ok());
    let last = pf("last").unwrap_or(0.0);
    let open = pf("open24h").unwrap_or(0.0);
    let change = if open > 0.0 { Some(last - open) } else { None };
    let change_pct = if open > 0.0 { Some((last - open) / open * 100.0) } else { None };

    AssetDataBuilder::new(symbol, "okx")
        .price(last).currency("USDT")
        .change_24h(change).change_percent_24h(change_pct)
        .high_24h(pf("high24h")).low_24h(pf("low24h"))
        .volume(pf("vol24h"))
        .extra_f64("成交額", pf("volCcy24h"))
        .build()
}

#[async_trait::async_trait]
impl DataProvider for OkxProvider {
    fn info(&self) -> ProviderInfo { get_provider_info("okx").unwrap() }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let inst = to_okx_symbol(symbol);
        let url = format!("https://www.okx.com/api/v5/market/ticker?instId={}", inst);
        let data: serde_json::Value = self.client.get(&url)
            .send().await.map_err(|e| format!("OKX 連接失敗: {}", e))?
            .json().await.map_err(|e| format!("OKX 解析失敗: {}", e))?;

        let item = data["data"].as_array()
            .and_then(|a| a.first())
            .ok_or("OKX: 找不到交易對數據")?;

        Ok(parse_okx_ticker(symbol, item))
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        // OKX tickers endpoint returns all SPOT tickers
        let url = "https://www.okx.com/api/v5/market/tickers?instType=SPOT";
        let data: serde_json::Value = self.client.get(url)
            .send().await.map_err(|e| format!("OKX 批量連接失敗: {}", e))?
            .json().await.map_err(|e| format!("OKX 批量解析失敗: {}", e))?;

        let list = data["data"].as_array().ok_or("OKX: 無結果")?;
        let mut map = std::collections::HashMap::new();
        for item in list {
            if let Some(s) = item["instId"].as_str() { map.insert(s.to_string(), item); }
        }

        let mut out = Vec::new();
        for sym in symbols {
            let okx_sym = to_okx_symbol(sym);
            if let Some(item) = map.get(&okx_sym) {
                out.push(parse_okx_ticker(sym, item));
            }
        }
        Ok(out)
    }
}

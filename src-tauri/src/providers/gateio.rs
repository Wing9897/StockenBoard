use super::traits::*;

pub struct GateioProvider {
    client: reqwest::Client,
}

impl GateioProvider {
    pub fn new() -> Self {
        Self { client: shared_client() }
    }
}

/// Convert to Gate.io format: BTC_USDT
fn to_gateio_symbol(symbol: &str) -> String {
    let (base, quote) = parse_crypto_symbol(symbol);
    let q = if quote == "USD" { "USDT" } else { &quote };
    format!("{}_{}", base, q)
}

fn parse_gateio_ticker(symbol: &str, item: &serde_json::Value) -> AssetData {
    let pf = |k: &str| item[k].as_str().and_then(|s| s.parse::<f64>().ok());
    let last = pf("last").unwrap_or(0.0);
    let pct = pf("change_percentage");
    // Gate.io change_percentage is already in percent (e.g. -4.47)
    // Calculate absolute change from percentage
    let change = pct.map(|p| last * p / (100.0 + p));

    AssetDataBuilder::new(symbol, "gateio")
        .price(last)
        .currency("USDT")
        .change_24h(change)
        .change_percent_24h(pct)
        .high_24h(pf("high_24h")).low_24h(pf("low_24h"))
        .volume(pf("base_volume"))
        .extra_f64("quote_volume", pf("quote_volume"))
        .build()
}

#[async_trait::async_trait]
impl DataProvider for GateioProvider {
    fn info(&self) -> ProviderInfo { get_provider_info("gateio").unwrap() }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let pair = to_gateio_symbol(symbol);
        let url = format!("https://api.gateio.ws/api/v4/spot/tickers?currency_pair={}", pair);
        let arr: Vec<serde_json::Value> = self.client.get(&url)
            .send().await.map_err(|e| format!("Gate.io 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Gate.io API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Gate.io 解析失敗: {}", e))?;

        let item = arr.first().ok_or("Gate.io: 找不到交易對數據")?;
        Ok(parse_gateio_ticker(symbol, item))
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        // Gate.io returns all tickers when no currency_pair specified
        let url = "https://api.gateio.ws/api/v4/spot/tickers";
        let arr: Vec<serde_json::Value> = self.client.get(url)
            .send().await.map_err(|e| format!("Gate.io 批量連接失敗: {}", e))?
            .json().await.map_err(|e| format!("Gate.io 批量解析失敗: {}", e))?;

        let mut map = std::collections::HashMap::new();
        for item in &arr {
            if let Some(s) = item["currency_pair"].as_str() { map.insert(s.to_string(), item); }
        }

        let mut out = Vec::new();
        for sym in symbols {
            let gate_sym = to_gateio_symbol(sym);
            if let Some(item) = map.get(&gate_sym) {
                out.push(parse_gateio_ticker(sym, item));
            }
        }
        Ok(out)
    }
}

use super::traits::*;

pub struct FcsApiProvider {
    client: reqwest::Client,
    api_key: String,
}

impl FcsApiProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: shared_client(),
            api_key: api_key.unwrap_or_default(),
        }
    }
}

// FCS API: https://api-v4.fcsapi.com/stock/latest?symbol=AAPL&access_key=KEY
// Response: { "status": true, "code": 200, "response": [{ "s": "AAPL", "c": "150.00", "h": "151", "l": "149", "ch": "1.5", "cp": "1.01", ... }] }

#[async_trait::async_trait]
impl DataProvider for FcsApiProvider {
    fn info(&self) -> ProviderInfo { get_provider_info("fcsapi").unwrap() }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        if self.api_key.is_empty() { return Err("FCS API: 需要 API Key".into()); }
        let url = format!(
            "https://api-v4.fcsapi.com/stock/latest?symbol={}&access_key={}",
            symbol.to_uppercase(), self.api_key
        );
        let data: serde_json::Value = self.client.get(&url)
            .send().await.map_err(|e| format!("FCS API 連接失敗: {}", e))?
            .json().await.map_err(|e| format!("FCS API 解析失敗: {}", e))?;

        let item = data["response"].as_array()
            .and_then(|a| a.first())
            .ok_or("FCS API: 找不到數據")?;

        Ok(parse_fcs_item(symbol, item))
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if self.api_key.is_empty() { return Err("FCS API: 需要 API Key".into()); }

        // FCS supports comma-separated symbols
        let syms = symbols.iter().map(|s| s.to_uppercase()).collect::<Vec<_>>().join(",");
        let url = format!(
            "https://api-v4.fcsapi.com/stock/latest?symbol={}&access_key={}",
            syms, self.api_key
        );
        let data: serde_json::Value = self.client.get(&url)
            .send().await.map_err(|e| format!("FCS API 批量連接失敗: {}", e))?
            .json().await.map_err(|e| format!("FCS API 批量解析失敗: {}", e))?;

        let arr = data["response"].as_array().ok_or("FCS API: 無結果")?;
        let mut map = std::collections::HashMap::new();
        for item in arr {
            if let Some(s) = item["s"].as_str() { map.insert(s.to_uppercase(), item); }
        }

        let mut out = Vec::new();
        for sym in symbols {
            let key = sym.to_uppercase();
            if let Some(item) = map.get(&key) {
                out.push(parse_fcs_item(sym, item));
            }
        }
        Ok(out)
    }
}

fn parse_fcs_item(symbol: &str, item: &serde_json::Value) -> AssetData {
    let pf = |k: &str| item[k].as_str().and_then(|s| s.parse::<f64>().ok())
        .or_else(|| item[k].as_f64());
    AssetDataBuilder::new(symbol, "fcsapi")
        .price(pf("c").unwrap_or(0.0))
        .currency("USD")
        .change_24h(pf("ch"))
        .change_percent_24h(pf("cp"))
        .high_24h(pf("h")).low_24h(pf("l"))
        .volume(pf("v"))
        .extra_str("交易所", item["cty"].as_str())
        .build()
}

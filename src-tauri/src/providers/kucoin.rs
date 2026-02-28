use super::traits::*;

pub struct KuCoinProvider {
    client: reqwest::Client,
}

impl KuCoinProvider {
    pub fn new() -> Self {
        Self {
            client: shared_client(),
        }
    }
}

/// Convert to KuCoin format: BTC-USDT
fn to_kucoin_symbol(symbol: &str) -> String {
    let (base, quote) = parse_crypto_symbol(symbol);
    let q = if quote == "USD" { "USDT" } else { &quote };
    format!("{}-{}", base, q)
}

fn parse_kucoin_ticker(symbol: &str, data: &serde_json::Value) -> AssetData {
    let pf = |k: &str| data[k].as_str().and_then(|s| s.parse::<f64>().ok());
    let price = pf("last").unwrap_or(0.0);
    AssetDataBuilder::new(symbol, "kucoin")
        .price(price)
        .currency("USDT")
        .change_24h(pf("changePrice"))
        .change_percent_24h(pf("changeRate").map(|r| r * 100.0))
        .high_24h(pf("high"))
        .low_24h(pf("low"))
        .volume(pf("vol"))
        .extra_f64("quote_volume", pf("volValue"))
        .extra_f64("avg_price", pf("averagePrice"))
        .build()
}

#[async_trait::async_trait]
impl DataProvider for KuCoinProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("kucoin").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let pair = to_kucoin_symbol(symbol);
        let url = format!("https://api.kucoin.com/api/v1/market/stats?symbol={}", pair);
        let resp: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("KuCoin 連接失敗: {}", e))?
            .json()
            .await
            .map_err(|e| format!("KuCoin 解析失敗: {}", e))?;

        if resp["code"].as_str() != Some("200000") {
            return Err(format!(
                "KuCoin: {}",
                resp["msg"].as_str().unwrap_or("未知錯誤")
            ));
        }
        Ok(parse_kucoin_ticker(symbol, &resp["data"]))
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        // KuCoin allTickers endpoint returns all tickers at once
        let url = "https://api.kucoin.com/api/v1/market/allTickers";
        let resp: serde_json::Value = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("KuCoin 批量連接失敗: {}", e))?
            .json()
            .await
            .map_err(|e| format!("KuCoin 批量解析失敗: {}", e))?;

        let tickers = resp["data"]["ticker"].as_array().ok_or("KuCoin: 無結果")?;
        let mut map = std::collections::HashMap::new();
        for t in tickers {
            if let Some(s) = t["symbol"].as_str() {
                map.insert(s.to_string(), t);
            }
        }

        let mut out = Vec::new();
        for sym in symbols {
            let kc_sym = to_kucoin_symbol(sym);
            if let Some(t) = map.get(&kc_sym) {
                out.push(parse_kucoin_ticker(sym, t));
            }
        }
        Ok(out)
    }
}

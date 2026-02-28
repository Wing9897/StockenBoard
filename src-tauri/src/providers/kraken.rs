use super::traits::*;

pub struct KrakenProvider {
    client: reqwest::Client,
}

impl KrakenProvider {
    pub fn new() -> Self {
        Self {
            client: shared_client(),
        }
    }
}

/// Convert symbol to Kraken format: XBTUSD, ETHUSD
fn to_kraken_symbol(symbol: &str) -> String {
    let (base, quote) = parse_crypto_symbol(symbol);
    let b = match base.as_str() {
        "BTC" => "XBT",
        _ => &base,
    };
    let q = match quote.as_str() {
        "USDT" => "USD",
        _ => &quote,
    };
    format!("{}{}", b, q)
}

#[async_trait::async_trait]
impl DataProvider for KrakenProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("kraken").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let pair = to_kraken_symbol(symbol);
        let url = format!("https://api.kraken.com/0/public/Ticker?pair={}", pair);
        let data: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Kraken 連接失敗: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Kraken 解析失敗: {}", e))?;

        if let Some(errs) = data["error"].as_array() {
            if !errs.is_empty() {
                let msg = errs
                    .iter()
                    .filter_map(|e| e.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                if !msg.is_empty() {
                    return Err(format!("Kraken: {}", msg));
                }
            }
        }

        // Response key may differ from input (e.g. XBTUSD -> XXBTZUSD)
        let result = &data["result"];
        let ticker = result
            .as_object()
            .and_then(|m| m.values().next())
            .ok_or("Kraken: 找不到交易對數據")?;

        let price = ticker["c"][0]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let open = ticker["o"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let high = ticker["h"][1].as_str().and_then(|s| s.parse::<f64>().ok());
        let low = ticker["l"][1].as_str().and_then(|s| s.parse::<f64>().ok());
        let volume = ticker["v"][1].as_str().and_then(|s| s.parse::<f64>().ok());
        let change = if open > 0.0 { Some(price - open) } else { None };
        let change_pct = if open > 0.0 {
            Some((price - open) / open * 100.0)
        } else {
            None
        };

        Ok(AssetDataBuilder::new(symbol, "kraken")
            .price(price)
            .currency("USD")
            .change_24h(change)
            .change_percent_24h(change_pct)
            .high_24h(high)
            .low_24h(low)
            .volume(volume)
            .build())
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.len() <= 1 {
            return if symbols.is_empty() {
                Ok(vec![])
            } else {
                self.fetch_price(&symbols[0]).await.map(|d| vec![d])
            };
        }
        let pairs: Vec<String> = symbols.iter().map(|s| to_kraken_symbol(s)).collect();
        let url = format!(
            "https://api.kraken.com/0/public/Ticker?pair={}",
            pairs.join(",")
        );
        let data: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Kraken 批量連接失敗: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Kraken 批量解析失敗: {}", e))?;

        let result = data["result"].as_object().ok_or("Kraken: 無結果")?;
        // Build lookup: Kraken returns keys like XXBTZUSD (X-prefix for crypto, Z-prefix for fiat)
        // We need to match our requested pairs to the returned keys
        let mut out = Vec::new();
        for (i, sym) in symbols.iter().enumerate() {
            let kraken_sym = &pairs[i];
            // Try exact match first, then search for key containing our pair
            let ticker = result
                .get(kraken_sym.as_str())
                .or_else(|| {
                    result
                        .iter()
                        .find(|(k, _)| k.contains(kraken_sym.as_str()))
                        .map(|(_, v)| v)
                })
                .or_else(|| result.values().nth(i));
            if let Some(t) = ticker {
                let price = t["c"][0]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let open = t["o"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let high = t["h"][1].as_str().and_then(|s| s.parse::<f64>().ok());
                let low = t["l"][1].as_str().and_then(|s| s.parse::<f64>().ok());
                let volume = t["v"][1].as_str().and_then(|s| s.parse::<f64>().ok());
                let change = if open > 0.0 { Some(price - open) } else { None };
                let change_pct = if open > 0.0 {
                    Some((price - open) / open * 100.0)
                } else {
                    None
                };
                out.push(
                    AssetDataBuilder::new(sym, "kraken")
                        .price(price)
                        .currency("USD")
                        .change_24h(change)
                        .change_percent_24h(change_pct)
                        .high_24h(high)
                        .low_24h(low)
                        .volume(volume)
                        .build(),
                );
            }
        }
        Ok(out)
    }
}

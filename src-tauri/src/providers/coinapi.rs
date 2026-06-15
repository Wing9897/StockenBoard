use super::traits::*;

pub struct CoinApiProvider {
    client: reqwest::Client,
    api_key: String,
}

impl CoinApiProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: shared_client(),
            api_key: api_key.unwrap_or_default(),
        }
    }
}

/// Convert to CoinAPI asset ID: BTC
fn to_coinapi_base(symbol: &str) -> String {
    let (base, _) = parse_crypto_symbol(symbol);
    base
}

#[async_trait::async_trait]
impl DataProvider for CoinApiProvider {
    fn info(&self) -> ProviderInfo {
        provider_info_or_panic("coinapi")
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        if self.api_key.is_empty() {
            return Err("CoinAPI: requires API key".into());
        }
        let base = to_coinapi_base(symbol);
        let url = format!("https://rest.coinapi.io/v1/exchangerate/{}/USD", base);
        let data: serde_json::Value = self
            .client
            .get(&url)
            .header("X-CoinAPI-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("CoinAPI connection failed: {}", e))?
            .error_for_status()
            .map_err(|e| format!("CoinAPI API error: {}", e))?
            .json()
            .await
            .map_err(|e| format!("CoinAPI parse failed: {}", e))?;

        let price = data["rate"].as_f64().unwrap_or(0.0);
        Ok(AssetDataBuilder::new(symbol, "coinapi")
            .price(price)
            .currency("USD")
            .build())
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if self.api_key.is_empty() {
            return Err("CoinAPI: requires API key".into());
        }

        // CoinAPI supports batch via /v1/exchangerate/{base} but one at a time
        // Use concurrent requests with limit
        use futures::stream::{self, StreamExt};
        let results: Vec<_> = stream::iter(symbols.to_vec())
            .map(|sym| {
                let client = self.client.clone();
                let key = self.api_key.clone();
                async move {
                    let base = to_coinapi_base(&sym);
                    let url = format!("https://rest.coinapi.io/v1/exchangerate/{}/USD", base);
                    match client.get(&url).header("X-CoinAPI-Key", &key).send().await {
                        Ok(resp) => match resp.json::<serde_json::Value>().await {
                            Ok(data) => {
                                let price = data["rate"].as_f64().unwrap_or(0.0);
                                Ok(AssetDataBuilder::new(&sym, "coinapi")
                                    .price(price)
                                    .currency("USD")
                                    .build())
                            }
                            Err(e) => Err(format!("CoinAPI parse failed: {}", e)),
                        },
                        Err(e) => Err(format!("CoinAPI connection failed: {}", e)),
                    }
                }
            })
            .buffer_unordered(2) // Conservative: free tier is limited
            .collect()
            .await;

        Ok(results.into_iter().filter_map(|r| r.ok()).collect())
    }
}

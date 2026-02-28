use super::traits::*;

pub struct PolymarketProvider {
    client: reqwest::Client,
}

impl PolymarketProvider {
    pub fn new() -> Self {
        Self {
            client: shared_client(),
        }
    }
}

#[async_trait::async_trait]
impl DataProvider for PolymarketProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("polymarket").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        // symbol = condition_id for the market
        let data: serde_json::Value = self
            .client
            .get(format!("https://clob.polymarket.com/markets/{}", symbol))
            .send()
            .await
            .map_err(|e| format!("Polymarket 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Polymarket API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Polymarket 解析失敗: {}", e))?;

        let price = data["outcome_prices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let volume = data["volume"].as_str().and_then(|s| s.parse::<f64>().ok());

        let outcomes = data["outcomes"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|o| o.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        });

        Ok(AssetDataBuilder::new(symbol, "polymarket")
            .price(price)
            .currency("PROB")
            .volume(volume)
            .extra_str("question", data["question"].as_str())
            .extra_str("end_date", data["end_date_iso"].as_str())
            .extra_str("outcomes", outcomes.as_deref())
            .build())
    }

    /// 限流並行查詢 — Polymarket 每個 market 是獨立 condition_id，限制同時 3 個
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        let client = self.client.clone();

        use futures::stream::{self, StreamExt};
        let results: Vec<_> = stream::iter(symbols.to_vec())
            .map(|sym| {
                let c = client.clone();
                async move {
                    let data: serde_json::Value = c
                        .get(format!("https://clob.polymarket.com/markets/{}", sym))
                        .send()
                        .await
                        .map_err(|e| format!("Polymarket: {}", e))?
                        .json()
                        .await
                        .map_err(|e| format!("Polymarket: {}", e))?;
                    let price = data["outcome_prices"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let volume = data["volume"].as_str().and_then(|s| s.parse::<f64>().ok());
                    let outcomes = data["outcomes"].as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|o| o.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    });
                    Ok::<AssetData, String>(
                        AssetDataBuilder::new(&sym, "polymarket")
                            .price(price)
                            .currency("PROB")
                            .volume(volume)
                            .extra_str("question", data["question"].as_str())
                            .extra_str("end_date", data["end_date_iso"].as_str())
                            .extra_str("outcomes", outcomes.as_deref())
                            .build(),
                    )
                }
            })
            .buffer_unordered(3)
            .collect()
            .await;

        let mut out = Vec::new();
        for r in results {
            match r {
                Ok(data) => out.push(data),
                Err(e) => eprintln!("Polymarket 跳過: {}", e),
            }
        }
        Ok(out)
    }
}

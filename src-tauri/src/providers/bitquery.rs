use super::traits::*;

pub struct BitqueryProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl BitqueryProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { client: shared_client(), api_key }
    }
}

#[async_trait::async_trait]
impl DataProvider for BitqueryProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("bitquery").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("Bitquery 需要 API Key (OAuth token)")?;

        // Bitquery v2 uses streaming.bitquery.io/graphql with Bearer token
        let query = format!(r#"{{
            EVM(dataset: combined, network: eth) {{
                DEXTradeByTokens(
                    limit: {{count: 1}}
                    orderBy: {{descending: Block_Time}}
                    where: {{Trade: {{Currency: {{SmartContract: {{is: "{}"}}}}}}}}
                ) {{
                    Trade {{
                        PriceInUSD
                        AmountInUSD
                    }}
                }}
            }}
        }}"#, symbol);

        let data: serde_json::Value = self.client
            .post("https://streaming.bitquery.io/graphql")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "query": query }))
            .send().await.map_err(|e| format!("Bitquery 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Bitquery API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Bitquery 解析失敗: {}", e))?;

        let trade = &data["data"]["EVM"]["DEXTradeByTokens"][0]["Trade"];

        Ok(AssetDataBuilder::new(symbol, "bitquery")
            .price(trade["PriceInUSD"].as_f64().unwrap_or(0.0))
            .volume(trade["AmountInUSD"].as_f64())
            .build())
    }

    /// 限流並行查詢 — Bitquery GraphQL 可以合併但太複雜，限制同時 2 個
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        let api_key = self.api_key.as_ref().ok_or("Bitquery 需要 API Key")?.clone();
        let client = self.client.clone();

        use futures::stream::{self, StreamExt};
        let results: Vec<_> = stream::iter(symbols.to_vec())
            .map(|sym| {
                let c = client.clone();
                let key = api_key.clone();
                async move {
                    let query = format!(r#"{{
                        EVM(dataset: combined, network: eth) {{
                            DEXTradeByTokens(
                                limit: {{count: 1}}
                                orderBy: {{descending: Block_Time}}
                                where: {{Trade: {{Currency: {{SmartContract: {{is: "{}"}}}}}}}}
                            ) {{ Trade {{ PriceInUSD AmountInUSD }} }}
                        }}
                    }}"#, sym);
                    let data: serde_json::Value = c
                        .post("https://streaming.bitquery.io/graphql")
                        .header("Authorization", format!("Bearer {}", key))
                        .header("Content-Type", "application/json")
                        .json(&serde_json::json!({ "query": query }))
                        .send().await.map_err(|e| format!("Bitquery: {}", e))?
                        .json().await.map_err(|e| format!("Bitquery: {}", e))?;
                    let trade = &data["data"]["EVM"]["DEXTradeByTokens"][0]["Trade"];
                    Ok::<AssetData, String>(AssetDataBuilder::new(&sym, "bitquery")
                        .price(trade["PriceInUSD"].as_f64().unwrap_or(0.0))
                        .volume(trade["AmountInUSD"].as_f64())
                        .build())
                }
            })
            .buffer_unordered(2)
            .collect()
            .await;

        let mut out = Vec::new();
        for r in results {
            match r {
                Ok(data) => out.push(data),
                Err(e) => eprintln!("Bitquery 跳過: {}", e),
            }
        }
        Ok(out)
    }
}

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
}

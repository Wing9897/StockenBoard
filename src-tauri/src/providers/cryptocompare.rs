use super::traits::*;

pub struct CryptoCompareProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl CryptoCompareProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: shared_client(),
            api_key,
        }
    }

    fn build_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.get(url);
        if let Some(key) = &self.api_key {
            if !key.is_empty() {
                req = req.header("authorization", format!("Apikey {}", key));
            }
        }
        req
    }

    fn parse_coin(symbol: &str, base: &str, data: &serde_json::Value) -> Result<AssetData, String> {
        let raw = &data["RAW"][base]["USD"];
        if raw.is_null() {
            return Err(format!(
                "CryptoCompare 找不到: {} (查詢: {})。格式: BTC, ETH",
                symbol, base
            ));
        }
        Ok(AssetDataBuilder::new(symbol, "cryptocompare")
            .price(raw["PRICE"].as_f64().unwrap_or(0.0))
            .change_24h(raw["CHANGE24HOUR"].as_f64())
            .change_percent_24h(raw["CHANGEPCT24HOUR"].as_f64())
            .high_24h(raw["HIGH24HOUR"].as_f64())
            .low_24h(raw["LOW24HOUR"].as_f64())
            .volume(raw["VOLUME24HOUR"].as_f64())
            .market_cap(raw["MKTCAP"].as_f64())
            .build())
    }
}

#[async_trait::async_trait]
impl DataProvider for CryptoCompareProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("cryptocompare").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let base = to_base_symbol(symbol);
        let url = format!(
            "https://min-api.cryptocompare.com/data/pricemultifull?fsyms={}&tsyms=USD",
            base
        );

        let data: serde_json::Value = self
            .build_request(&url)
            .send()
            .await
            .map_err(|e| format!("CryptoCompare 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("CryptoCompare API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("CryptoCompare 解析失敗: {}", e))?;

        Self::parse_coin(symbol, &base, &data)
    }

    /// 批量查詢 — fsyms=BTC,ETH 一次查多個幣
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        let mappings: Vec<(String, String)> = symbols
            .iter()
            .map(|s| (s.clone(), to_base_symbol(s)))
            .collect();

        let bases: Vec<&str> = mappings.iter().map(|(_, b)| b.as_str()).collect();
        let fsyms = bases.join(",");

        let url = format!(
            "https://min-api.cryptocompare.com/data/pricemultifull?fsyms={}&tsyms=USD",
            fsyms
        );

        let data: serde_json::Value = self
            .build_request(&url)
            .send()
            .await
            .map_err(|e| format!("CryptoCompare 批量連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("CryptoCompare API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("CryptoCompare 批量解析失敗: {}", e))?;

        let mut results = Vec::new();
        for (symbol, base) in &mappings {
            match Self::parse_coin(symbol, base, &data) {
                Ok(asset) => results.push(asset),
                Err(e) => eprintln!("CryptoCompare 批量跳過 {}: {}", symbol, e),
            }
        }
        Ok(results)
    }
}

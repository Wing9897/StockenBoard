use super::traits::*;

pub struct CoinGeckoProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl CoinGeckoProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { client: shared_client(), api_key }
    }

    fn build_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.get(url);
        if let Some(key) = &self.api_key {
            if !key.is_empty() {
                req = req.header("x-cg-demo-api-key", key);
            }
        }
        req
    }

    fn parse_coin(symbol: &str, coin_id: &str, coin: &serde_json::Value) -> Result<AssetData, String> {
        if coin.is_null() {
            return Err(format!("CoinGecko 找不到: {} (查詢ID: {})。請使用 CoinGecko ID 如: bitcoin, ethereum", symbol, coin_id));
        }
        Ok(AssetDataBuilder::new(symbol, "coingecko")
            .price(coin["usd"].as_f64().unwrap_or(0.0))
            .change_percent_24h(coin["usd_24h_change"].as_f64())
            .volume(coin["usd_24h_vol"].as_f64())
            .market_cap(coin["usd_market_cap"].as_f64())
            .build())
    }
}

#[async_trait::async_trait]
impl DataProvider for CoinGeckoProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("coingecko").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let coin_id = to_coingecko_id(symbol);
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&include_24hr_vol=true&include_24hr_change=true&include_market_cap=true",
            coin_id
        );

        let data: serde_json::Value = self.build_request(&url)
            .send().await.map_err(|e| format!("CoinGecko 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("CoinGecko API 錯誤 (可能達到速率限制，建議設定API Key): {}", e))?
            .json().await.map_err(|e| format!("CoinGecko 解析失敗: {}", e))?;

        Self::parse_coin(symbol, &coin_id, &data[&coin_id])
    }

    /// 批量查詢 — 一次 request 查多個幣，大幅減少 API 調用次數
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        // 建立 symbol -> coingecko_id 映射
        let mappings: Vec<(String, String)> = symbols.iter()
            .map(|s| (s.clone(), to_coingecko_id(s)))
            .collect();

        let ids: Vec<&str> = mappings.iter().map(|(_, id)| id.as_str()).collect();
        let ids_str = ids.join(",");

        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&include_24hr_vol=true&include_24hr_change=true&include_market_cap=true",
            ids_str
        );

        let data: serde_json::Value = self.build_request(&url)
            .send().await.map_err(|e| format!("CoinGecko 批量連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("CoinGecko API 錯誤 (速率限制): {}", e))?
            .json().await.map_err(|e| format!("CoinGecko 批量解析失敗: {}", e))?;

        let mut results = Vec::new();
        for (symbol, coin_id) in &mappings {
            match Self::parse_coin(symbol, coin_id, &data[coin_id]) {
                Ok(asset) => results.push(asset),
                Err(e) => eprintln!("CoinGecko 批量查詢跳過 {}: {}", symbol, e),
            }
        }
        Ok(results)
    }
}

use super::traits::*;

pub struct CoinMarketCapProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl CoinMarketCapProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: shared_client(),
            api_key,
        }
    }

    fn parse_coin(symbol: &str, base: &str, data: &serde_json::Value) -> Result<AssetData, String> {
        let coin = &data["data"][base];
        if coin.is_null() {
            return Err(format!(
                "CMC 找不到: {} (查詢: {})。格式: BTC, ETH",
                symbol, base
            ));
        }
        let quote = &coin["quote"]["USD"];
        Ok(AssetDataBuilder::new(symbol, "coinmarketcap")
            .price(quote["price"].as_f64().unwrap_or(0.0))
            .change_24h(None)
            .change_percent_24h(quote["percent_change_24h"].as_f64())
            .volume(quote["volume_24h"].as_f64())
            .market_cap(quote["market_cap"].as_f64())
            .extra_str("name", coin["name"].as_str())
            .extra_i64("cmc_rank", coin["cmc_rank"].as_i64())
            .extra_f64("circulating_supply", coin["circulating_supply"].as_f64())
            .extra_f64("change_7d_pct", quote["percent_change_7d"].as_f64())
            .build())
    }
}

#[async_trait::async_trait]
impl DataProvider for CoinMarketCapProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("coinmarketcap").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("CoinMarketCap 需要 API Key")?;
        let base = to_base_symbol(symbol);
        let url = format!(
            "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol={}",
            base
        );
        let data: serde_json::Value = self
            .client
            .get(&url)
            .header("X-CMC_PRO_API_KEY", api_key)
            .send()
            .await
            .map_err(|e| format!("CMC 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("CMC API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("CMC 解析失敗: {}", e))?;

        Self::parse_coin(symbol, &base, &data)
    }

    /// 批量查詢 — symbol=BTC,ETH 一次查多個
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        let api_key = self.api_key.as_ref().ok_or("CoinMarketCap 需要 API Key")?;
        let mappings: Vec<(String, String)> = symbols
            .iter()
            .map(|s| (s.clone(), to_base_symbol(s)))
            .collect();
        let bases: Vec<&str> = mappings.iter().map(|(_, b)| b.as_str()).collect();
        let syms = bases.join(",");

        let url = format!(
            "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol={}",
            syms
        );
        let data: serde_json::Value = self
            .client
            .get(&url)
            .header("X-CMC_PRO_API_KEY", api_key)
            .send()
            .await
            .map_err(|e| format!("CMC 批量連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("CMC API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("CMC 批量解析失敗: {}", e))?;

        let mut results = Vec::new();
        for (symbol, base) in &mappings {
            match Self::parse_coin(symbol, base, &data) {
                Ok(asset) => results.push(asset),
                Err(e) => eprintln!("CMC 批量跳過 {}: {}", symbol, e),
            }
        }
        Ok(results)
    }
}

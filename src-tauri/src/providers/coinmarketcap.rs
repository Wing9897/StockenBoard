use super::traits::*;
use super::types::*;

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
                "CMC not found: {} (query: {}). Format: BTC, ETH",
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
        provider_info_or_panic("coinmarketcap")
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("CoinMarketCap requires API key")?;
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
            .map_err(|e| format!("CMC connection failed: {}", e))?
            .error_for_status()
            .map_err(|e| format!("CMC API error: {}", e))?
            .json()
            .await
            .map_err(|e| format!("CMC parse failed: {}", e))?;

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

        let api_key = self.api_key.as_ref().ok_or("CoinMarketCap requires API key")?;
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
        let resp = self
            .client
            .get(&url)
            .header("X-CMC_PRO_API_KEY", api_key)
            .send()
            .await
            .map_err(|e| format!("CMC batch connection failed: {}", e))?
            .error_for_status()
            .map_err(|e| format!("CMC batch API error: {}", e))?;

        let body = resp
            .text()
            .await
            .map_err(|e| format!("CMC batch read failed: {}", e))?;

        let data: serde_json::Value = serde_json::from_str(&body)
            .map_err(|_| "CMC batch parse failed (possibly invalid symbol)".to_string())?;

        let mut results = Vec::new();
        for (symbol, base) in &mappings {
            match Self::parse_coin(symbol, base, &data) {
                Ok(asset) => results.push(asset),
                Err(e) => eprintln!("CMC batch skipping {}: {}", symbol, e),
            }
        }
        Ok(results)
    }
}

use super::traits::*;
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

/// 動態 symbol→CoinGecko ID 快取（從 /coins/list API 載入）
static COINGECKO_ID_CACHE: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();

fn id_cache() -> &'static RwLock<HashMap<String, String>> {
    COINGECKO_ID_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub struct CoinGeckoProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl CoinGeckoProvider {
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
                req = req.header("x-cg-demo-api-key", key);
            }
        }
        req
    }

    /// 載入 CoinGecko /coins/list 並建立 symbol(大寫) → id 對照表
    async fn ensure_id_cache(&self) -> Result<(), String> {
        {
            let cache = id_cache().read().await;
            if !cache.is_empty() {
                return Ok(());
            }
        }

        #[derive(serde::Deserialize)]
        struct CoinListItem {
            id: String,
            symbol: String,
        }

        let items: Vec<CoinListItem> = self
            .build_request("https://api.coingecko.com/api/v3/coins/list")
            .send()
            .await
            .map_err(|e| format!("CoinGecko coins/list 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("CoinGecko coins/list API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("CoinGecko coins/list 解析失敗: {}", e))?;

        let mut map = HashMap::with_capacity(items.len());
        for item in &items {
            let key = item.symbol.to_uppercase();
            // 相同 symbol 取 id 最短的（通常是最主流的幣）
            let replace = match map.get(&key) {
                Some(existing_id) => item.id.len() < String::len(existing_id),
                None => true,
            };
            if replace {
                map.insert(key, item.id.clone());
            }
        }
        // 也用 id 本身作為 key，讓使用者可以直接輸入 CoinGecko ID
        for item in &items {
            map.entry(item.id.to_uppercase()).or_insert_with(|| item.id.clone());
        }

        *id_cache().write().await = map;
        Ok(())
    }

    /// 將 symbol 轉換成 CoinGecko ID（動態查表）
    async fn resolve_id(&self, symbol: &str) -> String {
        let (base, _) = parse_crypto_symbol(symbol);

        // 嘗試從快取查找
        if let Ok(()) = self.ensure_id_cache().await {
            let cache = id_cache().read().await;
            if let Some(id) = cache.get(&base) {
                return id.clone();
            }
            // 也嘗試原始輸入（使用者可能直接輸入 CoinGecko ID 如 "bitcoin"）
            let upper = symbol.to_uppercase();
            if let Some(id) = cache.get(&upper) {
                return id.clone();
            }
        }

        // 快取 miss 或載入失敗：fallback 用 lowercase
        symbol.to_lowercase()
    }

    fn parse_coin(
        symbol: &str,
        coin_id: &str,
        coin: &serde_json::Value,
    ) -> Result<AssetData, String> {
        if coin.is_null() {
            return Err(format!(
                "CoinGecko 找不到: {} (查詢ID: {})。請到 coingecko.com 搜尋正確 ID",
                symbol, coin_id
            ));
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
        let coin_id = self.resolve_id(symbol).await;
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&include_24hr_vol=true&include_24hr_change=true&include_market_cap=true",
            coin_id
        );

        let data: serde_json::Value = self
            .build_request(&url)
            .send()
            .await
            .map_err(|e| format!("CoinGecko 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| {
                format!(
                    "CoinGecko API 錯誤 (可能達到速率限制，建議設定API Key): {}",
                    e
                )
            })?
            .json()
            .await
            .map_err(|e| format!("CoinGecko 解析失敗: {}", e))?;

        Self::parse_coin(symbol, &coin_id, &data[&coin_id])
    }

    /// 批量查詢 — 一次 request 查多個幣，大幅減少 API 調用次數
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        // 建立 symbol -> coingecko_id 映射（動態解析）
        let mut mappings = Vec::with_capacity(symbols.len());
        for s in symbols {
            let id = self.resolve_id(s).await;
            mappings.push((s.clone(), id));
        }

        let ids: Vec<&str> = mappings.iter().map(|(_, id)| id.as_str()).collect();
        let ids_str = ids.join(",");

        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&include_24hr_vol=true&include_24hr_change=true&include_market_cap=true",
            ids_str
        );

        let data: serde_json::Value = self
            .build_request(&url)
            .send()
            .await
            .map_err(|e| format!("CoinGecko 批量連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| {
                format!(
                    "CoinGecko 批量 API 錯誤 (可能達到速率限制，建議設定API Key): {}",
                    e
                )
            })?
            .json()
            .await
            .map_err(|e| format!("CoinGecko 批量解析失敗: {}", e))?;

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

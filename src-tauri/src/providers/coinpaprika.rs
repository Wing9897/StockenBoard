use super::traits::*;
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

/// 動態 symbol(大寫) → CoinPaprika ID 快取（從 /v1/coins API 載入）
static COINPAPRIKA_ID_CACHE: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();

fn id_cache() -> &'static RwLock<HashMap<String, String>> {
    COINPAPRIKA_ID_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub struct CoinPaprikaProvider {
    client: reqwest::Client,
}

impl CoinPaprikaProvider {
    pub fn new() -> Self {
        Self {
            client: shared_client(),
        }
    }

    /// 載入 /v1/coins 並建立 symbol(大寫) → id 對照表
    async fn ensure_id_cache(&self) -> Result<(), String> {
        {
            if !id_cache().read().await.is_empty() {
                return Ok(());
            }
        }

        #[derive(serde::Deserialize)]
        struct CoinItem {
            id: String,
            symbol: String,
            rank: u64,
        }

        let items: Vec<CoinItem> = self
            .client
            .get("https://api.coinpaprika.com/v1/coins")
            .send()
            .await
            .map_err(|e| format!("CoinPaprika coins 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("CoinPaprika coins API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("CoinPaprika coins 解析失敗: {}", e))?;

        let mut map: HashMap<String, (String, u64)> = HashMap::with_capacity(items.len());
        for item in &items {
            let key = item.symbol.to_uppercase();
            // 相同 symbol 取 rank 最低的（rank=1 是最主流的幣）
            let rank = if item.rank == 0 { u64::MAX } else { item.rank };
            let replace = match map.get(&key) {
                Some((_, existing_rank)) => rank < *existing_rank,
                None => true,
            };
            if replace {
                map.insert(key, (item.id.clone(), rank));
            }
        }
        // 也把 id 本身作為 key，讓使用者可以直接輸入 CoinPaprika ID
        for item in &items {
            map.entry(item.id.to_uppercase())
                .or_insert_with(|| (item.id.clone(), item.rank));
        }

        *id_cache().write().await = map.into_iter().map(|(k, (v, _))| (k, v)).collect();
        Ok(())
    }

    /// 將 symbol 轉換成 CoinPaprika ID（動態查表）
    async fn resolve_id(&self, symbol: &str) -> String {
        let (base, _) = parse_crypto_symbol(symbol);

        if let Ok(()) = self.ensure_id_cache().await {
            let cache = id_cache().read().await;
            if let Some(id) = cache.get(&base) {
                return id.clone();
            }
            // 也嘗試原始輸入（使用者可能直接輸入 id 如 "btc-bitcoin"）
            let upper = symbol.to_uppercase();
            if let Some(id) = cache.get(&upper) {
                return id.clone();
            }
        }

        // fallback: 猜測 id 格式
        format!("{0}-{0}", base.to_lowercase())
    }
}

fn parse_paprika_ticker(symbol: &str, data: &serde_json::Value) -> AssetData {
    let usd = &data["quotes"]["USD"];
    let pf = |k: &str| usd[k].as_f64();
    let price = pf("price").unwrap_or(0.0);
    let pct = pf("percent_change_24h");
    let change = pct.map(|p| price * p / (100.0 + p));

    AssetDataBuilder::new(symbol, "coinpaprika")
        .price(price)
        .currency("USD")
        .change_24h(change)
        .change_percent_24h(pct)
        .volume(pf("volume_24h"))
        .market_cap(pf("market_cap"))
        .extra_f64("ATH", pf("ath_price"))
        .extra_f64("1h%", pf("percent_change_1h"))
        .extra_f64("7d%", pf("percent_change_7d"))
        .build()
}

#[async_trait::async_trait]
impl DataProvider for CoinPaprikaProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("coinpaprika").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let id = self.resolve_id(symbol).await;
        let url = format!("https://api.coinpaprika.com/v1/tickers/{}", id);
        let data: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("CoinPaprika 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| {
                format!(
                    "CoinPaprika API 錯誤: {} (查詢ID: {}，請確認 symbol 正確)",
                    e, id
                )
            })?
            .json()
            .await
            .map_err(|e| format!("CoinPaprika 解析失敗: {}", e))?;

        Ok(parse_paprika_ticker(symbol, &data))
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        // CoinPaprika /tickers 一次回傳所有幣的行情
        let arr: Vec<serde_json::Value> = self
            .client
            .get("https://api.coinpaprika.com/v1/tickers")
            .send()
            .await
            .map_err(|e| format!("CoinPaprika 批量連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("CoinPaprika 批量 API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("CoinPaprika 批量解析失敗: {}", e))?;

        // 建立 id → ticker 索引
        let mut id_map: HashMap<String, &serde_json::Value> = HashMap::new();
        // 建立 symbol(大寫) → ticker 索引（同 symbol 取 rank 最低者）
        let mut sym_map: HashMap<String, &serde_json::Value> = HashMap::new();
        for item in &arr {
            if let Some(id) = item["id"].as_str() {
                id_map.insert(id.to_string(), item);
            }
            if let Some(sym) = item["symbol"].as_str() {
                let key = sym.to_uppercase();
                let rank = item["rank"].as_u64().unwrap_or(u64::MAX);
                let replace = match sym_map.get(&key) {
                    Some(existing) => {
                        rank < existing["rank"].as_u64().unwrap_or(u64::MAX)
                    }
                    None => true,
                };
                if replace {
                    sym_map.insert(key, item);
                }
            }
        }

        let mut out = Vec::new();
        for sym in symbols {
            let (base, _) = parse_crypto_symbol(sym);
            // 先嘗試 symbol，再嘗試輸入作為 id
            let ticker = sym_map
                .get(&base)
                .or_else(|| id_map.get(&sym.to_lowercase()));
            if let Some(item) = ticker {
                out.push(parse_paprika_ticker(sym, item));
            } else {
                eprintln!("CoinPaprika 找不到: {}", sym);
            }
        }
        Ok(out)
    }
}

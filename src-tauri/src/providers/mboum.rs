use super::traits::*;
use std::collections::HashMap;

pub struct MboumProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl MboumProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: shared_client(),
            api_key,
        }
    }

    fn parse_quote(symbol: &str, q: &serde_json::Value) -> AssetData {
        let price = q["regularMarketPrice"].as_f64().unwrap_or(0.0);
        let market_state = q["marketState"].as_str();
        let pre_price = q["preMarketPrice"].as_f64();
        let post_price = q["postMarketPrice"].as_f64();

        let mut builder = AssetDataBuilder::new(symbol, "mboum")
            .price(price)
            .currency(q["currency"].as_str().unwrap_or("USD"))
            .change_24h(q["regularMarketChange"].as_f64())
            .change_percent_24h(q["regularMarketChangePercent"].as_f64())
            .high_24h(q["regularMarketDayHigh"].as_f64())
            .low_24h(q["regularMarketDayLow"].as_f64())
            .volume(q["regularMarketVolume"].as_f64())
            .market_cap(q["marketCap"].as_f64())
            .extra_f64("open_price", q["regularMarketOpen"].as_f64())
            .extra_f64("prev_close", q["regularMarketPreviousClose"].as_f64())
            .extra_f64("52w_high", q["fiftyTwoWeekHigh"].as_f64())
            .extra_f64("52w_low", q["fiftyTwoWeekLow"].as_f64())
            .extra_str("name", q["shortName"].as_str())
            .extra_str("market_session", market_state);

        // 盤前數據
        if let Some(pp) = pre_price {
            builder = builder.extra_f64("pre_market_price", Some(pp));
            let pre_change = pp - price;
            let pre_pct = if price > 0.0 {
                (pre_change / price) * 100.0
            } else {
                0.0
            };
            builder = builder.extra_f64("pre_market_change", Some(pre_change));
            builder = builder.extra_f64("pre_market_change_pct", Some(pre_pct));
        }

        // 盤後數據
        if let Some(pp) = post_price {
            builder = builder.extra_f64("post_market_price", Some(pp));
            let post_change = pp - price;
            let post_pct = if price > 0.0 {
                (post_change / price) * 100.0
            } else {
                0.0
            };
            builder = builder.extra_f64("post_market_change", Some(post_change));
            builder = builder.extra_f64("post_market_change_pct", Some(post_pct));
        }

        builder.build()
    }
}

#[async_trait::async_trait]
impl DataProvider for MboumProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("mboum").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("Mboum 需要 API Key")?;

        let data: serde_json::Value = self
            .client
            .get(format!(
                "https://api.mboum.com/v1/markets/stock/quotes?ticker={}",
                symbol
            ))
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| format!("Mboum 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Mboum API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Mboum 解析失敗: {}", e))?;

        let q = &data["body"][0];
        if q.is_null() {
            return Err(format!("Mboum 找不到: {}", symbol));
        }
        Ok(Self::parse_quote(symbol, q))
    }

    /// 批量查詢 — ticker=AAPL,MSFT
    /// 如果批量查詢因為部分 invalid symbol 失敗 (400)，自動降級為受限的個別查詢
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        let api_key = self.api_key.as_ref().ok_or("Mboum 需要 API Key")?;
        let syms_csv = symbols.join(",");
        let url = format!(
            "https://api.mboum.com/v1/markets/stock/quotes?ticker={}",
            syms_csv
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| format!("Mboum 批量連接失敗: {}", e))?;

        let body = resp
            .text()
            .await
            .map_err(|e| format!("Mboum 批量讀取失敗: {}", e))?;

        // 嘗試解析批量回應
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(arr) = data["body"].as_array() {
                let response_map: HashMap<String, &serde_json::Value> = arr
                    .iter()
                    .filter_map(|v| v["symbol"].as_str().map(|s| (s.to_uppercase(), v)))
                    .collect();

                let mut results = Vec::new();
                for sym in symbols {
                    if let Some(q) = response_map.get(&sym.to_uppercase()) {
                        results.push(Self::parse_quote(sym, q));
                    }
                }
                if !results.is_empty() {
                    return Ok(results);
                }
            }
        }

        // 批量失敗或無法獲得任何資料，降級為逐一擷取 (限制並發數 5)
        eprintln!("Mboum 批量查詢失敗，嘗試限流逐一查詢");
        let client = self.client.clone();
        let api_key_clone = api_key.clone();
        let mut tasks = tokio::task::JoinSet::new();
        let mut results = Vec::new();
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(5));

        for sym in symbols.to_vec() {
            let c = client.clone();
            let k = api_key_clone.clone();
            let sem = semaphore.clone();
            tasks.spawn(async move {
                let _permit = sem.acquire().await;
                let url = format!(
                    "https://api.mboum.com/v1/markets/stock/quotes?ticker={}",
                    sym
                );
                match c.get(&url).header("Authorization", format!("Bearer {}", k)).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(data) = resp.json::<serde_json::Value>().await {
                            let q = &data["body"][0];
                            if !q.is_null() {
                                return Some(Self::parse_quote(&sym, q));
                            }
                        }
                        None
                    }
                    _ => None,
                }
            });
        }

        while let Some(Ok(maybe)) = tasks.join_next().await {
            if let Some(data) = maybe {
                results.push(data);
            }
        }

        Ok(results)
    }
}

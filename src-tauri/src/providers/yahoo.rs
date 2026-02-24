use super::traits::*;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Yahoo Finance now requires cookie + crumb authentication.
/// We fetch a cookie from fc.yahoo.com, then get a crumb, and use both for API calls.
pub struct YahooProvider {
    client: reqwest::Client,
    auth: Arc<RwLock<Option<YahooAuth>>>,
}

#[derive(Clone)]
struct YahooAuth {
    #[allow(dead_code)]
    cookie: String,
    crumb: String,
}

impl YahooProvider {
    pub fn new() -> Self {
        // Build client with cookie store enabled
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .cookie_store(true)
            .build()
            .unwrap_or_default();
        Self {
            client,
            auth: Arc::new(RwLock::new(None)),
        }
    }

    async fn get_auth(&self) -> Result<YahooAuth, String> {
        // Check cached auth
        {
            let cached = self.auth.read().await;
            if let Some(auth) = cached.as_ref() {
                return Ok(auth.clone());
            }
        }

        // Step 1: Get cookies from fc.yahoo.com
        let _ = self.client
            .get("https://fc.yahoo.com")
            .send().await
            .map_err(|e| format!("Yahoo cookie 獲取失敗: {}", e))?;

        // Step 2: Get crumb
        let crumb = self.client
            .get("https://query2.finance.yahoo.com/v1/test/getcrumb")
            .send().await
            .map_err(|e| format!("Yahoo crumb 獲取失敗: {}", e))?
            .text().await
            .map_err(|e| format!("Yahoo crumb 解析失敗: {}", e))?;

        if crumb.is_empty() || crumb.contains("<!DOCTYPE") {
            return Err("Yahoo crumb 獲取失敗，請稍後重試".to_string());
        }

        let auth = YahooAuth {
            cookie: String::new(), // cookie_store handles this
            crumb,
        };

        // Cache it
        let mut cached = self.auth.write().await;
        *cached = Some(auth.clone());
        Ok(auth)
    }

    async fn invalidate_auth(&self) {
        let mut cached = self.auth.write().await;
        *cached = None;
    }
}

#[async_trait::async_trait]
impl DataProvider for YahooProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("yahoo").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let auth = self.get_auth().await?;

        // Yahoo uses dash for share classes (BRK-B), convert dot notation (BRK.B)
        let yahoo_symbol = symbol.replace('.', "-");

        let url = format!(
            "https://query2.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1d&crumb={}",
            yahoo_symbol, auth.crumb
        );

        let resp = self.client.get(&url)
            .send().await
            .map_err(|e| format!("Yahoo 連接失敗: {}", e))?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED || resp.status() == reqwest::StatusCode::FORBIDDEN {
            // Invalidate and retry once
            self.invalidate_auth().await;
            let auth2 = self.get_auth().await?;
            let url2 = format!(
                "https://query2.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1d&crumb={}",
                yahoo_symbol, auth2.crumb
            );
            let resp2 = self.client.get(&url2)
                .send().await
                .map_err(|e| format!("Yahoo 重試連接失敗: {}", e))?;
            let data: serde_json::Value = resp2
                .error_for_status().map_err(|e| format!("Yahoo API 錯誤: {}", e))?
                .json().await.map_err(|e| format!("Yahoo 解析失敗: {}", e))?;
            return parse_yahoo_chart(symbol, &data);
        }

        let data: serde_json::Value = resp
            .error_for_status().map_err(|e| format!("Yahoo API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Yahoo 解析失敗: {}", e))?;

        parse_yahoo_chart(symbol, &data)
    }

    /// 批量查詢 — v7/quote 端點已失效，改用 v8/chart 並行查詢
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        let futures: Vec<_> = symbols.iter().map(|s| self.fetch_price(s)).collect();
        let settled = futures::future::join_all(futures).await;

        let mut results = Vec::with_capacity(symbols.len());
        let mut errors = Vec::new();
        for (i, r) in settled.into_iter().enumerate() {
            match r {
                Ok(data) => results.push(data),
                Err(e) => errors.push(format!("{}: {}", symbols[i], e)),
            }
        }
        // 只要有部分成功就回傳，全部失敗才報錯
        if results.is_empty() && !errors.is_empty() {
            // 去重：如果所有 symbol 都是同一個錯誤，只報一次
            let unique_msgs: std::collections::HashSet<String> = errors.iter()
                .map(|e| e.split_once(": ").map(|(_, msg)| msg.to_string()).unwrap_or_else(|| e.clone()))
                .collect();
            if unique_msgs.len() == 1 {
                let msg = unique_msgs.into_iter().next().unwrap();
                return Err(format!("Yahoo 批量查詢全部失敗 ({}個): {}", errors.len(), msg));
            }
            return Err(format!("Yahoo 批量查詢全部失敗: {}", errors.join("; ")));
        }
        Ok(results)
    }
}

fn parse_yahoo_chart(symbol: &str, data: &serde_json::Value) -> Result<AssetData, String> {
    let result = &data["chart"]["result"][0];
    if result.is_null() {
        return Err(format!("Yahoo 找不到: {}。請使用股票代號如 AAPL, GOOGL", symbol));
    }
    let meta = &result["meta"];

    let price = meta["regularMarketPrice"].as_f64().unwrap_or(0.0);
    let prev_close = meta["chartPreviousClose"].as_f64().unwrap_or(price);
    let change = price - prev_close;
    let pct = if prev_close > 0.0 { (change / prev_close) * 100.0 } else { 0.0 };
    let currency = meta["currency"].as_str().unwrap_or("USD");

    Ok(AssetDataBuilder::new(symbol, "yahoo")
        .price(price)
        .currency(currency)
        .change_24h(Some(change))
        .change_percent_24h(Some(pct))
        .high_24h(meta["regularMarketDayHigh"].as_f64())
        .low_24h(meta["regularMarketDayLow"].as_f64())
        .volume(meta["regularMarketVolume"].as_f64())
        .extra_f64("前收盤價", meta["previousClose"].as_f64())
        .extra_f64("52週高", meta["fiftyTwoWeekHigh"].as_f64())
        .extra_f64("52週低", meta["fiftyTwoWeekLow"].as_f64())
        .extra_str("交易所", meta["exchangeName"].as_str())
        .build())
}

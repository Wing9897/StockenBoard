use super::traits::*;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Yahoo Finance — 使用 cookie + crumb 認證
/// 主要端點: v7/finance/quote (支援批量 + 盤前盤後數據)
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

/// v7/quote 需要的欄位列表
const QUOTE_FIELDS: &str = "regularMarketPrice,regularMarketChange,regularMarketChangePercent,\
regularMarketDayHigh,regularMarketDayLow,regularMarketVolume,regularMarketPreviousClose,\
regularMarketOpen,marketCap,currency,exchangeName,marketState,shortName,\
fiftyTwoWeekHigh,fiftyTwoWeekLow,\
preMarketPrice,preMarketChange,preMarketChangePercent,\
postMarketPrice,postMarketChange,postMarketChangePercent";

impl YahooProvider {
    pub fn new() -> Self {
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
        {
            let cached = self.auth.read().await;
            if let Some(auth) = cached.as_ref() {
                return Ok(auth.clone());
            }
        }

        let _ = self
            .client
            .get("https://fc.yahoo.com")
            .send()
            .await
            .map_err(|e| format!("Yahoo cookie 獲取失敗: {}", e))?;

        let crumb = self
            .client
            .get("https://query2.finance.yahoo.com/v1/test/getcrumb")
            .send()
            .await
            .map_err(|e| format!("Yahoo crumb 獲取失敗: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Yahoo crumb 解析失敗: {}", e))?;

        if crumb.is_empty() || crumb.contains("<!DOCTYPE") {
            return Err("Yahoo crumb 獲取失敗，請稍後重試".to_string());
        }

        let auth = YahooAuth {
            cookie: String::new(),
            crumb,
        };
        let mut cached = self.auth.write().await;
        *cached = Some(auth.clone());
        Ok(auth)
    }

    async fn invalidate_auth(&self) {
        let mut cached = self.auth.write().await;
        *cached = None;
    }

    /// 呼叫 v7/finance/quote 端點，支援多個 symbol
    async fn fetch_v7_quote(&self, symbols_csv: &str) -> Result<serde_json::Value, String> {
        let auth = self.get_auth().await?;
        let url = format!(
            "https://query2.finance.yahoo.com/v7/finance/quote?symbols={}&fields={}&crumb={}",
            symbols_csv, QUOTE_FIELDS, auth.crumb
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Yahoo 連接失敗: {}", e))?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED
            || resp.status() == reqwest::StatusCode::FORBIDDEN
        {
            self.invalidate_auth().await;
            let auth2 = self.get_auth().await?;
            let url2 = format!(
                "https://query2.finance.yahoo.com/v7/finance/quote?symbols={}&fields={}&crumb={}",
                symbols_csv, QUOTE_FIELDS, auth2.crumb
            );
            let resp2 = self
                .client
                .get(&url2)
                .send()
                .await
                .map_err(|e| format!("Yahoo 重試連接失敗: {}", e))?;
            return resp2
                .error_for_status()
                .map_err(|e| format!("Yahoo API 錯誤: {}", e))?
                .json()
                .await
                .map_err(|e| format!("Yahoo 解析失敗: {}", e));
        }

        resp.error_for_status()
            .map_err(|e| format!("Yahoo API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Yahoo 解析失敗: {}", e))
    }
}

#[async_trait::async_trait]
impl DataProvider for YahooProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("yahoo").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let yahoo_symbol = symbol.replace('.', "-");
        let data = self.fetch_v7_quote(&yahoo_symbol).await?;
        let q = &data["quoteResponse"]["result"][0];
        if q.is_null() {
            return Err(format!(
                "Yahoo 找不到: {}。請使用股票代號如 AAPL, GOOGL",
                symbol
            ));
        }
        Ok(parse_v7_quote(symbol, q))
    }

    /// 批量查詢 — v7/quote 原生支援多 symbol
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        let yahoo_symbols: Vec<String> = symbols.iter().map(|s| s.replace('.', "-")).collect();
        let csv = yahoo_symbols.join(",");
        let data = self.fetch_v7_quote(&csv).await?;

        let arr = data["quoteResponse"]["result"]
            .as_array()
            .ok_or("Yahoo 批量回應格式錯誤")?;

        if arr.is_empty() {
            return Err("Yahoo 批量查詢全部失敗: 找不到任何結果".to_string());
        }

        let mut results = Vec::with_capacity(arr.len());
        let sym_map: std::collections::HashMap<String, &str> = symbols
            .iter()
            .map(|s| (s.replace('.', "-").to_uppercase(), s.as_str()))
            .collect();

        for q in arr {
            let yahoo_sym = q["symbol"].as_str().unwrap_or("").to_uppercase();
            let original_sym = sym_map.get(&yahoo_sym).copied().unwrap_or(&yahoo_sym);
            results.push(parse_v7_quote(original_sym, q));
        }
        Ok(results)
    }
}

fn parse_v7_quote(symbol: &str, q: &serde_json::Value) -> AssetData {
    let price = q["regularMarketPrice"].as_f64().unwrap_or(0.0);
    let currency = q["currency"].as_str().unwrap_or("USD");
    let market_state = q["marketState"].as_str();

    let pre_price = q["preMarketPrice"].as_f64();
    let pre_pct = q["preMarketChangePercent"].as_f64();
    let post_price = q["postMarketPrice"].as_f64();
    let post_pct = q["postMarketChangePercent"].as_f64();

    let mut builder = AssetDataBuilder::new(symbol, "yahoo")
        .price(price)
        .currency(currency)
        .change_24h(q["regularMarketChange"].as_f64())
        .change_percent_24h(q["regularMarketChangePercent"].as_f64())
        .high_24h(q["regularMarketDayHigh"].as_f64())
        .low_24h(q["regularMarketDayLow"].as_f64())
        .volume(q["regularMarketVolume"].as_f64())
        .market_cap(q["marketCap"].as_f64())
        .extra_f64("prev_close", q["regularMarketPreviousClose"].as_f64())
        .extra_f64("open_price", q["regularMarketOpen"].as_f64())
        .extra_f64("52w_high", q["fiftyTwoWeekHigh"].as_f64())
        .extra_f64("52w_low", q["fiftyTwoWeekLow"].as_f64())
        .extra_str("exchange", q["exchangeName"].as_str())
        .extra_str("name", q["shortName"].as_str())
        .extra_str("market_session", market_state);

    // 盤前數據
    if let Some(pp) = pre_price {
        builder = builder.extra_f64("pre_market_price", Some(pp));
        let pre_change = pp - price;
        builder = builder.extra_f64("pre_market_change", Some(pre_change));
        builder = builder.extra_f64("pre_market_change_pct", pre_pct);
    }

    // 盤後數據
    if let Some(pp) = post_price {
        builder = builder.extra_f64("post_market_price", Some(pp));
        let post_change = pp - price;
        builder = builder.extra_f64("post_market_change", Some(post_change));
        builder = builder.extra_f64("post_market_change_pct", post_pct);
    }

    builder.build()
}

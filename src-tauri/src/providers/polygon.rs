use super::traits::*;
use std::collections::HashMap;

pub struct PolygonProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl PolygonProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { client: shared_client(), api_key }
    }

    fn to_polygon_symbol(symbol: &str) -> String {
        if symbol.starts_with("X:") || symbol.starts_with("O:") || symbol.starts_with("C:") {
            return symbol.to_string();
        }
        let s = symbol.to_uppercase();
        let looks_crypto = s.ends_with("USDT") || s.ends_with("USD")
            || s.contains('-') || s.contains('/');
        if looks_crypto {
            let (base, quote) = parse_crypto_symbol(symbol);
            let q = if quote == "USDT" { "USD" } else { &quote };
            format!("X:{}{}", base, q)
        } else {
            symbol.to_string()
        }
    }

    fn parse_agg(symbol: &str, r: &serde_json::Value) -> AssetData {
        let price = r["c"].as_f64().unwrap_or(0.0);
        let open = r["o"].as_f64().unwrap_or(price);
        let change = price - open;
        let pct = if open > 0.0 { (change / open) * 100.0 } else { 0.0 };

        AssetDataBuilder::new(symbol, "polygon")
            .price(price)
            .change_24h(Some(change))
            .change_percent_24h(Some(pct))
            .high_24h(r["h"].as_f64())
            .low_24h(r["l"].as_f64())
            .volume(r["v"].as_f64())
            .extra_f64("open_price", r["o"].as_f64())
            .extra_f64("weighted_avg_price", r["vw"].as_f64())
            .extra_i64("trade_count", r["n"].as_i64())
            .build()
    }
}

#[async_trait::async_trait]
impl DataProvider for PolygonProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("polygon").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("Polygon.io 需要 API Key")?;
        let api_symbol = Self::to_polygon_symbol(symbol);

        let data: serde_json::Value = self.client
            .get(format!("https://api.polygon.io/v2/aggs/ticker/{}/prev?apiKey={}", api_symbol, api_key))
            .send().await.map_err(|e| format!("Polygon 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Polygon API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Polygon 解析失敗: {}", e))?;

        let r = &data["results"][0];
        if r.is_null() {
            return Err(format!("Polygon 找不到: {}。股票用 AAPL，加密用 X:BTCUSD", symbol));
        }
        Ok(Self::parse_agg(symbol, r))
    }

    /// 批量查詢 — 用並行 request 避免逐一串行被 rate limit
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        let api_key = self.api_key.as_ref().ok_or("Polygon.io 需要 API Key")?;

        // 分成 stock 和 crypto
        let mut stock_syms: Vec<(String, String)> = Vec::new(); // (original, polygon_sym)
        let mut crypto_syms: Vec<(String, String)> = Vec::new();
        for s in symbols {
            let ps = Self::to_polygon_symbol(s);
            if ps.starts_with("X:") {
                crypto_syms.push((s.clone(), ps));
            } else {
                stock_syms.push((s.clone(), ps));
            }
        }

        let mut results = Vec::new();

        // Stock: 用 snapshot endpoint 一次查所有
        if !stock_syms.is_empty() {
            let tickers: Vec<&str> = stock_syms.iter().map(|(_, p)| p.as_str()).collect();
            let tickers_param = tickers.join(",");
            let url = format!(
                "https://api.polygon.io/v2/snapshot/locale/us/markets/stocks/tickers?tickers={}&apiKey={}",
                tickers_param, api_key
            );
            match self.client.get(&url).send().await {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(arr) = data["tickers"].as_array() {
                            let snap_map: HashMap<String, &serde_json::Value> = arr.iter()
                                .filter_map(|t| t["ticker"].as_str().map(|s| (s.to_uppercase(), t)))
                                .collect();
                            for (original, ps) in &stock_syms {
                                if let Some(snap) = snap_map.get(&ps.to_uppercase()) {
                                    let day = &snap["day"];
                                    if !day.is_null() {
                                        results.push(Self::parse_agg(original, day));
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Polygon stock snapshot 失敗: {}", e),
            }
        }

        // Crypto: 限流並行查詢（Polygon 沒有 crypto snapshot batch endpoint）
        if !crypto_syms.is_empty() {
            use futures::stream::{self, StreamExt};
            let api_key_owned = api_key.clone();
            let client = self.client.clone();
            let crypto_results: Vec<_> = stream::iter(crypto_syms)
                .map(|(original, ps)| {
                    let url = format!("https://api.polygon.io/v2/aggs/ticker/{}/prev?apiKey={}", ps, api_key_owned);
                    let c = client.clone();
                    async move {
                        match c.get(&url).send().await {
                            Ok(resp) => match resp.json::<serde_json::Value>().await {
                                Ok(data) => {
                                    let r = &data["results"][0];
                                    if !r.is_null() { Some(Self::parse_agg(&original, r)) } else { None }
                                }
                                Err(_) => None,
                            }
                            Err(_) => None,
                        }
                    }
                })
                .buffer_unordered(3)
                .collect()
                .await;
            results.extend(crypto_results.into_iter().flatten());
        }

        Ok(results)
    }
}

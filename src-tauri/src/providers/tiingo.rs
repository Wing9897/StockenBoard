use super::traits::*;

pub struct TiingoProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl TiingoProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: shared_client(),
            api_key,
        }
    }

    fn is_crypto(symbol: &str) -> bool {
        let s = symbol.to_uppercase();
        s.contains("USD")
            || s.contains("BTC")
            || s.contains("ETH")
            || s.contains('-')
            || s.contains('/')
    }

    fn to_tiingo_crypto(symbol: &str) -> String {
        let (base, quote) = parse_crypto_symbol(symbol);
        let q = if quote == "USDT" { "USD" } else { &quote };
        format!("{}{}", base.to_lowercase(), q.to_lowercase())
    }

    fn parse_stock(symbol: &str, item: &serde_json::Value) -> Result<AssetData, String> {
        if item.is_null() {
            return Err(format!("Tiingo 找不到: {}", symbol));
        }
        let price = item["last"].as_f64().unwrap_or(0.0);
        let prev = item["prevClose"].as_f64().unwrap_or(price);
        let change = price - prev;
        let pct = if prev > 0.0 {
            (change / prev) * 100.0
        } else {
            0.0
        };

        Ok(AssetDataBuilder::new(symbol, "tiingo")
            .price(price)
            .change_24h(Some(change))
            .change_percent_24h(Some(pct))
            .high_24h(item["high"].as_f64())
            .low_24h(item["low"].as_f64())
            .volume(item["volume"].as_f64())
            .extra_f64("open_price", item["open"].as_f64())
            .extra_f64("prev_close", item["prevClose"].as_f64())
            .build())
    }
}

#[async_trait::async_trait]
impl DataProvider for TiingoProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("tiingo").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("Tiingo 需要 API Key")?;

        if Self::is_crypto(symbol) {
            let tiingo_sym = Self::to_tiingo_crypto(symbol);
            let url = format!(
                "https://api.tiingo.com/tiingo/crypto/top?tickers={}&token={}",
                tiingo_sym, api_key
            );
            let data: serde_json::Value = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Tiingo 連接失敗: {}", e))?
                .error_for_status()
                .map_err(|e| format!("Tiingo API 錯誤: {}", e))?
                .json()
                .await
                .map_err(|e| format!("Tiingo 解析失敗: {}", e))?;

            let top = &data[0]["topOfBookData"][0];
            if top.is_null() {
                return Err(format!("Tiingo 找不到加密貨幣: {}", symbol));
            }
            Ok(AssetDataBuilder::new(symbol, "tiingo")
                .price(top["lastPrice"].as_f64().unwrap_or(0.0))
                .build())
        } else {
            let url = format!("https://api.tiingo.com/iex/{}?token={}", symbol, api_key);
            let data: serde_json::Value = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Tiingo 連接失敗: {}", e))?
                .error_for_status()
                .map_err(|e| format!("Tiingo API 錯誤: {}", e))?
                .json()
                .await
                .map_err(|e| format!("Tiingo 解析失敗: {}", e))?;

            Self::parse_stock(symbol, &data[0])
        }
    }

    /// 批量查詢 — tickers=aapl,msft 或 tickers=btcusd,ethusd
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        let api_key = self.api_key.as_ref().ok_or("Tiingo 需要 API Key")?;

        // 分成 crypto 和 stock 兩組
        let mut crypto_syms: Vec<(String, String)> = Vec::new(); // (original, tiingo_sym)
        let mut stock_syms: Vec<String> = Vec::new();

        for s in symbols {
            if Self::is_crypto(s) {
                crypto_syms.push((s.clone(), Self::to_tiingo_crypto(s)));
            } else {
                stock_syms.push(s.clone());
            }
        }

        let mut results = Vec::new();

        // 批量查 crypto — 限流並行查詢
        if !crypto_syms.is_empty() {
            use futures::stream::{self, StreamExt};
            let api_key_owned = api_key.clone();
            let client = self.client.clone();
            let crypto_results: Vec<_> = stream::iter(crypto_syms)
                .map(|(original, tiingo_sym)| {
                    let c = client.clone();
                    let key = api_key_owned.clone();
                    async move {
                        let url = format!(
                            "https://api.tiingo.com/tiingo/crypto/top?tickers={}&token={}",
                            tiingo_sym, key
                        );
                        match c.get(&url).send().await {
                            Ok(resp) => match resp.json::<serde_json::Value>().await {
                                Ok(data) => {
                                    let top = &data[0]["topOfBookData"][0];
                                    if top.is_null() {
                                        return None;
                                    }
                                    Some(
                                        AssetDataBuilder::new(&original, "tiingo")
                                            .price(top["lastPrice"].as_f64().unwrap_or(0.0))
                                            .build(),
                                    )
                                }
                                Err(e) => {
                                    eprintln!("Tiingo crypto 跳過 {}: {}", original, e);
                                    None
                                }
                            },
                            Err(e) => {
                                eprintln!("Tiingo crypto 跳過 {}: {}", original, e);
                                None
                            }
                        }
                    }
                })
                .buffer_unordered(2)
                .collect()
                .await;
            results.extend(crypto_results.into_iter().flatten());
        }

        // 批量查 stock — Tiingo IEX 支持 tickers=aapl,msft
        if !stock_syms.is_empty() {
            let tickers = stock_syms.join(",");
            let url = format!(
                "https://api.tiingo.com/iex/?tickers={}&token={}",
                tickers, api_key
            );
            match self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Tiingo stock 批量失敗: {}", e))
            {
                Ok(resp) => {
                    if let Ok(arr) = resp
                        .json::<Vec<serde_json::Value>>()
                        .await
                        .map_err(|e| e.to_string())
                    {
                        let mut ticker_map: std::collections::HashMap<String, &serde_json::Value> =
                            std::collections::HashMap::new();
                        for item in &arr {
                            if let Some(t) = item["ticker"].as_str() {
                                ticker_map.insert(t.to_uppercase(), item);
                            }
                        }
                        for sym in &stock_syms {
                            if let Some(item) = ticker_map.get(&sym.to_uppercase()) {
                                match Self::parse_stock(sym, item) {
                                    Ok(asset) => results.push(asset),
                                    Err(e) => eprintln!("Tiingo stock 跳過 {}: {}", sym, e),
                                }
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Tiingo stock 批量失敗: {}", e),
            }
        }

        Ok(results)
    }
}

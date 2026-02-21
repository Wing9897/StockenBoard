use super::traits::*;
use std::collections::HashMap;

pub struct AlpacaProvider {
    client: reqwest::Client,
    api_key: Option<String>,
    api_secret: Option<String>,
}

impl AlpacaProvider {
    pub fn new(api_key: Option<String>, api_secret: Option<String>) -> Self {
        Self { client: shared_client(), api_key, api_secret }
    }

    fn is_crypto(symbol: &str) -> bool {
        symbol.contains('/')
            || symbol.contains('-')
            || symbol.to_uppercase().ends_with("USDT")
            || symbol.to_uppercase().ends_with("USD")
    }

    fn to_alpaca_crypto(symbol: &str) -> String {
        let (base, quote) = parse_crypto_symbol(symbol);
        let q = if quote == "USDT" { "USD" } else { &quote };
        format!("{}/{}", base, q)
    }

    fn parse_bar(symbol: &str, bar: &serde_json::Value) -> AssetData {
        let price = bar["c"].as_f64().unwrap_or(0.0);
        let open = bar["o"].as_f64().unwrap_or(price);
        let change = price - open;
        let pct = if open > 0.0 { (change / open) * 100.0 } else { 0.0 };

        AssetDataBuilder::new(symbol, "alpaca")
            .price(price)
            .change_24h(Some(change))
            .change_percent_24h(Some(pct))
            .high_24h(bar["h"].as_f64())
            .low_24h(bar["l"].as_f64())
            .volume(bar["v"].as_f64())
            .extra_f64("開盤價", bar["o"].as_f64())
            .extra_f64("加權平均價", bar["vw"].as_f64())
            .extra_i64("交易次數", bar["n"].as_i64())
            .build()
    }
}

#[async_trait::async_trait]
impl DataProvider for AlpacaProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("alpaca").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("Alpaca 需要 API Key")?;
        let api_secret = self.api_secret.as_ref().ok_or("Alpaca 需要 API Secret")?;

        let is_crypto = Self::is_crypto(symbol);
        let api_symbol = if is_crypto { Self::to_alpaca_crypto(symbol) } else { symbol.to_string() };

        let url = if is_crypto {
            format!("https://data.alpaca.markets/v1beta3/crypto/us/latest/bars?symbols={}", api_symbol)
        } else {
            format!("https://data.alpaca.markets/v2/stocks/{}/bars/latest", api_symbol)
        };

        let data: serde_json::Value = self.client.get(&url)
            .header("APCA-API-KEY-ID", api_key)
            .header("APCA-API-SECRET-KEY", api_secret)
            .send().await.map_err(|e| format!("Alpaca 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Alpaca API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Alpaca 解析失敗: {}", e))?;

        let bar = if is_crypto { &data["bars"][&api_symbol] } else { &data["bar"] };
        if bar.is_null() {
            return Err(format!("Alpaca 找不到: {}。股票用 AAPL，加密用 BTC/USD", symbol));
        }
        Ok(Self::parse_bar(symbol, bar))
    }

    /// 批量查詢 — symbols=AAPL,MSFT 或 symbols=BTC/USD,ETH/USD
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        let api_key = self.api_key.as_ref().ok_or("Alpaca 需要 API Key")?;
        let api_secret = self.api_secret.as_ref().ok_or("Alpaca 需要 API Secret")?;

        // 分成 crypto 和 stock
        let mut crypto_map: Vec<(String, String)> = Vec::new(); // (original, alpaca_sym)
        let mut stock_syms: Vec<String> = Vec::new();

        for s in symbols {
            if Self::is_crypto(s) {
                crypto_map.push((s.clone(), Self::to_alpaca_crypto(s)));
            } else {
                stock_syms.push(s.clone());
            }
        }

        let mut results = Vec::new();

        // 批量查 crypto
        if !crypto_map.is_empty() {
            let alpaca_syms: Vec<&str> = crypto_map.iter().map(|(_, a)| a.as_str()).collect();
            let url = format!("https://data.alpaca.markets/v1beta3/crypto/us/latest/bars?symbols={}",
                alpaca_syms.join(","));
            if let Ok(resp) = self.client.get(&url)
                .header("APCA-API-KEY-ID", api_key)
                .header("APCA-API-SECRET-KEY", api_secret)
                .send().await
            {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    let bars = &data["bars"];
                    for (original, alpaca_sym) in &crypto_map {
                        let bar = &bars[alpaca_sym];
                        if !bar.is_null() {
                            results.push(Self::parse_bar(original, bar));
                        }
                    }
                }
            }
        }

        // 批量查 stock
        if !stock_syms.is_empty() {
            let syms = stock_syms.join(",");
            let url = format!("https://data.alpaca.markets/v2/stocks/bars/latest?symbols={}", syms);
            if let Ok(resp) = self.client.get(&url)
                .header("APCA-API-KEY-ID", api_key)
                .header("APCA-API-SECRET-KEY", api_secret)
                .send().await
            {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    let bars = &data["bars"];
                    if let Some(obj) = bars.as_object() {
                        let bar_map: HashMap<String, &serde_json::Value> = obj.iter()
                            .map(|(k, v)| (k.to_uppercase(), v))
                            .collect();
                        for sym in &stock_syms {
                            if let Some(bar) = bar_map.get(&sym.to_uppercase()) {
                                results.push(Self::parse_bar(sym, bar));
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

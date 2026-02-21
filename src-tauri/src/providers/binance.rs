use super::traits::*;
use std::collections::HashMap;

pub struct BinanceProvider {
    client: reqwest::Client,
}

impl BinanceProvider {
    pub fn new(_api_key: Option<String>) -> Self {
        Self { client: shared_client() }
    }

    fn parse_ticker(symbol: &str, data: &serde_json::Value) -> AssetData {
        let parse_f64 = |key: &str| data[key].as_str().and_then(|s| s.parse::<f64>().ok());
        AssetDataBuilder::new(symbol, "binance")
            .price(parse_f64("lastPrice").unwrap_or(0.0))
            .currency("USDT")
            .change_24h(parse_f64("priceChange"))
            .change_percent_24h(parse_f64("priceChangePercent"))
            .high_24h(parse_f64("highPrice"))
            .low_24h(parse_f64("lowPrice"))
            .volume(parse_f64("volume"))
            .extra_f64("加權平均價", parse_f64("weightedAvgPrice"))
            .extra_f64("開盤價", parse_f64("openPrice"))
            .extra_i64("交易次數", data["count"].as_i64())
            .extra_f64("報價成交量", parse_f64("quoteVolume"))
            .build()
    }
}

#[async_trait::async_trait]
impl DataProvider for BinanceProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("binance").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let sym = to_binance_symbol(symbol);
        let url = format!("https://api.binance.com/api/v3/ticker/24hr?symbol={}", sym);
        let data: serde_json::Value = self.client.get(&url)
            .send().await.map_err(|e| format!("Binance 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Binance API 錯誤: {}。格式: BTCUSDT", e))?
            .json().await.map_err(|e| format!("Binance 解析失敗: {}", e))?;

        Ok(Self::parse_ticker(symbol, &data))
    }

    /// 批量查詢 — symbols=["BTCUSDT","ETHUSDT"] 一次查多個
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        // 建立 binance_symbol -> original_symbol 映射
        let mappings: Vec<(String, String)> = symbols.iter()
            .map(|s| (s.clone(), to_binance_symbol(s)))
            .collect();

        let binance_syms: Vec<String> = mappings.iter()
            .map(|(_, bs)| format!("\"{}\"", bs))
            .collect();
        let syms_param = format!("[{}]", binance_syms.join(","));

        let url = format!("https://api.binance.com/api/v3/ticker/24hr?symbols={}", syms_param);
        let arr: Vec<serde_json::Value> = self.client.get(&url)
            .send().await.map_err(|e| format!("Binance 批量連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Binance 批量 API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Binance 批量解析失敗: {}", e))?;

        // 建立 binance_symbol -> response data 的查找表
        let response_map: HashMap<String, &serde_json::Value> = arr.iter()
            .filter_map(|v| v["symbol"].as_str().map(|s| (s.to_string(), v)))
            .collect();

        let mut results = Vec::new();
        for (original, binance_sym) in &mappings {
            if let Some(data) = response_map.get(binance_sym) {
                results.push(Self::parse_ticker(original, data));
            } else {
                eprintln!("Binance 批量查詢跳過 {} ({})", original, binance_sym);
            }
        }
        Ok(results)
    }
}

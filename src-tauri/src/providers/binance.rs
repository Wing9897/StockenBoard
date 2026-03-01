use super::traits::*;
use std::collections::HashMap;

pub struct BinanceProvider {
    client: reqwest::Client,
}

impl BinanceProvider {
    pub fn new(_api_key: Option<String>) -> Self {
        Self {
            client: shared_client(),
        }
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
            .extra_f64("weighted_avg_price", parse_f64("weightedAvgPrice"))
            .extra_f64("open_price", parse_f64("openPrice"))
            .extra_i64("trade_count", data["count"].as_i64())
            .extra_f64("quote_volume", parse_f64("quoteVolume"))
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
        let data: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Binance 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Binance API 錯誤: {}。格式: BTCUSDT", e))?
            .json()
            .await
            .map_err(|e| format!("Binance 解析失敗: {}", e))?;

        Ok(Self::parse_ticker(symbol, &data))
    }

    /// 批量查詢 — 智慧策略：
    /// - ≤5 個 symbol：用 symbols=[...] 精確查詢
    /// - >5 個 symbol：不帶 symbols 參數取回所有 ticker，在本地過濾（免疫無效 symbol）
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        // 建立 binance_symbol -> original_symbol 映射
        let mappings: Vec<(String, String)> = symbols
            .iter()
            .map(|s| (s.clone(), to_binance_symbol(s)))
            .collect();

        if symbols.len() <= 5 {
            // 少量 symbol → 用精確批量查詢（不會包含太多無效 symbol）
            let binance_syms: Vec<String> = mappings
                .iter()
                .map(|(_, bs)| format!("\"{}\"", bs))
                .collect();
            let syms_param = format!("[{}]", binance_syms.join(","));
            let url = format!(
                "https://api.binance.com/api/v3/ticker/24hr?symbols={}",
                syms_param
            );
            let resp = self.client.get(&url).send().await
                .map_err(|e| format!("Binance 批量連接失敗: {}", e))?;
            let body = resp.text().await
                .map_err(|e| format!("Binance 批量讀取失敗: {}", e))?;

            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&body) {
                let response_map: HashMap<String, &serde_json::Value> = arr
                    .iter()
                    .filter_map(|v| v["symbol"].as_str().map(|s| (s.to_string(), v)))
                    .collect();
                let mut results = Vec::new();
                for (original, binance_sym) in &mappings {
                    if let Some(data) = response_map.get(binance_sym) {
                        results.push(Self::parse_ticker(original, data));
                    }
                }
                return Ok(results);
            }
            // 精確查詢失敗，降級到下方全量查詢
        }

        // 大量 symbol 或精確查詢失敗 → 取回所有 ticker，在本地過濾（免疫無效 symbol）
        let url = "https://api.binance.com/api/v3/ticker/24hr";
        let resp = self.client.get(url).send().await.map_err(|e| {
            eprintln!("Binance 請求失敗: {:?}", e);
            format!("Binance 全量查詢連接失敗: {}", e)
        })?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| {
            eprintln!("Binance 讀取 body 失敗: {:?}", e);
            format!("Binance 全量查詢讀取失敗: {}", e)
        })?;

        if !status.is_success() {
            eprintln!("Binance 全量查詢遭拒絕 ({}): {}", status, &body[..body.len().min(200)]);
            return Err(format!("Binance API 拒絕請求 (IP 可能受到限制): {}", status));
        }

        let arr: Vec<serde_json::Value> = serde_json::from_str(&body).map_err(|e| {
            eprintln!("Binance 序列化失敗: {:?}", e);
            format!("Binance 全量查詢解析失敗: {}", e)
        })?;

        let response_map: HashMap<String, &serde_json::Value> = arr
            .iter()
            .filter_map(|v| v["symbol"].as_str().map(|s| (s.to_string(), v)))
            .collect();

        let mut results = Vec::new();
        for (original, binance_sym) in &mappings {
            if let Some(data) = response_map.get(binance_sym) {
                results.push(Self::parse_ticker(original, data));
            }
        }
        Ok(results)
    }
}

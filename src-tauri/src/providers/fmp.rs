use super::traits::*;
use std::collections::HashMap;

pub struct FMPProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl FMPProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: shared_client(),
            api_key,
        }
    }

    fn to_fmp_symbol(symbol: &str) -> String {
        let s = symbol.to_uppercase();
        let looks_crypto =
            s.ends_with("USDT") || s.ends_with("USD") || s.contains('-') || s.contains('/');
        if looks_crypto {
            let (base, quote) = parse_crypto_symbol(symbol);
            let q = if quote == "USDT" { "USD" } else { &quote };
            format!("{}{}", base, q)
        } else {
            symbol.to_string()
        }
    }

    fn parse_quote(symbol: &str, q: &serde_json::Value) -> AssetData {
        let price = q["price"].as_f64().unwrap_or(0.0);

        let mut builder = AssetDataBuilder::new(symbol, "fmp")
            .price(price)
            .change_24h(q["change"].as_f64())
            .change_percent_24h(q["changesPercentage"].as_f64())
            .high_24h(q["dayHigh"].as_f64())
            .low_24h(q["dayLow"].as_f64())
            .volume(q["volume"].as_f64())
            .market_cap(q["marketCap"].as_f64())
            .extra_f64("open_price", q["open"].as_f64())
            .extra_f64("prev_close", q["previousClose"].as_f64())
            .extra_f64("52w_high", q["yearHigh"].as_f64())
            .extra_f64("52w_low", q["yearLow"].as_f64())
            .extra_f64("pe_ratio", q["pe"].as_f64())
            .extra_f64("eps", q["eps"].as_f64())
            .extra_str("name", q["name"].as_str())
            .extra_str("exchange", q["exchange"].as_str());

        // 市場狀態 — FMP 沒有 marketState 欄位，根據有無盤前盤後數據推斷
        let pre_price = q["preMarketPrice"].as_f64().filter(|&v| v > 0.0);
        let post_price = q["afterHoursPrice"].as_f64().filter(|&v| v > 0.0);

        if pre_price.is_some() || post_price.is_some() {
            // 有盤前或盤後數據 → 非交易時段
            if pre_price.is_some() {
                builder = builder.extra_str("market_session", Some("PRE"));
            } else {
                builder = builder.extra_str("market_session", Some("POST"));
            }
        } else {
            builder = builder.extra_str("market_session", Some("REGULAR"));
        }

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
            builder = builder.extra_f64(
                "pre_market_change_pct",
                q["preMarketChangePercent"].as_f64().or(Some(pre_pct)),
            );
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
            builder = builder.extra_f64(
                "post_market_change_pct",
                q["afterHoursChangePercent"].as_f64().or(Some(post_pct)),
            );
        }

        builder.build()
    }
}

#[async_trait::async_trait]
impl DataProvider for FMPProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("fmp").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_ref().ok_or("FMP 需要 API Key")?;
        let api_symbol = Self::to_fmp_symbol(symbol);

        let data: serde_json::Value = self
            .client
            .get(format!(
                "https://financialmodelingprep.com/api/v3/quote/{}?apikey={}",
                api_symbol, api_key
            ))
            .send()
            .await
            .map_err(|e| format!("FMP 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("FMP API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("FMP 解析失敗: {}", e))?;

        let q = &data[0];
        if q.is_null() {
            return Err(format!("FMP 找不到: {}", symbol));
        }
        Ok(Self::parse_quote(symbol, q))
    }

    /// 批量查詢 — /quote/AAPL,MSFT,BTCUSD
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        let api_key = self.api_key.as_ref().ok_or("FMP 需要 API Key")?;
        let mappings: Vec<(String, String)> = symbols
            .iter()
            .map(|s| (s.clone(), Self::to_fmp_symbol(s)))
            .collect();
        let fmp_syms: Vec<&str> = mappings.iter().map(|(_, f)| f.as_str()).collect();
        let syms_str = fmp_syms.join(",");

        let arr: Vec<serde_json::Value> = self
            .client
            .get(format!(
                "https://financialmodelingprep.com/api/v3/quote/{}?apikey={}",
                syms_str, api_key
            ))
            .send()
            .await
            .map_err(|e| format!("FMP 批量連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("FMP API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("FMP 批量解析失敗: {}", e))?;

        // 建立 fmp_symbol -> response 查找表
        let response_map: HashMap<String, &serde_json::Value> = arr
            .iter()
            .filter_map(|v| v["symbol"].as_str().map(|s| (s.to_uppercase(), v)))
            .collect();

        let mut results = Vec::new();
        for (original, fmp_sym) in &mappings {
            if let Some(q) = response_map.get(&fmp_sym.to_uppercase()) {
                results.push(Self::parse_quote(original, q));
            }
        }
        Ok(results)
    }
}

use super::traits::*;
use super::types::*;

pub struct BitfinexProvider {
    client: reqwest::Client,
}

impl Default for BitfinexProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl BitfinexProvider {
    pub fn new() -> Self {
        Self {
            client: shared_client(),
        }
    }
}

/// Convert to Bitfinex format: tBTCUSD
fn to_bitfinex_symbol(symbol: &str) -> String {
    let (base, quote) = parse_crypto_symbol(symbol);
    let q = match quote.as_str() {
        "USDT" => "USD",
        "USDC" => "UDC",
        _ => &quote,
    };
    format!("t{}{}", base, q)
}

// Bitfinex v2 ticker response is an array:
// [BID, BID_SIZE, ASK, ASK_SIZE, DAILY_CHANGE, DAILY_CHANGE_RELATIVE, LAST_PRICE, VOLUME, HIGH, LOW]
fn parse_bitfinex_arr(symbol: &str, arr: &[serde_json::Value]) -> AssetData {
    let f = |i: usize| arr.get(i).and_then(|v| v.as_f64());
    AssetDataBuilder::new(symbol, "bitfinex")
        .price(f(6).unwrap_or(0.0))
        .currency("USD")
        .change_24h(f(4))
        .change_percent_24h(f(5).map(|r| r * 100.0))
        .high_24h(f(8))
        .low_24h(f(9))
        .volume(f(7))
        .build()
}

#[async_trait::async_trait]
impl DataProvider for BitfinexProvider {
    fn info(&self) -> ProviderInfo {
        provider_info_or_panic("bitfinex")
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let bfx = to_bitfinex_symbol(symbol);
        let url = format!("https://api-pub.bitfinex.com/v2/ticker/{}", bfx);
        let arr: Vec<serde_json::Value> = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Bitfinex connection failed: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Bitfinex API error: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Bitfinex parse failed: {}", e))?;

        if arr.len() < 10 {
            return Err("Bitfinex: invalid response format".into());
        }
        Ok(parse_bitfinex_arr(symbol, &arr))
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        let bfx_syms: Vec<String> = symbols.iter().map(|s| to_bitfinex_symbol(s)).collect();
        let param = bfx_syms.join(",");
        let url = format!("https://api-pub.bitfinex.com/v2/tickers?symbols={}", param);
        let data: Vec<Vec<serde_json::Value>> = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Bitfinex batch connection failed: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Bitfinex batch parse failed: {}", e))?;

        let mut map = std::collections::HashMap::new();
        for row in &data {
            if let Some(sym_val) = row.first().and_then(|v| v.as_str()) {
                map.insert(sym_val.to_string(), row);
            }
        }

        let mut out = Vec::new();
        for (i, sym) in symbols.iter().enumerate() {
            if let Some(row) = map.get(&bfx_syms[i]) {
                // tickers response includes symbol as first element, data starts at index 1
                if row.len() >= 11 {
                    out.push(parse_bitfinex_arr(sym, &row[1..]));
                }
            }
        }
        Ok(out)
    }
}

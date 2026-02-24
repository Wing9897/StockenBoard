use crate::providers::traits::{shared_client, AssetData, AssetDataBuilder, DataProvider, DexPoolInfo, DexPoolLookup, ProviderInfo};
use crate::providers::traits::PROVIDER_INFO_MAP;
use serde::Deserialize;

pub struct RaydiumProvider {
    client: reqwest::Client,
    api_key: Option<String>,
    api_url: Option<String>,
}

impl RaydiumProvider {
    pub fn new(api_key: Option<String>, api_url: Option<String>) -> Self {
        Self { client: shared_client(), api_key, api_url }
    }

    fn base_url(&self) -> &str {
        self.api_url.as_deref().unwrap_or("https://api-v3.raydium.io")
    }

    /// Parse symbol: "pool_address:token_from:token_to"
    fn parse_symbol(symbol: &str) -> Result<(&str, &str, &str), String> {
        let parts: Vec<&str> = symbol.splitn(3, ':').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid Raydium symbol format '{}', expected 'pool:tokenFrom:tokenTo'", symbol));
        }
        Ok((parts[0], parts[1], parts[2]))
    }
}

#[derive(Debug, Deserialize)]
struct RaydiumPoolResponse {
    success: Option<bool>,
    data: Option<Vec<Option<RaydiumPool>>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RaydiumPool {
    #[serde(rename = "type")]
    _pool_type: Option<String>,
    price: Option<f64>,
    mint_a: Option<RaydiumMint>,
    mint_b: Option<RaydiumMint>,
    tvl: Option<f64>,
    day: Option<RaydiumDayStats>,
}

#[derive(Debug, Deserialize)]
struct RaydiumMint {
    address: Option<String>,
    symbol: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RaydiumDayStats {
    volume: Option<f64>,
}

#[async_trait::async_trait]
impl DataProvider for RaydiumProvider {
    fn info(&self) -> ProviderInfo {
        PROVIDER_INFO_MAP.get("raydium").cloned().unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let (pool_addr, token_from, token_to) = Self::parse_symbol(symbol)?;

        let url = format!("{}/pools/info/ids?ids={}", self.base_url(), pool_addr);
        let mut req = self.client.get(&url);
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await.map_err(|e| format!("Raydium request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Raydium API error: HTTP {}", resp.status()));
        }

        let body: RaydiumPoolResponse = resp.json().await
            .map_err(|e| format!("Raydium JSON parse failed: {}", e))?;

        if body.success == Some(false) || body.data.is_none() {
            return Err(format!("Raydium: pool {} not found", pool_addr));
        }

        let pool = body.data.unwrap()
            .into_iter()
            .flatten()
            .next()
            .ok_or_else(|| format!("Raydium: pool {} not found or returned null", pool_addr))?;

        // pool.price = token_b per token_a ratio
        // Determine direction: if token_from == mintA → price = pool.price (how many B per A)
        // if token_from == mintB → price = 1/pool.price
        let mint_a_addr = pool.mint_a.as_ref().and_then(|m| m.address.as_deref()).unwrap_or("");
        let mint_b_addr = pool.mint_b.as_ref().and_then(|m| m.address.as_deref()).unwrap_or("");

        let (amount_out, _from_symbol, _to_symbol) = if token_from.eq_ignore_ascii_case(mint_a_addr) {
            let out = pool.price.unwrap_or(0.0);
            let fs = pool.mint_a.as_ref().and_then(|m| m.symbol.as_deref()).unwrap_or("?");
            let ts = pool.mint_b.as_ref().and_then(|m| m.symbol.as_deref()).unwrap_or("?");
            (out, fs, ts)
        } else if token_from.eq_ignore_ascii_case(mint_b_addr) {
            let p = pool.price.unwrap_or(1.0);
            let out = if p > 0.0 { 1.0 / p } else { 0.0 };
            let fs = pool.mint_b.as_ref().and_then(|m| m.symbol.as_deref()).unwrap_or("?");
            let ts = pool.mint_a.as_ref().and_then(|m| m.symbol.as_deref()).unwrap_or("?");
            (out, fs, ts)
        } else {
            // Fallback: assume token_from is mintA direction
            let out = pool.price.unwrap_or(0.0);
            let fs = pool.mint_a.as_ref().and_then(|m| m.symbol.as_deref()).unwrap_or("?");
            let ts = pool.mint_b.as_ref().and_then(|m| m.symbol.as_deref()).unwrap_or("?");
            (out, fs, ts)
        };

        // For USD price, we use pool.price as a proxy (Raydium pools are often quoted in USD stables)
        let usd_price = pool.price.unwrap_or(0.0);

        Ok(AssetDataBuilder::new(symbol, "raydium")
            .price(usd_price)
            .volume(pool.day.as_ref().and_then(|d| d.volume))
            .extra_f64("pool_tvl", pool.tvl)
            .extra_f64("amount_out", Some(amount_out))
            .extra_str("token_from", Some(token_from))
            .extra_str("token_to", Some(token_to))
            .extra_str("route_path", Some("Raydium AMM"))
            .extra_str("gas_estimate", Some("~0.000005 SOL"))
            .build())
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        // Batch: collect unique pool addresses, fetch in one call
        let mut pool_map: std::collections::HashMap<String, Vec<(String, String, String)>> = std::collections::HashMap::new();
        for sym in symbols {
            let (pool, tf, tt) = Self::parse_symbol(sym)?;
            pool_map.entry(pool.to_string()).or_default().push((sym.clone(), tf.to_string(), tt.to_string()));
        }

        let pool_ids: Vec<&str> = pool_map.keys().map(|s| s.as_str()).collect();
        let url = format!("{}/pools/info/ids?ids={}", self.base_url(), pool_ids.join(","));
        let mut req = self.client.get(&url);
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await.map_err(|e| format!("Raydium batch request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Raydium API error: HTTP {}", resp.status()));
        }

        let body: RaydiumPoolResponse = resp.json().await
            .map_err(|e| format!("Raydium JSON parse failed: {}", e))?;

        let pools: Vec<RaydiumPool> = body.data.unwrap_or_default().into_iter().flatten().collect();
        let mut results = Vec::new();

        // Raydium API 按請求順序返回 pools，用 index 對應
        for (i, pool) in pools.iter().enumerate() {
            let pool_addr = match pool_ids.get(i) {
                Some(addr) => *addr,
                None => continue,
            };
            let requests = match pool_map.get(pool_addr) {
                Some(r) => r,
                None => continue,
            };
            let mint_a_addr = pool.mint_a.as_ref().and_then(|m| m.address.as_deref()).unwrap_or("");
            for (sym, token_from, token_to) in requests {
                    let amount_out = if token_from.eq_ignore_ascii_case(mint_a_addr) {
                        pool.price.unwrap_or(0.0)
                    } else {
                        let p = pool.price.unwrap_or(1.0);
                        if p > 0.0 { 1.0 / p } else { 0.0 }
                    };

                    let usd_price = pool.price.unwrap_or(0.0);

                    results.push(AssetDataBuilder::new(sym, "raydium")
                        .price(usd_price)
                        .volume(pool.day.as_ref().and_then(|d| d.volume))
                        .extra_f64("pool_tvl", pool.tvl)
                        .extra_f64("amount_out", Some(amount_out))
                        .extra_str("token_from", Some(token_from))
                        .extra_str("token_to", Some(token_to))
                        .extra_str("route_path", Some("Raydium AMM"))
                        .extra_str("gas_estimate", Some("~0.000005 SOL"))
                        .build());
            }
        }

        // Fallback: for any symbols not matched, try individual fetch
        let matched: std::collections::HashSet<String> = results.iter().map(|r| r.symbol.clone()).collect();
        for sym in symbols {
            if !matched.contains(sym.as_str()) {
                match self.fetch_price(sym).await {
                    Ok(d) => results.push(d),
                    Err(e) => eprintln!("[Raydium] fetch_price fallback for {} failed: {}", sym, e),
                }
            }
        }

        Ok(results)
    }
}

#[async_trait::async_trait]
impl DexPoolLookup for RaydiumProvider {
    async fn lookup_pool(&self, pool_address: &str) -> Result<DexPoolInfo, String> {
        let url = format!("{}/pools/info/ids?ids={}", self.base_url(), pool_address);
        let mut req = self.client.get(&url);
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let resp = req.send().await.map_err(|e| format!("Raydium request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Raydium API error: HTTP {}", resp.status()));
        }
        let body: RaydiumPoolResponse = resp.json().await
            .map_err(|e| format!("Raydium JSON parse failed: {}", e))?;
        let pool = body.data.and_then(|d| d.into_iter().flatten().next())
            .ok_or_else(|| format!("Raydium: pool {} not found", pool_address))?;
        Ok(DexPoolInfo {
            token0_address: pool.mint_a.as_ref().and_then(|m| m.address.clone()).unwrap_or_default(),
            token0_symbol: pool.mint_a.as_ref().and_then(|m| m.symbol.clone()).unwrap_or_else(|| "?".into()),
            token1_address: pool.mint_b.as_ref().and_then(|m| m.address.clone()).unwrap_or_default(),
            token1_symbol: pool.mint_b.as_ref().and_then(|m| m.symbol.clone()).unwrap_or_else(|| "?".into()),
        })
    }
}

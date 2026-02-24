use crate::providers::traits::{shared_client, AssetData, AssetDataBuilder, DataProvider, DexPoolInfo, DexPoolLookup, ProviderInfo};
use crate::providers::traits::PROVIDER_INFO_MAP;
use serde::Deserialize;
use std::collections::HashMap;

pub struct SubgraphProvider {
    client: reqwest::Client,
    api_key: Option<String>,
    api_url: Option<String>,
}

impl SubgraphProvider {
    pub fn new(api_key: Option<String>, api_url: Option<String>) -> Self {
        Self { client: shared_client(), api_key, api_url }
    }

    /// Parse symbol: "protocol:pool_address:token_from:token_to"
    fn parse_symbol(symbol: &str) -> Result<(&str, &str, &str, &str), String> {
        let parts: Vec<&str> = symbol.splitn(4, ':').collect();
        if parts.len() != 4 {
            return Err(format!(
                "Invalid Subgraph symbol format '{}', expected 'protocol:pool:tokenFrom:tokenTo'",
                symbol
            ));
        }
        Ok((parts[0], parts[1], parts[2], parts[3]))
    }

    fn get_subgraph_url(&self, protocol: &str) -> Result<String, String> {
        // If user provided a custom api_url, use it directly
        if let Some(ref url) = self.api_url {
            return Ok(url.clone());
        }

        let api_key = self.api_key.as_deref()
            .ok_or_else(|| "Subgraph requires an API key from The Graph (thegraph.com)".to_string())?;

        let subgraph_id = match protocol {
            "uniswap_v3" => "5zvR82QoaXYFyDEKLZ9t6v9adgnptxYpKpSbxtgVENFV",
            "sushiswap" => "6NUtT5mGjZ1tSPHceYRnFnJFYBGMvEPLszerMRmCw4C3",
            "pancakeswap" => "A1fvJWQLBeUAggX2WtXq31Dqkn2gHP3Jnj2bh8JqBnQo",
            _ => return Err(format!("Unsupported DEX protocol: {}", protocol)),
        };

        Ok(format!(
            "https://gateway.thegraph.com/api/{}/subgraphs/id/{}",
            api_key, subgraph_id
        ))
    }

    fn build_query(pool_address: &str) -> String {
        format!(
            r#"{{ pool(id: "{}") {{ token0 {{ id symbol decimals }} token1 {{ id symbol decimals }} token0Price token1Price totalValueLockedUSD volumeUSD }} }}"#,
            pool_address.to_lowercase()
        )
    }
}

#[derive(Debug, Deserialize)]
struct GraphResponse {
    data: Option<GraphData>,
    errors: Option<Vec<GraphError>>,
}

#[derive(Debug, Deserialize)]
struct GraphData {
    pool: Option<PoolData>,
}

#[derive(Debug, Deserialize)]
struct GraphError {
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PoolData {
    token0: Option<TokenData>,
    token1: Option<TokenData>,
    token0_price: Option<String>,
    token1_price: Option<String>,
    total_value_locked_usd: Option<String>,
    volume_usd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenData {
    id: Option<String>,
    symbol: Option<String>,
    #[allow(dead_code)]
    decimals: Option<String>,
}

#[async_trait::async_trait]
impl DataProvider for SubgraphProvider {
    fn info(&self) -> ProviderInfo {
        PROVIDER_INFO_MAP.get("subgraph").cloned().unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let (protocol, pool_addr, token_from, token_to) = Self::parse_symbol(symbol)?;
        let url = self.get_subgraph_url(protocol)?;
        let query = Self::build_query(pool_addr);

        let body = serde_json::json!({ "query": query });
        let resp = self.client.post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Subgraph request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Subgraph API error: HTTP {}", resp.status()));
        }

        let graph_resp: GraphResponse = resp.json().await
            .map_err(|e| format!("Subgraph JSON parse failed: {}", e))?;

        if let Some(errors) = &graph_resp.errors {
            let msg = errors.first()
                .and_then(|e| e.message.as_deref())
                .unwrap_or("Unknown error");
            return Err(format!("Subgraph query error: {}", msg));
        }

        let pool = graph_resp.data
            .and_then(|d| d.pool)
            .ok_or_else(|| format!("Subgraph: pool {} not found", pool_addr))?;

        let token0_id = pool.token0.as_ref().and_then(|t| t.id.as_deref()).unwrap_or("");
        let token1_id = pool.token1.as_ref().and_then(|t| t.id.as_deref()).unwrap_or("");

        // Determine price direction
        // token0Price = how many token0 per 1 token1
        // token1Price = how many token1 per 1 token0
        let price: f64 = if token_from.eq_ignore_ascii_case(token0_id) {
            // token_from is token0, we want: 1 token0 → X token1 = token1Price
            pool.token1_price.as_deref().unwrap_or("0").parse().unwrap_or(0.0)
        } else if token_from.eq_ignore_ascii_case(token1_id) {
            // token_from is token1, we want: 1 token1 → X token0 = token0Price
            pool.token0_price.as_deref().unwrap_or("0").parse().unwrap_or(0.0)
        } else {
            // Fallback
            pool.token1_price.as_deref().unwrap_or("0").parse().unwrap_or(0.0)
        };

        let tvl: f64 = pool.total_value_locked_usd.as_deref().unwrap_or("0").parse().unwrap_or(0.0);
        let volume: f64 = pool.volume_usd.as_deref().unwrap_or("0").parse().unwrap_or(0.0);

        let protocol_name = match protocol {
            "uniswap_v3" => "Uniswap V3",
            "sushiswap" => "SushiSwap",
            "pancakeswap" => "PancakeSwap",
            _ => protocol,
        };

        Ok(AssetDataBuilder::new(symbol, "subgraph")
            .price(price)
            .volume(Some(volume))
            .extra_f64("pool_tvl", Some(tvl))
            .extra_f64("volume_24h", Some(volume))
            .extra_str("token_from", Some(token_from))
            .extra_str("token_to", Some(token_to))
            .extra_str("route_path", Some(&format!("{} Direct", protocol_name)))
            .extra_str("gas_estimate", Some("~0.005 ETH"))
            .build())
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        // Group by protocol to minimize endpoint switches
        let mut by_protocol: HashMap<String, Vec<String>> = HashMap::new();
        for sym in symbols {
            let (protocol, _, _, _) = Self::parse_symbol(sym)?;
            by_protocol.entry(protocol.to_string()).or_default().push(sym.clone());
        }

        let mut results = Vec::new();
        for (_protocol, syms) in &by_protocol {
            // Subgraph doesn't support multi-pool queries easily, fetch individually
            for sym in syms {
                match self.fetch_price(sym).await {
                    Ok(d) => results.push(d),
                    Err(e) => eprintln!("[Subgraph] fetch_price for {} failed: {}", sym, e),
                }
            }
        }
        Ok(results)
    }
}

#[async_trait::async_trait]
impl DexPoolLookup for SubgraphProvider {
    async fn lookup_pool(&self, pool_address: &str) -> Result<DexPoolInfo, String> {
        // pool_address = "protocol:0x..." — extract protocol prefix
        let (protocol, addr) = pool_address.split_once(':')
            .ok_or_else(|| format!("Subgraph lookup requires 'protocol:address' format, got '{}'", pool_address))?;
        let url = self.get_subgraph_url(protocol)?;
        let query = Self::build_query(addr);
        let body = serde_json::json!({ "query": query });
        let resp = self.client.post(&url).json(&body).send().await
            .map_err(|e| format!("Subgraph request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Subgraph API error: HTTP {}", resp.status()));
        }
        let graph_resp: GraphResponse = resp.json().await
            .map_err(|e| format!("Subgraph JSON parse failed: {}", e))?;
        if let Some(errors) = &graph_resp.errors {
            let msg = errors.first().and_then(|e| e.message.as_deref()).unwrap_or("Unknown error");
            return Err(format!("Subgraph query error: {}", msg));
        }
        let pool = graph_resp.data.and_then(|d| d.pool)
            .ok_or_else(|| format!("Subgraph: pool {} not found", addr))?;
        Ok(DexPoolInfo {
            token0_address: pool.token0.as_ref().and_then(|t| t.id.clone()).unwrap_or_default(),
            token0_symbol: pool.token0.as_ref().and_then(|t| t.symbol.clone()).unwrap_or_else(|| "?".into()),
            token1_address: pool.token1.as_ref().and_then(|t| t.id.clone()).unwrap_or_default(),
            token1_symbol: pool.token1.as_ref().and_then(|t| t.symbol.clone()).unwrap_or_else(|| "?".into()),
        })
    }
}

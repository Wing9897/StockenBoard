use super::traits::*;

/// OKX DEX 聚合器 — 多鏈 DEX 聚合器 Spot Price
/// 使用 swap quote API 推導即時價格：
/// GET https://web3.okx.com/api/v5/dex/aggregator/quote
///   ?chainId=1&fromTokenAddress=<token>&toTokenAddress=<USDC>&amount=<1_token>
///
/// 需要 API key（OKX Web3 Developer Portal 免費申請）
/// Header: OK-ACCESS-KEY
pub struct OkxDexProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl OkxDexProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: shared_client(),
            api_key,
        }
    }
}

/// 鏈 ID 常量
const CHAIN_ETH: &str = "1";
const CHAIN_BSC: &str = "56";
const CHAIN_POLYGON: &str = "137";
const CHAIN_ARBITRUM: &str = "42161";
const CHAIN_SOLANA: &str = "501";

/// USDC 地址（各鏈）
fn usdc_address(chain_id: &str) -> &'static str {
    match chain_id {
        "1" => "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",       // ETH USDC
        "56" => "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d",      // BSC USDC
        "137" => "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359",     // Polygon USDC
        "42161" => "0xaf88d065e77c8cc2239327c5edb3a432268e5831",   // Arbitrum USDC
        "501" => "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  // Solana USDC
        _ => "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",         // 默認 ETH
    }
}

/// USDC decimals
fn usdc_decimals(chain_id: &str) -> u32 {
    match chain_id {
        "501" => 6,  // Solana USDC
        _ => 6,      // EVM USDC 都是 6
    }
}

/// 解析用戶輸入的 symbol → (chain_id, token_address, decimals)
/// 格式：
///   - "ETH" / "WETH" → Ethereum mainnet WETH
///   - "BNB" → BSC WBNB
///   - "SOL" → Solana wrapped SOL
///   - "eth:0x..." → 指定鏈 + 合約地址
///   - "sol:mint_address" → Solana mint address
///   - "arb:0x..." → Arbitrum 合約地址
fn parse_okx_dex_symbol(symbol: &str) -> (String, String, u32) {
    let s = symbol.trim();

    // 格式: "chain:address" 或 "chain:address:decimals"
    if let Some((chain_prefix, rest)) = s.split_once(':') {
        let chain_id = match chain_prefix.to_lowercase().as_str() {
            "eth" | "ethereum" => CHAIN_ETH,
            "bsc" | "bnb" => CHAIN_BSC,
            "polygon" | "matic" => CHAIN_POLYGON,
            "arb" | "arbitrum" => CHAIN_ARBITRUM,
            "sol" | "solana" => CHAIN_SOLANA,
            _ => CHAIN_ETH,
        };
        // 可能有 :decimals 後綴
        if let Some((addr, dec_str)) = rest.split_once(':') {
            let decimals = dec_str.parse().unwrap_or(18);
            return (chain_id.to_string(), addr.to_string(), decimals);
        }
        let decimals = if chain_id == CHAIN_SOLANA { 9 } else { 18 };
        return (chain_id.to_string(), rest.to_string(), decimals);
    }

    // 常見代號快捷映射
    let upper = s.to_uppercase();
    match upper.as_str() {
        // Ethereum
        "ETH" | "WETH" => (CHAIN_ETH.into(), "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".into(), 18),
        "WBTC" => (CHAIN_ETH.into(), "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599".into(), 8),
        "UNI" => (CHAIN_ETH.into(), "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984".into(), 18),
        "LINK" => (CHAIN_ETH.into(), "0x514910771af9ca656af840dff83e8264ecf986ca".into(), 18),
        "AAVE" => (CHAIN_ETH.into(), "0x7fc66500c84a76ad7e9c93437bfc5ac33e2ddae9".into(), 18),
        "PEPE" => (CHAIN_ETH.into(), "0x6982508145454ce325ddbe47a25d4ec3d2311933".into(), 18),
        "SHIB" => (CHAIN_ETH.into(), "0x95ad61b0a150d79219dcf64e1e6cc01f0b64c4ce".into(), 18),
        // BSC
        "BNB" | "WBNB" => (CHAIN_BSC.into(), "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".into(), 18),
        "CAKE" => (CHAIN_BSC.into(), "0x0e09fabb73bd3ade0a17ecc321fd13a19e81ce82".into(), 18),
        // Solana
        "SOL" | "WSOL" => (CHAIN_SOLANA.into(), "So11111111111111111111111111111111111111112".into(), 9),
        "JUP" => (CHAIN_SOLANA.into(), "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN".into(), 6),
        "BONK" => (CHAIN_SOLANA.into(), "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".into(), 5),
        "WIF" => (CHAIN_SOLANA.into(), "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm".into(), 6),
        // Polygon
        "MATIC" | "POL" => (CHAIN_POLYGON.into(), "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".into(), 18),
        // Arbitrum
        "ARB" => (CHAIN_ARBITRUM.into(), "0x912ce59144191c1204e64559fe8253a0e49e6548".into(), 18),
        // 默認：假設 Ethereum 合約地址
        _ => (CHAIN_ETH.into(), s.to_string(), 18),
    }
}

fn chain_name(chain_id: &str) -> &'static str {
    match chain_id {
        "1" => "Ethereum",
        "56" => "BSC",
        "137" => "Polygon",
        "42161" => "Arbitrum",
        "501" => "Solana",
        _ => "Unknown",
    }
}

#[async_trait::async_trait]
impl DataProvider for OkxDexProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("okx_dex").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_deref()
            .ok_or_else(|| "OKX DEX 需要 API Key（在 OKX Web3 Developer Portal 免費申請）".to_string())?;

        let (chain_id, token_address, decimals) = parse_okx_dex_symbol(symbol);
        let usdc_addr = usdc_address(&chain_id);
        let usdc_dec = usdc_decimals(&chain_id);

        // 用 1 個完整 token 的最小單位數量來查詢報價
        let amount = 10u128.pow(decimals);

        let url = format!(
            "https://web3.okx.com/api/v5/dex/aggregator/quote?chainId={}&fromTokenAddress={}&toTokenAddress={}&amount={}",
            chain_id, token_address, usdc_addr, amount
        );

        let resp: serde_json::Value = self.client.get(&url)
            .header("OK-ACCESS-KEY", api_key)
            .send().await.map_err(|e| format!("OKX DEX 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("OKX DEX API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("OKX DEX 解析失敗: {}", e))?;

        let code = resp["code"].as_str().unwrap_or("");
        if code != "0" {
            let msg = resp["msg"].as_str().unwrap_or("未知錯誤");
            return Err(format!("OKX DEX 錯誤 ({}): {}", code, msg));
        }

        let data = &resp["data"][0];
        let to_amount_str = data["toTokenAmount"].as_str().unwrap_or("0");
        let to_amount: f64 = to_amount_str.parse().unwrap_or(0.0);
        // toTokenAmount 是 USDC 的最小單位，需要除以 10^usdc_decimals 得到 USD 價格
        let price = to_amount / 10f64.powi(usdc_dec as i32);

        let estimate_gas = data["estimateGasFee"].as_str()
            .and_then(|s| s.parse::<f64>().ok());

        Ok(
            AssetDataBuilder::new(symbol, "okx_dex")
                .price(price)
                .currency("USD")
                .extra_str("鏈", Some(chain_name(&chain_id)))
                .extra_str("token", Some(&token_address))
                .extra_f64("預估Gas", estimate_gas)
                .build()
        )
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        // OKX DEX quote API 不支持批量，使用並行請求
        let futures: Vec<_> = symbols.iter()
            .map(|s| self.fetch_price(s))
            .collect();
        let results = futures::future::join_all(futures).await;
        Ok(results.into_iter().filter_map(|r| r.ok()).collect())
    }
}

use super::traits::*;
use std::collections::HashMap;

/// Jupiter Price API v3 — Solana DEX 聚合器
/// Spot Price endpoint: GET https://api.jup.ag/price/v3?ids=<mint_addresses>
/// 需要 API Key（在 portal.jup.ag 免費申請）
///
/// 用戶輸入格式：Solana mint address 或常見代號（SOL, JUP 等）
/// 內部自動轉換為 mint address
pub struct JupiterProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl JupiterProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: shared_client(),
            api_key,
        }
    }
}

/// 常見 Solana token → mint address 映射
fn to_mint_address(symbol: &str) -> String {
    let s = symbol.trim();
    // 如果已經是 mint address（base58, 長度 32-44），直接返回
    if s.len() >= 32 && s.chars().all(|c| c.is_alphanumeric()) {
        return s.to_string();
    }
    let upper = s.to_uppercase();
    // 去掉常見後綴
    let base = upper
        .strip_suffix("-USD").or(upper.strip_suffix("-USDC"))
        .or(upper.strip_suffix("USDT")).or(upper.strip_suffix("USDC"))
        .unwrap_or(&upper);
    match base {
        "SOL" | "WSOL" => "So11111111111111111111111111111111111111112",
        "JUP" => "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
        "USDC" => "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "USDT" => "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
        "BONK" => "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
        "WIF" => "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm",
        "PYTH" => "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3",
        "RAY" => "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R",
        "ORCA" => "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE",
        "MNDE" => "MNDEFzGvMt87ueuHvVU9VcTqsAP5b3fTGPsHuuPA5ey",
        "JITO" => "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn",
        "RENDER" | "RNDR" => "rndrizKT3MK1iimdxRdWabcF7Zg7AR5T4nud4EkHBof",
        "HNT" => "hntyVP6YFm1Hg25TN9WGLqM12b8TQmcknKrdu1oxWux",
        "TRUMP" => "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN",
        _ => return s.to_string(), // 假設用戶直接輸入 mint address
    }.to_string()
}

/// 從 mint address 反查顯示用代號（best effort，未來可用於 UI）
#[allow(dead_code)]
fn mint_to_symbol(mint: &str) -> &str {
    match mint {
        "So11111111111111111111111111111111111111112" => "SOL",
        "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN" => "JUP",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => "USDC",
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => "USDT",
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => "BONK",
        "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm" => "WIF",
        _ => mint,
    }
}

fn parse_jupiter_price(symbol: &str, mint: &str, data: &serde_json::Value) -> Option<AssetData> {
    let entry = data.get(mint)?;
    let price = entry.get("usdPrice")?.as_f64()?;
    let change_pct = entry.get("priceChange24h").and_then(|v| v.as_f64());
    let change_abs = change_pct.map(|p| price * p / (100.0 + p));

    Some(
        AssetDataBuilder::new(symbol, "jupiter")
            .price(price)
            .currency("USD")
            .change_24h(change_abs)
            .change_percent_24h(change_pct)
            .extra_str("mint", Some(mint))
            .build(),
    )
}

#[async_trait::async_trait]
impl DataProvider for JupiterProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("jupiter").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let api_key = self.api_key.as_deref()
            .ok_or_else(|| "Jupiter 需要 API Key（在 portal.jup.ag 免費申請）".to_string())?;
        let mint = to_mint_address(symbol);
        let url = format!("https://api.jup.ag/price/v3?ids={}", mint);
        let req = self.client.get(&url).header("x-api-key", api_key);
        let data: serde_json::Value = req
            .send().await.map_err(|e| format!("Jupiter 連接失敗: {}", e))?
            .error_for_status().map_err(|e| format!("Jupiter API 錯誤: {}", e))?
            .json().await.map_err(|e| format!("Jupiter 解析失敗: {}", e))?;

        parse_jupiter_price(symbol, &mint, &data)
            .ok_or_else(|| format!("Jupiter 找不到 {} 的價格", symbol))
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() { return Ok(vec![]); }
        if symbols.len() == 1 { return self.fetch_price(&symbols[0]).await.map(|d| vec![d]); }

        let api_key = self.api_key.as_deref()
            .ok_or_else(|| "Jupiter 需要 API Key（在 portal.jup.ag 免費申請）".to_string())?;

        // 批量查詢：最多 50 個 mint address
        let mint_map: HashMap<String, String> = symbols.iter()
            .map(|s| (to_mint_address(s), s.clone()))
            .collect();
        let mints: Vec<&str> = mint_map.keys().map(|s| s.as_str()).collect();

        // Jupiter 支持逗號分隔批量查詢
        let mut results = Vec::new();
        for chunk in mints.chunks(50) {
            let ids = chunk.join(",");
            let url = format!("https://api.jup.ag/price/v3?ids={}", ids);
            let req = self.client.get(&url).header("x-api-key", api_key);
            let data: serde_json::Value = req
                .send().await.map_err(|e| format!("Jupiter 批量連接失敗: {}", e))?
                .error_for_status().map_err(|e| format!("Jupiter 批量 API 錯誤: {}", e))?
                .json().await.map_err(|e| format!("Jupiter 批量解析失敗: {}", e))?;

            for (mint, original_symbol) in &mint_map {
                if chunk.contains(&mint.as_str()) {
                    if let Some(asset) = parse_jupiter_price(original_symbol, mint, &data) {
                        results.push(asset);
                    }
                }
            }
        }
        Ok(results)
    }
}

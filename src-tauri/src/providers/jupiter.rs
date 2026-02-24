use super::traits::*;
use std::collections::HashMap;

/// Jupiter — Solana DEX 聚合器
///
/// 現貨模式: Price API v3 — GET https://api.jup.ag/price/v3?ids=<mint_addresses>
/// DEX 模式:  Quote API   — GET https://api.jup.ag/swap/v1/quote?inputMint=&outputMint=&amount=
///
/// DEX symbol 格式: auto:<inputMint>:<outputMint>
/// 需要 API Key（在 portal.jup.ag 免費申請）
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

    /// 判斷是否為 DEX symbol 格式 (含 ':')
    fn is_dex_symbol(symbol: &str) -> bool {
        symbol.contains(':')
    }

    /// 解析 DEX symbol: "pool:inputMint:outputMint"
    fn parse_dex_symbol(symbol: &str) -> Result<(&str, &str), String> {
        let parts: Vec<&str> = symbol.splitn(3, ':').collect();
        if parts.len() < 3 {
            return Err("Jupiter DEX 格式: auto:inputMint:outputMint".into());
        }
        // parts[0] = pool (ignored, always "auto" for Jupiter)
        let input_mint = parts[1].trim();
        let output_mint = parts[2].trim();
        if input_mint.is_empty() || output_mint.is_empty() {
            return Err("inputMint 和 outputMint 不能為空".into());
        }
        Ok((input_mint, output_mint))
    }

    /// 用 Quote API 取得報價
    async fn fetch_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Result<serde_json::Value, String> {
        let api_key = self.api_key.as_deref()
            .ok_or_else(|| "Jupiter 需要 API Key（在 portal.jup.ag 免費申請）".to_string())?;

        let url = format!(
            "https://api.jup.ag/swap/v1/quote?inputMint={}&outputMint={}&amount={}&slippageBps=50&restrictIntermediateTokens=true",
            input_mint, output_mint, amount
        );
        let resp = self.client.get(&url)
            .header("x-api-key", api_key)
            .send().await
            .map_err(|e| format!("Jupiter Quote 連接失敗: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Jupiter Quote API 錯誤: HTTP {} — {}", status, body));
        }

        resp.json().await.map_err(|e| format!("Jupiter Quote 解析失敗: {}", e))
    }

    /// DEX 模式的 fetch_price
    async fn fetch_dex_price(&self, symbol: &str) -> Result<AssetData, String> {
        let (input_mint, output_mint) = Self::parse_dex_symbol(symbol)?;

        // 取得 input token 的 decimals（用 Price API 查一下）
        let decimals = self.get_token_decimals(input_mint).await.unwrap_or(9);
        let amount = 10u64.pow(decimals as u32); // 1 個 input token

        let quote = self.fetch_quote(input_mint, output_mint, amount).await?;

        let in_amount_raw = quote.get("inAmount")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(amount as f64);
        let out_amount_raw = quote.get("outAmount")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or("Jupiter Quote 缺少 outAmount")?;

        // 取得 output token decimals
        let out_decimals = self.get_token_decimals(output_mint).await.unwrap_or(6);
        let amount_out = out_amount_raw / 10f64.powi(out_decimals as i32);
        let amount_in = in_amount_raw / 10f64.powi(decimals as i32);

        // price = outAmount / inAmount (以 output token 計價)
        let price = if amount_in > 0.0 { amount_out / amount_in } else { 0.0 };

        let price_impact = quote.get("priceImpactPct")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok());

        // 路由路徑
        let route_path = quote.get("routePlan")
            .and_then(|v| v.as_array())
            .map(|plans| {
                plans.iter()
                    .filter_map(|p| p.get("swapInfo").and_then(|s| s.get("label")).and_then(|l| l.as_str()))
                    .collect::<Vec<_>>()
                    .join(" → ")
            })
            .unwrap_or_else(|| "Jupiter".into());

        let input_sym = mint_to_symbol(input_mint);
        let output_sym = mint_to_symbol(output_mint);

        Ok(AssetDataBuilder::new(symbol, "jupiter")
            .price(price)
            .currency(output_sym)
            .extra_f64("amount_out", Some(amount_out))
            .extra_f64("price_impact", price_impact)
            .extra_str("route_path", Some(&route_path))
            .extra_str("gas_estimate", Some("~0.000005 SOL"))
            .extra_str("token_from", Some(input_sym))
            .extra_str("token_to", Some(output_sym))
            .build())
    }

    /// 取得 token decimals（透過 Price API 的 extraInfo）
    async fn get_token_decimals(&self, mint: &str) -> Result<u8, String> {
        // 常見 token 直接返回
        match mint {
            "So11111111111111111111111111111111111111112" => return Ok(9),  // SOL
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => return Ok(6), // USDC
            "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => return Ok(6),  // USDT
            _ => {}
        }
        // 用 Price API 查 extraInfo
        if let Some(api_key) = self.api_key.as_deref() {
            let url = format!("https://api.jup.ag/price/v3?ids={}&showExtraInfo=true", mint);
            if let Ok(resp) = self.client.get(&url).header("x-api-key", api_key).send().await {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(decimals) = json.get("data")
                        .and_then(|d| d.get(mint))
                        .and_then(|e| e.get("extraInfo"))
                        .and_then(|e| e.get("quotedPrice"))
                        .and_then(|e| e.get("buyTokenDecimals"))
                        .and_then(|v| v.as_u64())
                    {
                        return Ok(decimals as u8);
                    }
                }
            }
        }
        Ok(9) // fallback: assume 9 decimals (SOL standard)
    }
}

/// 常見 Solana token → mint address 映射
fn to_mint_address(symbol: &str) -> String {
    let s = symbol.trim();
    // 如果已經是 mint address（>= 32 字元的 base58），直接返回
    if s.len() >= 32 && s.chars().all(|c| c.is_alphanumeric()) {
        return s.to_string();
    }
    let upper = s.to_uppercase();
    // 去掉常見的計價後綴
    let base = upper.strip_suffix("-USD")
        .or_else(|| upper.strip_suffix("-USDC"))
        .or_else(|| upper.strip_suffix("/USD"))
        .or_else(|| upper.strip_suffix("/USDC"))
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
        _ => return s.to_string(),
    }.to_string()
}

fn mint_to_symbol(mint: &str) -> &str {
    match mint {
        "So11111111111111111111111111111111111111112" => "SOL",
        "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN" => "JUP",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => "USDC",
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => "USDT",
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => "BONK",
        "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm" => "WIF",
        "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R" => "RAY",
        "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE" => "ORCA",
        "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3" => "PYTH",
        "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn" => "JITO",
        "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN" => "TRUMP",
        _ => mint,
    }
}

fn parse_jupiter_price(symbol: &str, mint: &str, resp: &serde_json::Value) -> Option<AssetData> {
    // Price API v3 回應格式: { "data": { "<mint>": { "price": "123.45", ... } } }
    let entry = resp.get("data").and_then(|d| d.get(mint))?;
    let price = entry.get("price")
        .and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| v.as_f64()))
        ?;

    Some(
        AssetDataBuilder::new(symbol, "jupiter")
            .price(price)
            .currency("USD")
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
        // DEX 模式: symbol 含 ':'
        if Self::is_dex_symbol(symbol) {
            return self.fetch_dex_price(symbol).await;
        }

        // 現貨模式: Price API
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

        // 分離 DEX 和現貨 symbols
        let (dex_syms, spot_syms): (Vec<_>, Vec<_>) = symbols.iter()
            .partition(|s| Self::is_dex_symbol(s));

        let mut results = Vec::new();

        // DEX symbols: 逐一查詢（每個都是不同的 quote）
        for sym in &dex_syms {
            match self.fetch_dex_price(sym).await {
                Ok(data) => results.push(data),
                Err(e) => eprintln!("[Jupiter DEX] Error fetching {}: {}", sym, e),
            }
        }

        // 現貨 symbols: 批量查詢
        if !spot_syms.is_empty() {
            let api_key = self.api_key.as_deref()
                .ok_or_else(|| "Jupiter 需要 API Key（在 portal.jup.ag 免費申請）".to_string())?;

            let mint_map: HashMap<String, String> = spot_syms.iter()
                .map(|s| (to_mint_address(s), s.to_string()))
                .collect();
            let mints: Vec<&str> = mint_map.keys().map(|s| s.as_str()).collect();

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
        }

        Ok(results)
    }
}

/// Jupiter DexPoolLookup — 用 Quote API 查詢交易對資訊
/// pool_address 格式: "auto" 或 "inputMint,outputMint" 或 "inputSymbol,outputSymbol"
#[async_trait::async_trait]
impl DexPoolLookup for JupiterProvider {
    async fn lookup_pool(&self, pool_address: &str) -> Result<DexPoolInfo, String> {
        // 解析: "SOL,USDC" 或 "mintA,mintB"
        let (input_raw, output_raw) = pool_address.split_once(',')
            .ok_or_else(|| "Jupiter 查詢格式: SOL,USDC 或 inputMint,outputMint（逗號分隔）".to_string())?;

        let input_mint = to_mint_address(input_raw.trim());
        let output_mint = to_mint_address(output_raw.trim());

        // 做一次小額 quote 來驗證交易對存在
        let decimals = self.get_token_decimals(&input_mint).await.unwrap_or(9);
        let amount = 10u64.pow(decimals as u32); // 1 token
        let _quote = self.fetch_quote(&input_mint, &output_mint, amount).await?;

        let input_sym = mint_to_symbol(&input_mint).to_string();
        let output_sym = mint_to_symbol(&output_mint).to_string();

        Ok(DexPoolInfo {
            token0_address: input_mint,
            token0_symbol: input_sym,
            token1_address: output_mint,
            token1_symbol: output_sym,
        })
    }
}

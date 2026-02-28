use super::traits::*;

pub struct CoinPaprikaProvider {
    client: reqwest::Client,
}

impl CoinPaprikaProvider {
    pub fn new() -> Self {
        Self {
            client: shared_client(),
        }
    }
}

/// Convert to CoinPaprika ID format: btc-bitcoin, eth-ethereum
fn to_coinpaprika_id(symbol: &str) -> String {
    let (base, _) = parse_crypto_symbol(symbol);
    match base.as_str() {
        "BTC" => "btc-bitcoin",
        "ETH" => "eth-ethereum",
        "BNB" => "bnb-binance-coin",
        "SOL" => "sol-solana",
        "XRP" => "xrp-xrp",
        "ADA" => "ada-cardano",
        "DOGE" => "doge-dogecoin",
        "DOT" => "dot-polkadot",
        "AVAX" => "avax-avalanche",
        "MATIC" | "POL" => "matic-polygon",
        "LINK" => "link-chainlink",
        "UNI" => "uni-uniswap",
        "ATOM" => "atom-cosmos",
        "LTC" => "ltc-litecoin",
        "SHIB" => "shib-shiba-inu",
        "TRX" => "trx-tron",
        "NEAR" => "near-near-protocol",
        "APT" => "apt-aptos",
        "ARB" => "arb-arbitrum",
        "OP" => "op-optimism",
        "SUI" => "sui-sui",
        "PEPE" => "pepe-pepe",
        "FIL" => "fil-filecoin",
        "AAVE" => "aave-new",
        "MKR" => "mkr-maker",
        _ => return format!("{}-{}", base.to_lowercase(), base.to_lowercase()),
    }
    .to_string()
}

fn parse_paprika_ticker(symbol: &str, data: &serde_json::Value) -> AssetData {
    let usd = &data["quotes"]["USD"];
    let pf = |k: &str| usd[k].as_f64();
    let price = pf("price").unwrap_or(0.0);
    let pct = pf("percent_change_24h");
    // Calculate absolute change from percentage
    let change = pct.map(|p| price * p / (100.0 + p));

    AssetDataBuilder::new(symbol, "coinpaprika")
        .price(price)
        .currency("USD")
        .change_24h(change)
        .change_percent_24h(pct)
        .volume(pf("volume_24h"))
        .market_cap(pf("market_cap"))
        .extra_f64("ATH", pf("ath_price"))
        .extra_f64("1h%", pf("percent_change_1h"))
        .extra_f64("7d%", pf("percent_change_7d"))
        .build()
}

#[async_trait::async_trait]
impl DataProvider for CoinPaprikaProvider {
    fn info(&self) -> ProviderInfo {
        get_provider_info("coinpaprika").unwrap()
    }

    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String> {
        let id = to_coinpaprika_id(symbol);
        let url = format!("https://api.coinpaprika.com/v1/tickers/{}", id);
        let data: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("CoinPaprika 連接失敗: {}", e))?
            .error_for_status()
            .map_err(|e| format!("CoinPaprika API 錯誤: {}", e))?
            .json()
            .await
            .map_err(|e| format!("CoinPaprika 解析失敗: {}", e))?;

        Ok(parse_paprika_ticker(symbol, &data))
    }

    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        if symbols.len() == 1 {
            return self.fetch_price(&symbols[0]).await.map(|d| vec![d]);
        }

        // CoinPaprika /tickers returns all coins
        let url = "https://api.coinpaprika.com/v1/tickers";
        let arr: Vec<serde_json::Value> = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("CoinPaprika 批量連接失敗: {}", e))?
            .json()
            .await
            .map_err(|e| format!("CoinPaprika 批量解析失敗: {}", e))?;

        let mut map = std::collections::HashMap::new();
        for item in &arr {
            if let Some(id) = item["id"].as_str() {
                map.insert(id.to_string(), item);
            }
        }

        let mut out = Vec::new();
        for sym in symbols {
            let id = to_coinpaprika_id(sym);
            if let Some(item) = map.get(&id) {
                out.push(parse_paprika_ticker(sym, item));
            }
        }
        Ok(out)
    }
}

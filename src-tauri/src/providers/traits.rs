use std::sync::Arc;

use super::types::{AssetData, DexPoolInfo, ProviderInfo, WsTickerUpdate};

#[async_trait::async_trait]
pub trait DataProvider: Send + Sync {
    fn info(&self) -> ProviderInfo;
    async fn fetch_price(&self, symbol: &str) -> Result<AssetData, String>;
    async fn fetch_prices(&self, symbols: &[String]) -> Result<Vec<AssetData>, String> {
        // Default fallback: 逐一查詢（各 provider 應覆寫此方法以使用批量/並行）
        let mut results = Vec::new();
        for symbol in symbols {
            match self.fetch_price(symbol).await {
                Ok(data) => results.push(data),
                Err(e) => eprintln!("Error fetching {}: {}", symbol, e),
            }
        }
        Ok(results)
    }
}

/// Trait for DEX providers that can look up pool token info
#[async_trait::async_trait]
pub trait DexPoolLookup: Send + Sync {
    async fn lookup_pool(&self, pool_address: &str) -> Result<DexPoolInfo, String>;
}

/// Trait for providers that support WebSocket streaming
#[async_trait::async_trait]
pub trait WebSocketProvider: Send + Sync {
    /// Subscribe to real-time updates for given symbols.
    /// Returns a JoinHandle for the WS connection task so it can be aborted on cleanup.
    async fn subscribe(
        &self,
        symbols: Vec<String>,
        sender: Arc<tokio::sync::broadcast::Sender<WsTickerUpdate>>,
    ) -> Result<tokio::task::JoinHandle<()>, String>;
}

/// ProviderRegistry — 共享 Provider 管理 + Rate Limiting
///
/// 1. Lazy init：首次使用時才建立 provider instance
/// 2. 共用實例：Polling 和 IPC commands 共用同一組 provider
/// 3. Rate limiting：每個 provider 一個 Semaphore，防止 API 過載
use crate::db::DbPool;
use crate::providers::{create_provider_with_url, AssetData, DataProvider};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};

/// 每個 provider 的默認並發上限
const DEFAULT_CONCURRENT_REQUESTS: usize = 3;
/// 有 API key 的 provider 並發上限
const KEYED_CONCURRENT_REQUESTS: usize = 5;

pub struct ProviderRegistry {
    /// 共享的 provider instances（lazy init）
    providers: RwLock<HashMap<String, Arc<dyn DataProvider>>>,
    /// 每個 provider 的 rate limiter
    limiters: RwLock<HashMap<String, Arc<Semaphore>>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            limiters: RwLock::new(HashMap::new()),
        }
    }

    /// 取得或建立 provider instance（lazy，從 DbPool 讀取 API key）
    pub async fn get_or_create(
        &self,
        id: &str,
        db: &DbPool,
    ) -> Option<Arc<dyn DataProvider>> {
        // 先嘗試從快取中取得
        {
            let p = self.providers.read().await;
            if let Some(provider) = p.get(id) {
                return Some(provider.clone());
            }
        }

        // 從 DB 讀取設定
        let (key, secret, url) = match db.get_provider_settings(id) {
            Ok(Some(settings)) => (
                settings.api_key.filter(|k| !k.is_empty()),
                settings.api_secret.filter(|s| !s.is_empty()),
                settings.api_url.filter(|u| !u.is_empty()),
            ),
            _ => (None, None, None),
        };

        // 建立 provider
        let provider = create_provider_with_url(id, key.clone(), secret, url)?;
        self.providers
            .write()
            .await
            .insert(id.to_string(), provider.clone());

        // 確保有對應的 rate limiter
        self.ensure_limiter(id, key.is_some()).await;

        Some(provider)
    }

    /// 取得或建立 provider（直接指定 key，用於 enable_provider 場景）
    pub async fn get_or_create_with_key(
        &self,
        id: &str,
        api_key: Option<String>,
        api_secret: Option<String>,
        api_url: Option<String>,
    ) -> Option<Arc<dyn DataProvider>> {
        // 如果沒提供 key，嘗試用快取
        if api_key.is_none() {
            let p = self.providers.read().await;
            if let Some(provider) = p.get(id) {
                return Some(provider.clone());
            }
        }

        let has_key = api_key.is_some();
        let provider = create_provider_with_url(id, api_key, api_secret, api_url)?;
        self.providers
            .write()
            .await
            .insert(id.to_string(), provider.clone());
        self.ensure_limiter(id, has_key).await;
        Some(provider)
    }

    /// 帶 rate limiting 的 fetch_prices
    pub async fn fetch_with_limit(
        &self,
        id: &str,
        symbols: &[String],
        db: &DbPool,
    ) -> Result<Vec<AssetData>, String> {
        let provider = self
            .get_or_create(id, db)
            .await
            .ok_or_else(|| format!("找不到數據源: {}", id))?;
        let limiter = self.get_limiter(id).await;
        let _permit = limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter: {}", e))?;
        provider.fetch_prices(symbols).await
    }

    /// 更新已有的 provider instance（例如 API key 變更後）
    pub async fn update_provider(
        &self,
        id: &str,
        api_key: Option<String>,
        api_secret: Option<String>,
        api_url: Option<String>,
    ) {
        let has_key = api_key.is_some();
        if let Some(provider) = create_provider_with_url(id, api_key, api_secret, api_url) {
            self.providers
                .write()
                .await
                .insert(id.to_string(), provider);
            self.ensure_limiter(id, has_key).await;
        }
    }

    /// 移除 provider instance（例如 API key 被刪除）
    #[allow(dead_code)]
    pub async fn remove_provider(&self, id: &str) {
        self.providers.write().await.remove(id);
        self.limiters.write().await.remove(id);
    }

    /// 取得 rate limiter（如果不存在則建立默認的）
    async fn get_limiter(&self, id: &str) -> Arc<Semaphore> {
        {
            let limiters = self.limiters.read().await;
            if let Some(limiter) = limiters.get(id) {
                return limiter.clone();
            }
        }
        // 不存在則建立默認的
        let limiter = Arc::new(Semaphore::new(DEFAULT_CONCURRENT_REQUESTS));
        self.limiters
            .write()
            .await
            .insert(id.to_string(), limiter.clone());
        limiter
    }

    /// 確保有對應的 rate limiter
    async fn ensure_limiter(&self, id: &str, has_key: bool) {
        let limit = if has_key {
            KEYED_CONCURRENT_REQUESTS
        } else {
            DEFAULT_CONCURRENT_REQUESTS
        };
        let mut limiters = self.limiters.write().await;
        if !limiters.contains_key(id) {
            limiters.insert(id.to_string(), Arc::new(Semaphore::new(limit)));
        }
    }
}

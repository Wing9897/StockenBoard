# Request Logic & Data Flow

StockenBoard 的資料請求架構，從 app 啟動到價格顯示在卡片上的完整流程。

## 架構概覽

```
┌─────────────────────────────────────────────────────────────┐
│  Frontend (React + TypeScript)                              │
│                                                             │
│  App.tsx                                                    │
│    ├── useAssetData() hook                                  │
│    │     ├── PriceStore (singleton, key-based subscription) │
│    │     ├── listen("price-update")                         │
│    │     ├── listen("price-error")                          │
│    │     ├── listen("poll-tick")                             │
│    │     └── listen("ws-ticker-update")                     │
│    ├── useViews() hook                                      │
│    └── set_visible_subscriptions(ids) ──────────────┐       │
│                                                     │       │
│  AssetCard                                          │       │
│    └── useAssetPrice(symbol, providerId) ◄── PriceStore     │
│                                                     │       │
└─────────────────────────────────────────────────────│───────┘
                                                      │
                        Tauri IPC (invoke / emit)     │
                                                      │
┌─────────────────────────────────────────────────────│───────┐
│  Backend (Rust + Tauri 2)                           │       │
│                                                     ▼       │
│  commands.rs                                                │
│    ├── set_visible_subscriptions() → PollingManager          │
│    ├── reload_polling() → PollingManager.reload()           │
│    ├── get_cached_prices() → PollingManager.cache           │
│    ├── get_poll_ticks() → PollingManager.ticks              │
│    ├── fetch_asset_price() (on-demand 驗證用)               │
│    └── enable/disable_provider()                            │
│                                                             │
│  polling.rs — PollingManager                                │
│    ├── cache: RwLock<HashMap<"pid:symbol", AssetData>>      │
│    ├── ticks: RwLock<HashMap<"pid", PollTick>>              │
│    ├── visible_ids: RwLock<HashMap<window_id, HashSet<id>>> │
│    └── start() → 主循環 (spawn per-provider polling tasks)  │
│                                                             │
│  providers/*.rs — 31 個 DataProvider 實現                    │
│    └── fetch_prices(&[String]) → Vec<AssetData>             │
│                                                             │
│  db.rs — SQLite (stockenboard.db)                           │
│    ├── subscriptions                                        │
│    ├── provider_settings                                    │
│    ├── views                                                │
│    └── view_subscriptions                                   │
└─────────────────────────────────────────────────────────────┘
```

## 啟動流程

### 1. Backend 啟動 (`lib.rs`)

```
tauri::Builder
  ├── 1. 註冊 SQL plugin + 執行 migration (db.rs MIGRATION_V1)
  ├── 2. manage(AppState::new())  ← 建立 PollingManager
  ├── 3. invoke_handler(所有 commands)
  └── 4. setup() → polling.start(app_handle, db_path)
```

### 2. PollingManager 啟動 (`polling.rs`)

```
PollingManager::start()
  └── tauri::async_runtime::spawn(主循環)
       ├── 讀取 visible_ids 快照
       ├── spawn_blocking(load_config(db_path, visible_ids))
       │    ├── SELECT subscriptions (按 visible_ids 過濾)
       │    ├── SELECT provider_settings
       │    ├── 跳過 enabled=0 的 provider
       │    ├── 按 provider_id 分組 → PollingGroup { symbols, interval_ms }
       │    └── create_provider() 建立實例
       ├── 清理快取（移除已不存在的 subscription）
       └── 為每個 provider group 啟動 polling task
```

### 3. Frontend 啟動 (`useAssetData.ts`)

```
useAssetData() init
  ├── 1. loadProviderInfo()     ← invoke('get_all_providers')
  ├── 2. loadSubscriptions()    ← SQL SELECT subscriptions
  ├── 3. setupPriceListener()   ← listen("price-update")
  ├── 4. setupErrorListener()   ← listen("price-error")
  ├── 5. setupTickListener()    ← listen("poll-tick")
  ├── 6. loadCachedPrices()     ← invoke('get_cached_prices')
  ├── 7. loadCachedTicks()      ← invoke('get_poll_ticks')
  └── 8. startWsConnections()   ← invoke('start_ws_stream') (如有 WS provider)
```

### 4. 頁面可見性通知 (`App.tsx`)

```
useEffect([viewFilteredSubs])
  └── invoke('set_visible_subscriptions', { ids })
       └── PollingManager.set_visible(window_id, id_set)
            ├── 比較新舊值，無變化則跳過
            └── 有變化 → reload_tx.send() → 觸發主循環重新載入
```

## 定時 Polling 流程

```
Per-provider polling task (tokio::spawn)
  loop {
    ├── provider.fetch_prices(&symbols)
    │    ├── 成功 → cache.write().insert() + emit("price-update", results)
    │    └── 失敗 → emit("price-error", { "pid:symbol": error_msg })
    ├── 建立 PollTick { provider_id, fetched_at, interval_ms }
    ├── ticks.write().insert(pid, tick)
    ├── emit("poll-tick", tick)
    └── tokio::select! {
          sleep(interval_ms) → 繼續下一輪,
          gen_stop.changed() → break (reload 或停止)
        }
  }
```

## 前端資料接收流程

### REST Polling 路徑

```
Backend emit("price-update", Vec<AssetData>)
  → Frontend listen("price-update")
    → PriceStore.updatePrices(results)
      → 對每個 AssetData:
         key = "providerId:symbol"
         比較 prev.price / prev.last_updated
         有變化 → notifyKey(key)
           → 只有訂閱了該 key 的 AssetCard re-render
```

### WebSocket 路徑

```
Backend emit("ws-ticker-update", WsTickerUpdate)
  → Frontend listen("ws-ticker-update")
    → PriceStore.updateWs(providerId, symbol, data)
      → notifyKey("providerId:symbol")
        → 對應的 AssetCard re-render
```

### 倒計時同步路徑

```
Backend emit("poll-tick", PollTick)
  → Frontend listen("poll-tick")
    → PriceStore.updateTick(providerId, fetchedAt, intervalMs)
      → notifyTick(providerId)
        → CountdownCircle 使用 usePollTick(providerId) 接收
          → 計算剩餘時間 = intervalMs - (now - fetchedAt)
```

## Reload 觸發時機

以下操作會觸發 `PollingManager.reload()`，使主循環重新讀取 DB 並重建 polling tasks：

| 觸發來源 | 說明 |
|---|---|
| `set_visible_subscriptions` | 前端切換頁面，可見 subscription IDs 變化 |
| `reload_polling` | 前端手動觸發（updateSubscription 改了 symbol/provider 時） |
| `enable_provider` | 啟用 provider |
| `disable_provider` | 停用 provider |

### Reload 流程

```
reload_tx.send_modify(+1)
  → 主循環 tokio::select! 收到 reload_rx.changed()
    → drop(gen_stop_tx) → 所有 per-provider tasks 收到 stop 信號 → break
    → 所有 handles.abort()
    → 重新進入主循環頂部
      → 重新讀取 visible_ids
      → 重新 load_config()
      → 重新建立 polling tasks
```

## Interval 決定邏輯

```
1. 檢查 provider_settings.refresh_interval（用戶自訂）
   ├── 有值 → 使用該值
   └── 無值 → 進入步驟 2

2. 檢查 provider 是否有 API key
   ├── 有 key → 使用 ProviderInfo.key_interval
   └── 無 key → 使用 ProviderInfo.free_interval
```

各 provider 的預設 interval 定義在 `traits.rs` 的 `build_all_provider_info()` 中。

## Provider 實例化

```
create_provider(id, api_key, api_secret) → Option<Arc<dyn DataProvider>>
  ├── match id {
  │     "binance" → BinanceProvider::new(api_key),
  │     "coinbase" → CoinbaseProvider::new(),
  │     ...31 個 provider
  │   }
  └── 所有 provider 共用 shared_client() (reqwest::Client 全局單例)
```

## 資料庫 Schema

```sql
-- 訂閱
subscriptions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  symbol TEXT NOT NULL UNIQUE,
  display_name TEXT,
  selected_provider_id TEXT NOT NULL DEFAULT 'binance',
  asset_type TEXT NOT NULL DEFAULT 'crypto',
  sort_order INTEGER NOT NULL DEFAULT 0
)

-- Provider 設定
provider_settings (
  provider_id TEXT PRIMARY KEY,
  api_key TEXT,
  api_secret TEXT,
  refresh_interval INTEGER,
  connection_type TEXT NOT NULL DEFAULT 'rest',
  enabled INTEGER NOT NULL DEFAULT 1
)

-- 自訂頁面
views (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  is_default INTEGER NOT NULL DEFAULT 0
)

-- 頁面-訂閱關聯
view_subscriptions (
  view_id INTEGER NOT NULL,
  subscription_id INTEGER NOT NULL,
  PRIMARY KEY (view_id, subscription_id)
)
```

## PriceStore 架構（前端 singleton）

```
PriceStore
  ├── assets: Map<"pid:symbol", AssetData>
  ├── errors: Map<"pid:symbol", string>
  ├── ticks: Map<"pid", { fetchedAt, intervalMs }>
  ├── keyListeners: Map<"pid:symbol", Set<callback>>  ← O(1) 通知
  └── tickListeners: Map<"pid", Set<callback>>

使用方式：
  AssetCard → useAssetPrice(symbol, pid)
    → subscribeKey("pid:symbol", rerender)
    → 只在自己的 key 變化時 re-render

  CountdownCircle → usePollTick(pid)
    → subscribeTick(pid, rerender)
    → 只在對應 provider 的 tick 變化時 re-render
```

## 頁面感知 Polling（Page-Aware）

```
前端切換頁面 / 訂閱變化
  → viewFilteredSubs 變化
    → useEffect 計算可見 IDs
      → invoke('set_visible_subscriptions', { ids })
        → PollingManager.set_visible()
          → 比較新舊值
            ├── 相同 → 跳過
            └── 不同 → reload
              → load_config() 只 SELECT 可見的 subscription IDs
                → 只為可見的 symbol 建立 polling tasks
                  → 不可見的 provider 不會發送任何 API 請求
```

## 錯誤處理

| 層級 | 處理方式 |
|---|---|
| Provider fetch 失敗 | emit("price-error") → PriceStore.updateErrors() → AssetCard 顯示「獲取失敗」 |
| DB 讀取失敗 | eprintln + sleep 5s → 重試 |
| spawn_blocking 失敗 | eprintln + sleep 5s → 重試 |
| 前端 invoke 失敗 | console.error（不中斷 UI） |
| WS 連接失敗 | console.error（REST polling 繼續運作） |

# AssetData 標準欄位對照表

所有 provider 回傳的資料都統一為 `AssetData` 結構，以下列出每個 provider 實際提供的標準欄位。

## AssetData 結構定義

```rust
pub struct AssetData {
    pub symbol: String,           // 代號
    pub price: f64,               // 目前價格
    pub currency: String,         // 計價幣種 (USD, USDT, PROB 等)
    pub change_24h: Option<f64>,  // 24小時價格變動 (絕對值)
    pub change_percent_24h: Option<f64>, // 24小時變動百分比
    pub high_24h: Option<f64>,    // 24小時最高價
    pub low_24h: Option<f64>,     // 24小時最低價
    pub volume: Option<f64>,      // 成交量
    pub market_cap: Option<f64>,  // 市值
    pub last_updated: i64,        // 最後更新時間 (ms timestamp)
    pub provider_id: String,      // 數據源 ID
    pub extra: Option<HashMap<String, Value>>, // 額外欄位 (provider 特有)
}
```

## 欄位覆蓋率統計

| 欄位 | 有提供的 provider 數 | 覆蓋率 |
|---|:---:|:---:|
| price | 33/33 | 100% |
| volume | 26/33 | 79% |
| change_percent_24h | 27/33 | 82% |
| change_24h | 25/33 | 76% |
| high_24h | 21/33 | 64% |
| low_24h | 21/33 | 64% |
| market_cap | 6/33 | 18% |

## 完整對照表

| # | Provider | 類型 | price | currency | change_24h | change_%_24h | high_24h | low_24h | volume | market_cap |
|:---:|---|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | Binance | crypto | ✅ | USDT | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 2 | Coinbase | crypto | ✅ | USD | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 3 | CoinGecko | crypto | ✅ | USD | ❌ | ✅ | ❌ | ❌ | ✅ | ✅ |
| 4 | CoinMarketCap | crypto | ✅ | USD | ❌ | ✅ | ❌ | ❌ | ✅ | ✅ |
| 5 | CryptoCompare | crypto | ✅ | USD | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 6 | Yahoo Finance | stock | ✅ | 動態 | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 7 | Finnhub | both | ✅ | USD | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| 8 | Alpha Vantage | both | ✅ | USD | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 9 | Polygon.io | both | ✅ | USD | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 10 | Twelve Data | both | ✅ | 動態 | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 11 | Alpaca | both | ✅ | USD | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 12 | Tiingo (股票) | both | ✅ | USD | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 13 | Tiingo (加密) | both | ✅ | USD | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 14 | FMP | both | ✅ | USD | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 15 | Marketstack | stock | ✅ | USD | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 16 | EODHD | stock | ✅ | USD | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 17 | Mboum | stock | ✅ | 動態 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 18 | Polymarket | prediction | ✅ | PROB | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| 19 | Bitquery | prediction | ✅ | USD | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| 20 | Kraken | crypto | ✅ | USD | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 21 | Bybit | crypto | ✅ | USDT | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 22 | KuCoin | crypto | ✅ | USDT | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 23 | OKX | crypto | ✅ | USDT | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 24 | Gate.io | crypto | ✅ | USDT | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 25 | Bitfinex | crypto | ✅ | USD | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 26 | HTX (Huobi) | crypto | ✅ | USDT | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 27 | MEXC | crypto | ✅ | USDT | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 28 | CoinPaprika | crypto | ✅ | USD | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| 29 | CoinAPI | both | ✅ | USD | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 30 | FCS API | both | ✅ | USD | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 31 | Jupiter | dex | ✅ | USD | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| 32 | OKX DEX | dex | ✅ | USD | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 33 | Raydium | dex | ✅ | USD | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 34 | Subgraph | dex | ✅ | USD | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

> 注意：Tiingo 根據查詢的是股票還是加密貨幣，回傳的欄位不同，因此分兩行列出。
> Raydium 和 Subgraph 為 DEX Pool Provider，主要透過 extra 欄位提供 pool_tvl、route_path 等資訊。

## 分類

### 通用欄位（建議 Card 永遠顯示）
- `price` — 100% 覆蓋
- `change_percent_24h` — 82% 覆蓋
- `currency` — 100% 覆蓋（但值不同：USD / USDT / PROB / 動態）

### 高覆蓋欄位（建議 Card 預設顯示，無資料時隱藏）
- `volume` — 79%
- `change_24h` — 76%
- `high_24h` / `low_24h` — 64%

### 低覆蓋欄位（建議放在展開區域）
- `market_cap` — 18%（僅 CoinGecko, CoinMarketCap, CryptoCompare, FMP, Mboum, CoinPaprika）

## 缺少關鍵欄位的 Provider

| Provider | 缺少的欄位 | 說明 |
|---|---|---|
| Coinbase | change_24h, change_%_24h, high/low, volume | 僅提供即時價格 |
| CoinAPI | change_24h, change_%_24h, high/low, volume | 僅提供即時匯率 |
| Tiingo (加密) | change_24h, change_%_24h, high/low, volume | 加密端點僅回傳最新價 |
| CoinGecko | change_24h, high/low | 有 change_%_24h 但無絕對值 |
| CoinMarketCap | change_24h, high/low | 同上 |
| Polymarket | change_24h, change_%_24h, high/low | 預測市場，概念不同 |
| Bitquery | change_24h, change_%_24h, high/low | DEX 聚合，僅提供價格和成交量 |

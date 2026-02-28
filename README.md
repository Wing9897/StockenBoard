# StockenBoard

即時股票與加密貨幣看板，支援多數據源切換、自訂頁面分類、批量管理訂閱。

## 功能

- 即時價格追蹤（REST + WebSocket）
- HTTP API（端口 8080）- 供 AI 或外部程式訪問數據
- 33 個數據源
  - 加密貨幣交易所：Binance、Coinbase、Kraken、Bybit、KuCoin、OKX、Gate.io、Bitfinex、HTX、MEXC
  - 加密貨幣聚合器：CoinGecko、CoinMarketCap、CoinPaprika、CryptoCompare
  - 股票 / 多資產：Yahoo Finance、Finnhub、Alpha Vantage、Polygon、Twelve Data、Alpaca、Tiingo、FMP、Marketstack、EODHD、Mboum、FCS API
  - 多資產聚合器：CoinAPI
  - 預測市場：Polymarket、Bitquery
  - DEX 聚合器：Jupiter、OKX DEX、Raydium、Subgraph
- 後端統一 Polling 架構（Rust 端定時 fetch，前端零 setInterval）
- 自訂頁面分類與置頂
- Grid / List / Compact 三種視圖模式
- 每張卡片獨立切換數據源
- 資料匯出 / 匯入（JSON）
- 批量新增與刪除訂閱
- 自訂資產圖示
- Toast 通知系統

## HTTP API

StockenBoard 提供 HTTP API 讓外部程式（如 AI、Python 腳本）訪問數據。

**API 地址**: `http://localhost:8080/api`

### 端點

- `GET /api/status` - 系統狀態
- `GET /api/subscriptions` - 所有訂閱
- `GET /api/prices` - 所有最新價格
- `GET /api/prices/{provider}/{symbol}` - 特定價格
- `GET /api/history?symbol=&provider=&from=&to=&limit=` - 歷史數據

### 使用範例

```python
import requests

# 獲取所有價格
prices = requests.get("http://localhost:8080/api/prices").json()
for p in prices['prices']:
    print(f"{p['symbol']}: ${p['price']}")

# 獲取歷史數據
history = requests.get("http://localhost:8080/api/history", params={
    "symbol": "BTCUSDT",
    "provider": "binance",
    "limit": 1000
}).json()
```

詳細說明請參考應用程式內的「設定 → API 使用說明」。

## 技術棧

- **前端**：React 19 + TypeScript 5.8 + Vite 7
- **後端**：Tauri 2 + Rust
- **資料庫**：SQLite（tauri-plugin-sql）
- **API**：Axum + Tower
- **主題**：Catppuccin Mocha

## 架構

- 後端 `PollingManager` 統一管理所有 provider 的定時 fetch，透過 Tauri event 推送價格更新
- 前端 `PriceStore` 單例 + 細粒度訂閱，每張卡片只在自己的價格變化時 re-render
- `poll-tick` 事件驅動倒計時圓圈，精確同步後端 fetch 時間
- 共用 `getDb()` 單例，全局共享 SQLite 連接
- HTTP API Server 自動啟動，提供 RESTful 接口

## 開發

```bash
# 安裝前端依賴
npm install

# 啟動開發模式
npm run tauri dev

# 建置
npm run tauri build

# 測試 API
python test_api.py
```

## 授權

MIT

## 免責聲明

本軟體提供的所有市場數據僅供參考，不構成任何投資建議。數據可能存在延遲、不準確或不完整的情況。使用者應自行判斷並承擔所有投資風險。

All market data provided by this software is for informational purposes only and does not constitute investment advice. Data may be delayed, inaccurate, or incomplete. Users should exercise their own judgment and assume all investment risks.

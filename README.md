# StockenBoard

即時股票與加密貨幣看板，支援多數據源切換、自訂頁面分類、批量管理訂閱。

## 功能

- 即時價格追蹤（REST + WebSocket）
- 29 個數據源
  - 加密貨幣交易所：Binance、Coinbase、Kraken、Bybit、KuCoin、OKX、Gate.io、Bitfinex、HTX、MEXC
  - 加密貨幣聚合器：CoinGecko、CoinMarketCap、CoinPaprika、CryptoCompare
  - 股票 / 多資產：Yahoo Finance、Finnhub、Alpha Vantage、Polygon、Twelve Data、Alpaca、Tiingo、FMP、Marketstack、EODHD、Mboum、FCS API
  - 多資產聚合器：CoinAPI
  - 預測市場：Polymarket、Bitquery
- 後端統一 Polling 架構（Rust 端定時 fetch，前端零 setInterval）
- 自訂頁面分類與置頂
- Grid / List / Compact 三種視圖模式
- 每張卡片獨立切換數據源
- 資料匯出 / 匯入（JSON）
- 批量新增與刪除訂閱
- 自訂資產圖示
- Toast 通知系統

## 技術棧

- **前端**：React 19 + TypeScript 5.8 + Vite 7
- **後端**：Tauri 2 + Rust
- **資料庫**：SQLite（tauri-plugin-sql）
- **主題**：Catppuccin Mocha

## 架構

- 後端 `PollingManager` 統一管理所有 provider 的定時 fetch，透過 Tauri event 推送價格更新
- 前端 `PriceStore` 單例 + 細粒度訂閱，每張卡片只在自己的價格變化時 re-render
- `poll-tick` 事件驅動倒計時圓圈，精確同步後端 fetch 時間
- 共用 `getDb()` 單例，全局共享 SQLite 連接

## 開發

```bash
# 安裝前端依賴
npm install

# 啟動開發模式
npm run tauri dev

# 建置
npm run tauri build
```

## 授權

MIT

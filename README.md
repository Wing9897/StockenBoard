# StockenBoard

即時股票與加密貨幣看板，支援多數據源切換、自訂頁面分類、批量管理訂閱。

![Dashboard](dashboard.png)

## 功能

- 即時價格追蹤（REST + WebSocket）
- 18 個數據源（Binance、Yahoo Finance、CoinGecko、Polygon 等）
- 自訂頁面分類與置頂
- Grid / List 雙視圖切換
- 每張卡片獨立切換數據源
- 資料匯出 / 匯入（JSON）
- 批量新增與刪除訂閱
- Toast 通知系統

## 技術棧

- **前端**：React 19 + TypeScript 5.8 + Vite
- **後端**：Tauri 2 + Rust
- **資料庫**：SQLite（tauri-plugin-sql）
- **主題**：Catppuccin Mocha

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

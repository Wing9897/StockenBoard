# StockenBoard

<div align="center">

**即時股票與加密貨幣看板**

支援 33 個數據源 | 多頁面管理 | HTTP API | AI 智能通知 | 獨立 Web Server 模式

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://github.com/nicekid1/stockenboard/actions/workflows/release.yml/badge.svg)](https://github.com/nicekid1/stockenboard/actions/workflows/release.yml)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB?logo=tauri)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react)](https://react.dev/)
[![Rust](https://img.shields.io/badge/Rust-1.93-orange?logo=rust)](https://www.rust-lang.org/)

[English](README.en.md) | 繁體中文 | [简体中文](README.zh-CN.md) | [日本語](README.ja.md) | [한국어](README.ko.md)

</div>

---

## ✨ 功能特色

### 📊 數據源支援（33 個）

<details>
<summary><b>加密貨幣交易所（10 個）</b></summary>

- Binance、Coinbase、Kraken、Bybit、KuCoin
- OKX、Gate.io、Bitfinex、HTX、MEXC

</details>

<details>
<summary><b>加密貨幣聚合器（4 個）</b></summary>

- CoinGecko、CoinMarketCap、CoinPaprika、CryptoCompare

</details>

<details>
<summary><b>股票 / 多資產（12 個）</b></summary>

- Yahoo Finance、Finnhub、Alpha Vantage、Polygon
- Twelve Data、Alpaca、Tiingo、FMP
- Marketstack、EODHD、Mboum、FCS API

</details>

<details>
<summary><b>DEX 聚合器（4 個）</b></summary>

- Jupiter（Solana）
- OKX DEX（多鏈）
- Raydium（Solana）
- Subgraph（Uniswap/Sushi/Pancake）

</details>

<details>
<summary><b>預測市場（2 個）</b></summary>

- Polymarket、Bitquery

</details>

<details>
<summary><b>多資產聚合器（1 個）</b></summary>

- CoinAPI

</details>

### 🎯 核心功能

- ⚡ **即時價格追蹤** - REST + WebSocket 雙模式
- 🔌 **HTTP API** - 供 AI 或外部程式訪問（可配置端口）
- 📈 **歷史數據記錄** - 自動記錄價格變化，支援圖表分析
- 🎨 **多視圖模式** - Grid / List / Compact 三種顯示方式
- 📁 **自訂頁面** - 建立多個頁面分類管理訂閱
- 🔄 **批量操作** - 批量新增、刪除、匯出、匯入訂閱
- 🎭 **自訂外觀** - 14 種主題、自訂圖示、背景圖片
- 🌍 **多語言** - 繁中、簡中、英文、日文、韓文
- 🔔 **推播通知** - 條件觸發 + AI 智能評估，支援 Telegram Bot 及 Webhook，全局冷卻期防止過度通知

### 🏗️ 技術架構

- **雙模式運行** - Tauri 桌面版 + Standalone Web Server（Docker / 原生）
- **後端統一 Polling** - Rust 端定時 fetch + 指數退避，前端零 setInterval
- **細粒度訂閱** - 每張卡片只在自己的價格變化時 re-render
- **頁面感知** - 只為可見訂閱發送 API 請求，節省資源
- **單例資料庫** - 全局共享 SQLite 連接，避免鎖定問題
- **AI 通知引擎** - 支援本地 Ollama 或 OpenAI-compatible API，定期分析價格歷史

---

## 🚀 快速開始

### 桌面版安裝

從 [Releases](https://github.com/nicekid1/StockenBoard/releases) 下載最新版本（Windows / macOS / Linux）。

### 🌐 Web Server 模式（無需 GUI）

StockenBoard 支援獨立 Web Server 模式，無需桌面環境即可運行：

```bash
# 使用 Docker
docker run -p 8080:8080 -v stockenboard-data:/data ghcr.io/nicekid1/stockenboard:latest

# 或直接執行二進位檔
SB_PORT=8080 SB_DATA_DIR=./data ./stockenboard-server
```

**環境變數：**
| 變數 | 預設值 | 說明 |
|------|--------|------|
| `SB_BIND` | `0.0.0.0` | 綁定地址 |
| `SB_PORT` | `8080` | HTTP 端口 |
| `SB_DATA_DIR` | `./data` | 資料目錄 |
| `SB_STATIC_DIR` | `./static` | 前端靜態檔案路徑 |

### 開發

```bash
# 安裝依賴
npm install

# 啟動桌面開發模式
npm run tauri dev

# 啟動 Web Server 開發模式
cd src-tauri && cargo run --no-default-features --features server --bin server

# 建置桌面版
npm run tauri build

# 建置 Web Server
cd src-tauri && cargo build --no-default-features --features server --release --bin server
```

---

## 🔌 HTTP API

StockenBoard 提供完整 REST API，供 AI、自動化腳本或第三方應用程式存取數據。

### 核心端點

| 端點 | 方法 | 說明 |
|------|------|------|
| `/api/system/config` | GET | 系統配置 |
| `/api/subscriptions` | GET/POST | 訂閱列表 / 新增訂閱 |
| `/api/subscriptions/{id}` | PUT/DELETE | 更新 / 刪除訂閱 |
| `/api/prices/cached` | GET | 所有最新快取價格 |
| `/api/prices/fetch/{provider}/{symbol}` | GET | 即時抓取特定價格 |
| `/api/history/{sub_id}` | GET | 歷史數據查詢 |
| `/api/providers` | GET | 所有數據源資訊 |
| `/api/providers/{id}` | PUT | 更新數據源設定 |
| `/api/notifications/rules` | GET/POST | 通知規則 CRUD |
| `/api/notifications/channels` | GET/POST | 通知通道管理 |
| `/api/notifications/history` | GET | 通知觸發歷史 |
| `/api/ai/config` | GET/POST | AI Provider 設定 |
| `/api/ai/test` | POST | 測試 AI 連線 |
| `/api/ws` | WebSocket | 即時價格推播 |

### 快速範例

```python
import requests

BASE = "http://localhost:8080"

# 獲取所有快取價格
prices = requests.get(f"{BASE}/api/prices/cached").json()
for key, p in prices['data'].items():
    print(f"{p['symbol']}: ${p['price']}")

# 新增訂閱
requests.post(f"{BASE}/api/subscriptions", json={
    "symbol": "ETH", "provider_id": "binance",
    "asset_type": "crypto", "sub_type": "asset"
})

# 設定 AI 通知（搭配本地 Ollama）
requests.post(f"{BASE}/api/ai/config", json={
    "base_url": "http://localhost:11434/v1",
    "model": "qwen2.5:3b",
    "api_key": None
})
```

> 💡 **提示**：桌面版 API 端口可在設定頁面修改。Web Server 模式透過 `SB_PORT` 環境變數設定。

---

## 🛠️ 技術棧

| 類別 | 技術 |
|------|------|
| **前端** | React 19 + TypeScript 5.8 + Vite 7 |
| **後端** | Tauri 2 + Rust 1.93 |
| **資料庫** | SQLite (rusqlite) |
| **Web API** | Axum 0.7 + Tower + WebSocket |
| **AI 整合** | OpenAI-compatible API（支援 Ollama 本地部署）|
| **圖表** | lightweight-charts 5.1 |
| **主題** | Catppuccin Mocha |
| **部署** | Docker / 原生二進位 |

---

## 📦 專案結構

```
StockenBoard/
├── src/                    # 前端代碼
│   ├── components/         # React 組件
│   ├── hooks/              # 自訂 Hooks
│   ├── lib/                # 工具函數、Transport 抽象層、i18n
│   └── types/              # TypeScript 類型定義
├── src-tauri/              # 後端代碼
│   └── src/
│       ├── api/            # HTTP API (Axum 路由)
│       ├── commands/       # Tauri IPC 指令（按領域拆分）
│       ├── db/             # 資料庫存取層（按領域拆分）
│       ├── notifications/  # 推播通知系統（引擎、AI 排程、派發）
│       ├── providers/      # 33 個數據源實現
│       ├── bin/server.rs   # Standalone Web Server 入口
│       ├── core_state.rs   # 共享核心狀態
│       ├── icons.rs        # Logo 批量下載邏輯
│       └── polling.rs      # 統一 Polling + 指數退避
└── check_i18n.js           # i18n 完整性檢查
```

---

## 🎨 截圖

### 主頁面 - Grid 視圖
![主頁面 1](doc/main_page1.png)

### 主頁面 - List 視圖
![主頁面 2](doc/main_page2.png)

### DEX 頁面
![DEX 頁面](doc/dex.png)

### 歷史記錄與圖表
![歷史記錄](doc/records.png)

### 數據源設定
![數據源設定](doc/providers.png)

### 設定頁面
![設定頁面](doc/setting_page.png)

---

## 📝 授權

本專案採用 [MIT License](LICENSE) 授權。

---

## ⚠️ 免責聲明

本軟體提供的所有市場數據僅供參考，不構成任何投資建議。數據可能存在延遲、不準確或不完整的情況。使用者應自行判斷並承擔所有投資風險。

All market data provided by this software is for informational purposes only and does not constitute investment advice. Data may be delayed, inaccurate, or incomplete. Users should exercise their own judgment and assume all investment risks.

---

<div align="center">

**[⭐ Star this project](https://github.com/nicekid1/StockenBoard)** if you find it useful!

Made with ❤️ using Tauri + React + Rust

</div>

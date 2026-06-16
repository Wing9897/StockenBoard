# StockenBoard

<div align="center">

**Real-time Stock & Cryptocurrency Dashboard**

33 Data Sources | AI Notifications | Desktop & Web Server | Docker | Historical Charts

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://github.com/Wing9897/stockenboard/actions/workflows/release.yml/badge.svg)](https://github.com/Wing9897/stockenboard/actions/workflows/release.yml)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB?logo=tauri)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react)](https://react.dev/)
[![Rust](https://img.shields.io/badge/Rust-1.93-orange?logo=rust)](https://www.rust-lang.org/)

English | [繁體中文](README.md) | [简体中文](README.zh-CN.md) | [日本語](README.ja.md) | [한국어](README.ko.md)

</div>

---

## ✨ Features

### 📊 Data Sources (33 Total)

<details>
<summary><b>Cryptocurrency Exchanges (10)</b></summary>

- Binance, Coinbase, Kraken, Bybit, KuCoin
- OKX, Gate.io, Bitfinex, HTX, MEXC

</details>

<details>
<summary><b>Crypto Aggregators (4)</b></summary>

- CoinGecko, CoinMarketCap, CoinPaprika, CryptoCompare

</details>

<details>
<summary><b>Stock / Multi-Asset (12)</b></summary>

- Yahoo Finance, Finnhub, Alpha Vantage, Polygon
- Twelve Data, Alpaca, Tiingo, FMP
- Marketstack, EODHD, Mboum, FCS API

</details>

<details>
<summary><b>DEX Aggregators (4)</b></summary>

- Jupiter (Solana)
- OKX DEX (Multi-chain)
- Raydium (Solana)
- Subgraph (Uniswap/Sushi/Pancake)

</details>

<details>
<summary><b>Prediction Markets (2)</b></summary>

- Polymarket, Bitquery

</details>

<details>
<summary><b>Multi-Asset Aggregators (1)</b></summary>

- CoinAPI

</details>

### 🎯 Core Features

- ⚡ **Real-time Price Tracking** - REST + WebSocket dual mode
- 🔌 **HTTP API** - For AI or external programs (configurable port)
- 📈 **Historical Data Recording** - Auto-record price changes with chart analysis
- 🎨 **Multiple View Modes** - Grid / List / Compact display options
- 📁 **Custom Pages** - Create multiple pages to organize subscriptions
- 🔄 **Batch Operations** - Bulk add, delete, export, import subscriptions
- 🎭 **Customizable Appearance** - 14 themes, custom icons, background images
- 🌍 **Multi-language** - Traditional Chinese, Simplified Chinese, English, Japanese, Korean
- 🔔 **Push Notifications** - Condition-based + AI evaluation, supports Telegram Bot & Webhook, global cooldown to prevent over-notification

### 🏗️ Technical Architecture

- **Unified Backend Polling** - Rust-side scheduled fetch, zero setInterval on frontend
- **Fine-grained Subscriptions** - Each card re-renders only when its own price changes
- **Page-aware** - API requests only for visible subscriptions, saving resources
- **Singleton Database** - Global shared SQLite connection, avoiding lock issues

---

## 🚀 Quick Start

### Installation

Download the latest version from [Releases](https://github.com/Wing9897/StockenBoard/releases):
- **Desktop**: Download the installer for your OS (Windows/macOS/Linux)
- **Web Server**: Download `stockenboard-server-*` binary
- **Docker**: `docker run -d -p 8080:8080 -v stockenboard-data:/data ghcr.io/wing9897/stockenboard:latest`

### Development

```bash
# Install dependencies
npm install

# Start desktop development mode
npm run dev:desktop

# Start web server development mode
npm run dev:server

# Build desktop
npm run build:desktop

# Build web server (cross-platform)
npm run build:server              # current platform
npm run build:server:linux-x64    # Linux x86_64
npm run build:server:mac-arm64    # macOS ARM64
npm run build:server:win-x64      # Windows x86_64

# Run all tests
npm run test:all
```

---

## 🔌 HTTP API

StockenBoard provides a full REST API + WebSocket for AI, automation scripts, or third-party applications.

### Main Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/prices/cached` | All latest cached prices |
| `GET /api/subscriptions` | All subscriptions |
| `POST /api/subscriptions` | Add subscription |
| `GET /api/history/{id}` | Price history for a subscription |
| `POST /api/history/cleanup` | Clean up old history records |
| `GET /api/notifications/rules` | Notification rules |
| `POST /api/ai/config` | Configure AI provider |
| `POST /api/ai/test` | Test AI connection |
| `GET /api/ws` | WebSocket real-time updates |

### Quick Example

```python
import requests
BASE = "http://localhost:8080"

# Get all cached prices
prices = requests.get(f"{BASE}/api/prices/cached").json()

# Add a subscription
requests.post(f"{BASE}/api/subscriptions", json={
    "symbol": "ETH", "provider_id": "binance",
    "asset_type": "crypto", "sub_type": "asset"
})

# Configure AI notifications (with Ollama)
requests.post(f"{BASE}/api/ai/config", json={
    "base_url": "http://localhost:11434/v1",
    "model": "qwen2.5:7b", "api_key": None
})
```

> 💡 **Tip**: API port can be modified in "Settings → API Guide". See that page for detailed documentation and more examples.

---

## 🛠️ Tech Stack

| Category | Technology |
|----------|------------|
| **Frontend** | React 19 + TypeScript 5.8 + Vite 7 |
| **Backend** | Tauri 2 + Rust 1.93 |
| **Database** | SQLite (rusqlite) |
| **API** | Axum 0.7 + Tower |
| **Charts** | lightweight-charts 5.1 |
| **AI** | OpenAI-compatible API (Ollama, OpenAI, OpenRouter) |
| **Theme** | Catppuccin Mocha |
| **Deploy** | Desktop (Tauri) / Web Server / Docker |

---

## 📦 Project Structure

```
StockenBoard/
├── src/                    # Frontend (React + TypeScript)
│   ├── components/         # React components
│   ├── hooks/              # Custom hooks
│   ├── lib/                # Utilities, i18n, transport layer
│   └── types/              # TypeScript type definitions
├── src-tauri/              # Backend (Rust)
│   └── src/
│       ├── api/            # HTTP REST API (Axum routes)
│       ├── commands/       # Tauri IPC commands
│       ├── db/             # SQLite database layer
│       ├── notifications/  # AI evaluator, engine, scheduler, Telegram/Webhook
│       ├── providers/      # 33 data source implementations
│       ├── bin/server.rs   # Standalone web server entry point
│       ├── polling.rs      # Unified polling manager
│       └── core_state.rs   # Shared application state
├── scripts/                # Build & utility scripts
├── data/                   # Runtime data (DB, icons)
└── .github/workflows/      # CI/CD (cross-platform builds)
```

---

## 🎨 Screenshots

### Main Page - Grid View
![Main Page 1](doc/main_page1.png)

### Main Page - List View
![Main Page 2](doc/main_page2.png)

### DEX Page
![DEX Page](doc/dex.png)

### Historical Records & Charts
![Historical Records](doc/records.png)

### Data Source Settings
![Data Source Settings](doc/providers.png)

### Settings Page
![Settings Page](doc/setting_page.png)

---

## 📝 License

This project is licensed under the [MIT License](LICENSE).

---

## ⚠️ Disclaimer

All market data provided by this software is for informational purposes only and does not constitute investment advice. Data may be delayed, inaccurate, or incomplete. Users should exercise their own judgment and assume all investment risks.

本軟體提供的所有市場數據僅供參考，不構成任何投資建議。數據可能存在延遲、不準確或不完整的情況。使用者應自行判斷並承擔所有投資風險。

---

<div align="center">

**[⭐ Star this project](https://github.com/Wing9897/StockenBoard)** if you find it useful!

Made with ❤️ using Tauri + React + Rust

</div>

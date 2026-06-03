# StockenBoard

<div align="center">

**Real-time Stock & Cryptocurrency Dashboard**

33 Data Sources | Multi-page Management | HTTP API | Historical Data Recording

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://github.com/yourusername/stockenboard/actions/workflows/release.yml/badge.svg)](https://github.com/yourusername/stockenboard/actions/workflows/release.yml)
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

Download the latest version from [Releases](https://github.com/yourusername/StockenBoard/releases).

### Development

```bash
# Install dependencies
npm install

# Start development mode
npm run tauri dev

# Build
npm run tauri build
```

---

## 🔌 HTTP API

StockenBoard provides HTTP API for external programs (e.g., AI, Python scripts) to access data.

### Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/status` | System status |
| `GET /api/subscriptions` | All subscriptions |
| `GET /api/prices` | All latest prices |
| `GET /api/prices/{provider}/{symbol}` | Specific price |
| `GET /api/history` | Historical data query |

### Quick Example

```python
import requests

# Get all prices
prices = requests.get("http://localhost:8080/api/prices").json()
for p in prices['prices']:
    print(f"{p['symbol']}: ${p['price']}")

# Get historical data
history = requests.get("http://localhost:8080/api/history", params={
    "symbol": "BTCUSDT",
    "provider": "binance",
    "limit": 1000
}).json()
```

> 💡 **Tip**: API port can be modified in "Settings → API Guide" within the application. See that page for detailed documentation and more examples.

---

## 🛠️ Tech Stack

| Category | Technology |
|----------|------------|
| **Frontend** | React 19 + TypeScript 5.8 + Vite 7 |
| **Backend** | Tauri 2 + Rust 1.93 |
| **Database** | SQLite (tauri-plugin-sql) |
| **API** | Axum 0.7 + Tower |
| **Charts** | lightweight-charts 5.1 |
| **Theme** | Catppuccin Mocha |

---

## 📦 Project Structure

```
StockenBoard/
├── src/                    # Frontend code
│   ├── components/         # React components
│   ├── hooks/              # Custom hooks
│   ├── lib/                # Utilities and i18n
│   └── types/              # TypeScript type definitions
├── src-tauri/              # Backend code
│   └── src/
│       ├── providers/      # 33 data source implementations
│       ├── api_server.rs   # HTTP API Server
│       ├── polling.rs      # Unified polling manager
│       ├── commands.rs     # Tauri commands
│       └── db.rs           # Database schema
├── test_api.py             # API test script
└── example_ai_usage.py     # AI usage example
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

**[⭐ Star this project](https://github.com/yourusername/StockenBoard)** if you find it useful!

Made with ❤️ using Tauri + React + Rust

</div>

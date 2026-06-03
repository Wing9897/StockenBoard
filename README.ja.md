# StockenBoard

<div align="center">

**リアルタイム株式・暗号通貨ダッシュボード**

33 データソース対応 | マルチページ管理 | HTTP API | 履歴データ記録

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://github.com/yourusername/stockenboard/actions/workflows/release.yml/badge.svg)](https://github.com/yourusername/stockenboard/actions/workflows/release.yml)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB?logo=tauri)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react)](https://react.dev/)
[![Rust](https://img.shields.io/badge/Rust-1.93-orange?logo=rust)](https://www.rust-lang.org/)

[English](README.en.md) | [繁體中文](README.md) | [简体中文](README.zh-CN.md) | 日本語 | [한국어](README.ko.md)

</div>

---

## ✨ 機能

### 📊 データソース（33 種類）

<details>
<summary><b>暗号通貨取引所（10）</b></summary>

- Binance、Coinbase、Kraken、Bybit、KuCoin
- OKX、Gate.io、Bitfinex、HTX、MEXC

</details>

<details>
<summary><b>暗号通貨アグリゲーター（4）</b></summary>

- CoinGecko、CoinMarketCap、CoinPaprika、CryptoCompare

</details>

<details>
<summary><b>株式 / マルチアセット（12）</b></summary>

- Yahoo Finance、Finnhub、Alpha Vantage、Polygon
- Twelve Data、Alpaca、Tiingo、FMP
- Marketstack、EODHD、Mboum、FCS API

</details>

<details>
<summary><b>DEX アグリゲーター（4）</b></summary>

- Jupiter（Solana）
- OKX DEX（マルチチェーン）
- Raydium（Solana）
- Subgraph（Uniswap/Sushi/Pancake）

</details>

<details>
<summary><b>予測市場（2）</b></summary>

- Polymarket、Bitquery

</details>

<details>
<summary><b>マルチアセットアグリゲーター（1）</b></summary>

- CoinAPI

</details>

### 🎯 主な機能

- ⚡ **リアルタイム価格追跡** - REST + WebSocket デュアルモード
- 🔌 **HTTP API** - AI や外部プログラムからアクセス可能（ポート設定可能）
- 📈 **履歴データ記録** - 価格変動を自動記録、チャート分析対応
- 🎨 **複数表示モード** - Grid / List / Compact の 3 種類
- 📁 **カスタムページ** - 複数ページを作成して購読を整理
- 🔄 **一括操作** - 購読の一括追加、削除、エクスポート、インポート
- 🎭 **カスタマイズ可能な外観** - 14 種類のテーマ、カスタムアイコン、背景画像
- 🌍 **多言語対応** - 繁体字中国語、簡体字中国語、英語、日本語、韓国語
- 🔔 **プッシュ通知** - 条件トリガー + AI 評価、Telegram Bot & Webhook 対応、グローバルクールダウンで過剰通知を防止

### 🏗️ 技術アーキテクチャ

- **統一バックエンドポーリング** - Rust 側で定期的に fetch、フロントエンドは setInterval ゼロ
- **細粒度サブスクリプション** - 各カードは自身の価格変更時のみ再レンダリング
- **ページ認識** - 表示中の購読のみ API リクエスト、リソース節約
- **シングルトンデータベース** - グローバル共有 SQLite 接続、ロック問題回避

---

## 🚀 クイックスタート

### インストール

[Releases](https://github.com/yourusername/StockenBoard/releases) から最新版をダウンロード。

### 開発

```bash
# 依存関係のインストール
npm install

# 開発モード起動
npm run tauri dev

# ビルド
npm run tauri build
```

---

## 🔌 HTTP API

StockenBoard は外部プログラム（AI、Python スクリプトなど）がデータにアクセスできる HTTP API を提供します。

### エンドポイント

| エンドポイント | 説明 |
|---------------|------|
| `GET /api/status` | システムステータス |
| `GET /api/subscriptions` | 全購読リスト |
| `GET /api/prices` | 全最新価格 |
| `GET /api/prices/{provider}/{symbol}` | 特定価格 |
| `GET /api/history` | 履歴データクエリ |

### クイック例

```python
import requests

# 全価格取得
prices = requests.get("http://localhost:8080/api/prices").json()
for p in prices['prices']:
    print(f"{p['symbol']}: ${p['price']}")

# 履歴データ取得
history = requests.get("http://localhost:8080/api/history", params={
    "symbol": "BTCUSDT",
    "provider": "binance",
    "limit": 1000
}).json()
```

> 💡 **ヒント**：API ポートはアプリケーション内の「設定 → API 使用ガイド」で変更できます。詳細なドキュメントと例はそのページを参照してください。

---

## 🛠️ 技術スタック

| カテゴリ | 技術 |
|---------|------|
| **フロントエンド** | React 19 + TypeScript 5.8 + Vite 7 |
| **バックエンド** | Tauri 2 + Rust 1.93 |
| **データベース** | SQLite (tauri-plugin-sql) |
| **API** | Axum 0.7 + Tower |
| **チャート** | lightweight-charts 5.1 |
| **テーマ** | Catppuccin Mocha |

---

## 📦 プロジェクト構造

```
StockenBoard/
├── src/                    # フロントエンドコード
│   ├── components/         # React コンポーネント
│   ├── hooks/              # カスタムフック
│   ├── lib/                # ユーティリティと i18n
│   └── types/              # TypeScript 型定義
├── src-tauri/              # バックエンドコード
│   └── src/
│       ├── providers/      # 33 データソース実装
│       ├── api_server.rs   # HTTP API サーバー
│       ├── polling.rs      # 統一ポーリングマネージャー
│       ├── commands.rs     # Tauri コマンド
│       └── db.rs           # データベーススキーマ
├── test_api.py             # API テストスクリプト
└── example_ai_usage.py     # AI 使用例
```

---

## 🎨 スクリーンショット

### メインページ - Grid ビュー
![メインページ 1](doc/main_page1.png)

### メインページ - List ビュー
![メインページ 2](doc/main_page2.png)

### DEX ページ
![DEX ページ](doc/dex.png)

### 履歴記録とチャート
![履歴記録](doc/records.png)

### データソース設定
![データソース設定](doc/providers.png)

### 設定ページ
![設定ページ](doc/setting_page.png)

---

## 📝 ライセンス

このプロジェクトは [MIT License](LICENSE) の下でライセンスされています。

---

## ⚠️ 免責事項

本ソフトウェアが提供するすべての市場データは情報提供のみを目的としており、投資助言を構成するものではありません。データには遅延・不正確・不完全な場合があります。利用者は自己の判断で投資リスクを負うものとします。

All market data provided by this software is for informational purposes only and does not constitute investment advice. Data may be delayed, inaccurate, or incomplete. Users should exercise their own judgment and assume all investment risks.

---

<div align="center">

**[⭐ Star this project](https://github.com/yourusername/StockenBoard)** if you find it useful!

Made with ❤️ using Tauri + React + Rust

</div>

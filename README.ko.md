# StockenBoard

<div align="center">

**실시간 주식 및 암호화폐 대시보드**

33개 데이터 소스 지원 | 다중 페이지 관리 | HTTP API | 과거 데이터 기록

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB?logo=tauri)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react)](https://react.dev/)
[![Rust](https://img.shields.io/badge/Rust-1.93-orange?logo=rust)](https://www.rust-lang.org/)

[English](README.en.md) | [繁體中文](README.md) | [简体中文](README.zh-CN.md) | [日本語](README.ja.md) | 한국어

</div>

---

## ✨ 기능

### 📊 데이터 소스 (33개)

<details>
<summary><b>암호화폐 거래소 (10개)</b></summary>

- Binance, Coinbase, Kraken, Bybit, KuCoin
- OKX, Gate.io, Bitfinex, HTX, MEXC

</details>

<details>
<summary><b>암호화폐 애그리게이터 (4개)</b></summary>

- CoinGecko, CoinMarketCap, CoinPaprika, CryptoCompare

</details>

<details>
<summary><b>주식 / 다중 자산 (12개)</b></summary>

- Yahoo Finance, Finnhub, Alpha Vantage, Polygon
- Twelve Data, Alpaca, Tiingo, FMP
- Marketstack, EODHD, Mboum, FCS API

</details>

<details>
<summary><b>DEX 애그리게이터 (4개)</b></summary>

- Jupiter (Solana)
- OKX DEX (멀티체인)
- Raydium (Solana)
- Subgraph (Uniswap/Sushi/Pancake)

</details>

<details>
<summary><b>예측 시장 (2개)</b></summary>

- Polymarket, Bitquery

</details>

<details>
<summary><b>다중 자산 애그리게이터 (1개)</b></summary>

- CoinAPI

</details>

### 🎯 핵심 기능

- ⚡ **실시간 가격 추적** - REST + WebSocket 듀얼 모드
- 🔌 **HTTP API** - AI 또는 외부 프로그램 액세스 (포트 설정 가능)
- 📈 **과거 데이터 기록** - 가격 변동 자동 기록, 차트 분석 지원
- 🎨 **다중 뷰 모드** - Grid / List / Compact 3가지 표시 방식
- 📁 **사용자 정의 페이지** - 여러 페이지를 만들어 구독 정리
- 🔄 **일괄 작업** - 구독 일괄 추가, 삭제, 내보내기, 가져오기
- 🎭 **사용자 정의 외관** - 14가지 테마, 사용자 정의 아이콘, 배경 이미지
- 🌍 **다국어** - 번체 중국어, 간체 중국어, 영어, 일본어, 한국어

### 🏗️ 기술 아키텍처

- **통합 백엔드 폴링** - Rust 측에서 정기적으로 fetch, 프론트엔드는 setInterval 제로
- **세밀한 구독** - 각 카드는 자체 가격 변경 시에만 재렌더링
- **페이지 인식** - 표시된 구독에 대해서만 API 요청, 리소스 절약
- **싱글톤 데이터베이스** - 전역 공유 SQLite 연결, 잠금 문제 방지

---

## 🚀 빠른 시작

### 설치

[Releases](https://github.com/yourusername/StockenBoard/releases)에서 최신 버전을 다운로드하세요.

### 개발

```bash
# 의존성 설치
npm install

# 개발 모드 시작
npm run tauri dev

# 빌드
npm run tauri build
```

---

## 🔌 HTTP API

StockenBoard는 외부 프로그램(예: AI, Python 스크립트)이 데이터에 액세스할 수 있는 HTTP API를 제공합니다.

### 엔드포인트

| 엔드포인트 | 설명 |
|-----------|------|
| `GET /api/status` | 시스템 상태 |
| `GET /api/subscriptions` | 모든 구독 목록 |
| `GET /api/prices` | 모든 최신 가격 |
| `GET /api/prices/{provider}/{symbol}` | 특정 가격 |
| `GET /api/history` | 과거 데이터 쿼리 |

### 빠른 예제

```python
import requests

# 모든 가격 가져오기
prices = requests.get("http://localhost:8080/api/prices").json()
for p in prices['prices']:
    print(f"{p['symbol']}: ${p['price']}")

# 과거 데이터 가져오기
history = requests.get("http://localhost:8080/api/history", params={
    "symbol": "BTCUSDT",
    "provider": "binance",
    "limit": 1000
}).json()
```

> 💡 **팁**: API 포트는 애플리케이션 내 "설정 → API 사용 가이드"에서 수정할 수 있습니다. 자세한 문서와 예제는 해당 페이지를 참조하세요.

---

## 🛠️ 기술 스택

| 카테고리 | 기술 |
|---------|------|
| **프론트엔드** | React 19 + TypeScript 5.8 + Vite 7 |
| **백엔드** | Tauri 2 + Rust 1.93 |
| **데이터베이스** | SQLite (tauri-plugin-sql) |
| **API** | Axum 0.7 + Tower |
| **차트** | lightweight-charts 5.1 |
| **테마** | Catppuccin Mocha |

---

## 📦 프로젝트 구조

```
StockenBoard/
├── src/                    # 프론트엔드 코드
│   ├── components/         # React 컴포넌트
│   ├── hooks/              # 커스텀 훅
│   ├── lib/                # 유틸리티 및 i18n
│   └── types/              # TypeScript 타입 정의
├── src-tauri/              # 백엔드 코드
│   └── src/
│       ├── providers/      # 33개 데이터 소스 구현
│       ├── api_server.rs   # HTTP API 서버
│       ├── polling.rs      # 통합 폴링 매니저
│       ├── commands.rs     # Tauri 커맨드
│       └── db.rs           # 데이터베이스 스키마
├── test_api.py             # API 테스트 스크립트
└── example_ai_usage.py     # AI 사용 예제
```

---

## 🎨 스크린샷

### 메인 페이지 - Grid 뷰
![메인 페이지 1](doc/main_page1.png)

### 메인 페이지 - List 뷰
![메인 페이지 2](doc/main_page2.png)

### DEX 페이지
![DEX 페이지](doc/dex.png)

### 과거 기록 및 차트
![과거 기록](doc/records.png)

### 데이터 소스 설정
![데이터 소스 설정](doc/providers.png)

### 설정 페이지
![설정 페이지](doc/setting_page.png)

---

## 📝 라이선스

이 프로젝트는 [MIT License](LICENSE)에 따라 라이선스가 부여됩니다.

---

## ⚠️ 면책 조항

본 소프트웨어가 제공하는 모든 시장 데이터는 참고용이며 투자 조언을 구성하지 않습니다. 데이터는 지연되거나 부정확하거나 불완전할 수 있습니다. 사용자는 스스로 판단하고 모든 투자 위험을 부담해야 합니다.

All market data provided by this software is for informational purposes only and does not constitute investment advice. Data may be delayed, inaccurate, or incomplete. Users should exercise their own judgment and assume all investment risks.

---

<div align="center">

**[⭐ Star this project](https://github.com/yourusername/StockenBoard)** if you find it useful!

Made with ❤️ using Tauri + React + Rust

</div>

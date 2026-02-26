# Extra 欄位對照表（Provider 特有資料）

每個 provider 除了標準 `AssetData` 欄位外，可能在 `extra: HashMap<String, Value>` 中回傳額外資料。
以下列出所有 provider 的 extra 欄位。

## Extra 欄位總覽

| # | Provider | extra 欄位 |
|:---:|---|---|
| 1 | Binance | 開盤價, 加權平均價, 交易次數, 報價成交量 |
| 2 | Coinbase | （無） |
| 3 | CoinGecko | （無） |
| 4 | CoinMarketCap | 名稱, CMC排名, 流通量, 7日變動% |
| 5 | CryptoCompare | （無） |
| 6 | Yahoo Finance | 前收盤價, 52週高, 52週低, 交易所 |
| 7 | Finnhub | 開盤價, 前收盤價 |
| 8 | Alpha Vantage | 開盤價, 前收盤價 |
| 9 | Polygon.io | 開盤價, 加權平均價, 交易次數 |
| 10 | Twelve Data | 開盤價, 前收盤價, 52週高, 52週低 |
| 11 | Alpaca | 開盤價, 加權平均價, 交易次數 |
| 12 | Tiingo (股票) | 開盤價, 前收盤價 |
| 13 | Tiingo (加密) | （無） |
| 14 | FMP | 開盤價, 前收盤價, 52週高, 52週低, 本益比, 每股盈餘, 公司名稱 |
| 15 | Marketstack | 開盤價, 交易所 |
| 16 | EODHD | 開盤價, 前收盤價 |
| 17 | Mboum | 開盤價, 前收盤價, 52週高, 52週低, 名稱 |
| 18 | Polymarket | 問題, 結束日期, 選項 |
| 19 | Bitquery | （無） |
| 20 | Kraken | （無） |
| 21 | Bybit | 成交額 |
| 22 | KuCoin | 成交額, 均價 |
| 23 | OKX | 成交額 |
| 24 | Gate.io | 成交額 |
| 25 | Bitfinex | （無） |
| 26 | HTX (Huobi) | 成交額 |
| 27 | MEXC | 成交額 |
| 28 | CoinPaprika | ATH, 1h%, 7d% |
| 29 | CoinAPI | （無） |
| 30 | FCS API | 交易所 |
| 31 | Jupiter | mint |
| 32 | OKX DEX | 鏈, token, 預估Gas |
| 33 | Raydium | pool_tvl, route_path, token_from, token_to, amount_out, gas_estimate |
| 34 | Subgraph | pool_tvl, volume_24h, route_path, token_from, token_to |

## Extra 欄位交叉對照表

以下表格以欄位為列、provider 為欄，標示哪些 provider 提供哪些 extra 欄位。

### 股票相關欄位

| 欄位 | Binance | Yahoo | Finnhub | AlphaV | Polygon | 12Data | Alpaca | Tiingo(股) | FMP | Mktstack | EODHD | Mboum | FCS |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| 開盤價 | ✅ | | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | |
| 前收盤價 | | ✅ | ✅ | ✅ | | ✅ | | ✅ | ✅ | | ✅ | ✅ | |
| 52週高 | | ✅ | | | | ✅ | | | ✅ | | | ✅ | |
| 52週低 | | ✅ | | | | ✅ | | | ✅ | | | ✅ | |
| 本益比 | | | | | | | | | ✅ | | | | |
| 每股盈餘 | | | | | | | | | ✅ | | | | |
| 公司名稱 | | | | | | | | | ✅ | | | | |
| 交易所 | | ✅ | | | | | | | | ✅ | | | ✅ |

### 交易相關欄位

| 欄位 | Binance | Polygon | Alpaca | Bybit | KuCoin | OKX | Gate.io | HTX | MEXC |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| 加權平均價 | ✅ | ✅ | ✅ | | | | | | |
| 交易次數 | ✅ | ✅ | ✅ | | | | | | |
| 報價成交量 | ✅ | | | | | | | | |
| 成交額 | | | | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 均價 | | | | | ✅ | | | | |

### 加密貨幣聚合器欄位

| 欄位 | CoinMarketCap | CoinPaprika |
|---|:---:|:---:|
| 名稱 | ✅ | |
| CMC排名 | ✅ | |
| 流通量 | ✅ | |
| 7日變動% | ✅ | ✅ |
| 1h% | | ✅ |
| ATH | | ✅ |

### 預測市場欄位（Polymarket 獨有）

| 欄位 | 說明 |
|---|---|
| 問題 | 預測市場的問題描述 |
| 結束日期 | 市場結束日期 |
| 選項 | 可選的結果選項 |

### Mboum 獨有

| 欄位 | 說明 |
|---|---|
| 名稱 | 資產全名（如 "Apple Inc."） |

## 欄位出現頻率排序

| 欄位 | 出現次數 | Provider 列表 |
|---|:---:|---|
| 開盤價 | 11 | Binance, Finnhub, AlphaVantage, Polygon, TwelveData, Alpaca, Tiingo(股), FMP, Marketstack, EODHD, Mboum |
| 前收盤價 | 9 | Yahoo, Finnhub, AlphaVantage, TwelveData, Tiingo(股), FMP, EODHD, Mboum |
| 成交額 | 6 | Bybit, KuCoin, OKX, Gate.io, HTX, MEXC |
| 52週高 | 4 | Yahoo, TwelveData, FMP, Mboum |
| 52週低 | 4 | Yahoo, TwelveData, FMP, Mboum |
| 交易所 | 3 | Yahoo, Marketstack, FCS API |
| 加權平均價 | 3 | Binance, Polygon, Alpaca |
| 交易次數 | 3 | Binance, Polygon, Alpaca |
| 7日變動% | 2 | CoinMarketCap, CoinPaprika |
| 名稱 | 2 | CoinMarketCap, Mboum |
| 報價成交量 | 1 | Binance |
| 均價 | 1 | KuCoin |
| CMC排名 | 1 | CoinMarketCap |
| 流通量 | 1 | CoinMarketCap |
| 1h% | 1 | CoinPaprika |
| ATH | 1 | CoinPaprika |
| 本益比 | 1 | FMP |
| 每股盈餘 | 1 | FMP |
| 公司名稱 | 1 | FMP |
| 問題 | 1 | Polymarket |
| 結束日期 | 1 | Polymarket |
| 選項 | 1 | Polymarket |

## Card 設計建議

### 展開區域優先顯示（出現頻率高）
1. 開盤價（11 個 provider）
2. 前收盤價（9 個 provider）
3. 成交額（6 個 provider）

### 條件顯示（特定類型才有意義）
- 股票類：52週高/低, 本益比, 每股盈餘, 公司名稱
- 交易所類：加權平均價, 交易次數, 報價成交量
- 加密聚合器：CMC排名, 流通量, ATH, 1h%, 7d%
- 預測市場：問題, 結束日期, 選項（需要特殊 UI）

### 無 extra 的 Provider（8 個）
Coinbase, CoinGecko, CryptoCompare, Tiingo(加密), Bitquery, Kraken, Bitfinex, CoinAPI

### DEX 聚合器欄位

| 欄位 | Jupiter | OKX DEX | Raydium | Subgraph | 說明 |
|---|:---:|:---:|:---:|:---:|---|
| mint | ✅ | | | | Solana mint address |
| 鏈 | | ✅ | | | 所在區塊鏈 (Ethereum, BSC, Solana 等) |
| token | | ✅ | | | Token 合約地址 |
| 預估Gas | | ✅ | ✅ | | 預估 Gas 費用 |
| pool_tvl | | | ✅ | ✅ | 池 TVL |
| route_path | | | ✅ | ✅ | 路由路徑 |
| token_from | | | ✅ | ✅ | 來源 token 地址 |
| token_to | | | ✅ | ✅ | 目標 token 地址 |
| amount_out | | | ✅ | | 1 個 token_from 可換得的 token_to 數量 |
| volume_24h | | | | ✅ | 24h 成交量 (USD) |

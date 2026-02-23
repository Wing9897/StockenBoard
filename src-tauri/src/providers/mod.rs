pub mod traits;

// Crypto exchanges
pub mod binance;
pub mod bitfinex;
pub mod bybit;
pub mod coinbase;
pub mod gateio;
pub mod htx;
pub mod kraken;
pub mod kucoin;
pub mod mexc;
pub mod okx;

// Crypto aggregators
pub mod coingecko;
pub mod coinmarketcap;
pub mod coinpaprika;
pub mod cryptocompare;

// Stock / multi-asset
pub mod alpaca;
pub mod alphavantage;
pub mod eodhd;
pub mod fcsapi;
pub mod finnhub;
pub mod fmp;
pub mod marketstack;
pub mod mboum;
pub mod polygon;
pub mod tiingo;
pub mod twelvedata;
pub mod yahoo;

// Multi-asset aggregators
pub mod coinapi;

// DEX aggregators
pub mod jupiter;
pub mod okx_dex;

// Prediction markets
pub mod bitquery;
pub mod polymarket;

// WebSocket
pub mod ws_binance;

pub use traits::{
    get_all_provider_info, AssetData, DataProvider, ProviderInfo, WebSocketProvider, WsTickerUpdate,
};

use std::sync::Arc;

pub fn create_provider(
    id: &str,
    api_key: Option<String>,
    api_secret: Option<String>,
) -> Option<Arc<dyn DataProvider>> {
    match id {
        // Crypto exchanges
        "binance" => Some(Arc::new(binance::BinanceProvider::new(api_key))),
        "coinbase" => Some(Arc::new(coinbase::CoinbaseProvider::new())),
        "kraken" => Some(Arc::new(kraken::KrakenProvider::new())),
        "bybit" => Some(Arc::new(bybit::BybitProvider::new())),
        "kucoin" => Some(Arc::new(kucoin::KuCoinProvider::new())),
        "okx" => Some(Arc::new(okx::OkxProvider::new())),
        "gateio" => Some(Arc::new(gateio::GateioProvider::new())),
        "bitfinex" => Some(Arc::new(bitfinex::BitfinexProvider::new())),
        "htx" => Some(Arc::new(htx::HtxProvider::new())),
        "mexc" => Some(Arc::new(mexc::MexcProvider::new())),
        // Crypto aggregators
        "coingecko" => Some(Arc::new(coingecko::CoinGeckoProvider::new(api_key))),
        "coinmarketcap" => Some(Arc::new(coinmarketcap::CoinMarketCapProvider::new(api_key))),
        "coinpaprika" => Some(Arc::new(coinpaprika::CoinPaprikaProvider::new())),
        "cryptocompare" => Some(Arc::new(cryptocompare::CryptoCompareProvider::new(api_key))),
        // Stock / multi-asset
        "yahoo" => Some(Arc::new(yahoo::YahooProvider::new())),
        "finnhub" => Some(Arc::new(finnhub::FinnhubProvider::new(api_key))),
        "alphavantage" => Some(Arc::new(alphavantage::AlphaVantageProvider::new(api_key))),
        "polygon" => Some(Arc::new(polygon::PolygonProvider::new(api_key))),
        "twelvedata" => Some(Arc::new(twelvedata::TwelveDataProvider::new(api_key))),
        "alpaca" => Some(Arc::new(alpaca::AlpacaProvider::new(api_key, api_secret))),
        "tiingo" => Some(Arc::new(tiingo::TiingoProvider::new(api_key))),
        "fmp" => Some(Arc::new(fmp::FMPProvider::new(api_key))),
        "marketstack" => Some(Arc::new(marketstack::MarketstackProvider::new(api_key))),
        "eodhd" => Some(Arc::new(eodhd::EODHDProvider::new(api_key))),
        "mboum" => Some(Arc::new(mboum::MboumProvider::new(api_key))),
        "fcsapi" => Some(Arc::new(fcsapi::FcsApiProvider::new(api_key))),
        // Multi-asset aggregators
        "coinapi" => Some(Arc::new(coinapi::CoinApiProvider::new(api_key))),
        // DEX aggregators
        "jupiter" => Some(Arc::new(jupiter::JupiterProvider::new(api_key))),
        "okx_dex" => Some(Arc::new(okx_dex::OkxDexProvider::new(api_key))),
        // Prediction markets
        "polymarket" => Some(Arc::new(polymarket::PolymarketProvider::new())),
        "bitquery" => Some(Arc::new(bitquery::BitqueryProvider::new(api_key))),
        _ => None,
    }
}

pub fn create_ws_provider(id: &str) -> Option<Arc<dyn WebSocketProvider>> {
    match id {
        "binance" => Some(Arc::new(ws_binance::BinanceWsProvider::new())),
        _ => None,
    }
}

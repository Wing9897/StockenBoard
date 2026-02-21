pub mod traits;
pub mod binance;
pub mod coinbase;
pub mod coingecko;
pub mod coinmarketcap;
pub mod cryptocompare;
pub mod yahoo;
pub mod finnhub;
pub mod alphavantage;
pub mod polygon;
pub mod twelvedata;
pub mod alpaca;
pub mod tiingo;
pub mod fmp;
pub mod marketstack;
pub mod eodhd;
pub mod polymarket;
pub mod mboum;
pub mod bitquery;
pub mod ws_binance;

pub use traits::{AssetData, DataProvider, ProviderInfo, WebSocketProvider, WsTickerUpdate, get_all_provider_info};

use std::sync::Arc;

pub fn create_provider(provider_id: &str, api_key: Option<String>, api_secret: Option<String>) -> Option<Arc<dyn DataProvider>> {
    match provider_id {
        "binance" => Some(Arc::new(binance::BinanceProvider::new(api_key))),
        "coinbase" => Some(Arc::new(coinbase::CoinbaseProvider::new())),
        "coingecko" => Some(Arc::new(coingecko::CoinGeckoProvider::new(api_key))),
        "coinmarketcap" => Some(Arc::new(coinmarketcap::CoinMarketCapProvider::new(api_key))),
        "cryptocompare" => Some(Arc::new(cryptocompare::CryptoCompareProvider::new(api_key))),
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
        "polymarket" => Some(Arc::new(polymarket::PolymarketProvider::new())),
        "mboum" => Some(Arc::new(mboum::MboumProvider::new(api_key))),
        "bitquery" => Some(Arc::new(bitquery::BitqueryProvider::new(api_key))),
        _ => None,
    }
}

pub fn create_ws_provider(provider_id: &str) -> Option<Arc<dyn WebSocketProvider>> {
    match provider_id {
        "binance" => Some(Arc::new(ws_binance::BinanceWsProvider::new())),
        _ => None,
    }
}

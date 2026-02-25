use super::traits::*;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Binance WebSocket streaming for real-time ticker data
pub struct BinanceWsProvider;

const MAX_RECONNECT_ATTEMPTS: u32 = 10;
const INITIAL_RECONNECT_DELAY_MS: u64 = 1000;

impl BinanceWsProvider {
    pub fn new() -> Self { Self }

    /// 解析 miniTicker WS 訊息為 WsTickerUpdate
    fn parse_mini_ticker(d: &serde_json::Value) -> Option<WsTickerUpdate> {
        if d.is_null() { return None; }
        let symbol = d["s"].as_str()?.to_string();
        let parse_f64 = |key: &str| d[key].as_str().and_then(|s| s.parse::<f64>().ok());

        let asset = AssetDataBuilder::new(&symbol, "binance")
            .price(parse_f64("c").unwrap_or(0.0))
            .currency("USDT")
            .high_24h(parse_f64("h"))
            .low_24h(parse_f64("l"))
            .volume(parse_f64("v"))
            .extra_f64("open_price", parse_f64("o"))
            .build();

        Some(WsTickerUpdate {
            symbol,
            provider_id: "binance".to_string(),
            data: asset,
        })
    }
}

#[async_trait::async_trait]
impl WebSocketProvider for BinanceWsProvider {
    async fn subscribe(
        &self,
        symbols: Vec<String>,
        sender: Arc<tokio::sync::broadcast::Sender<WsTickerUpdate>>,
    ) -> Result<tokio::task::JoinHandle<()>, String> {
        if symbols.is_empty() {
            // 返回一個立即完成的 task
            return Ok(tokio::spawn(async {}));
        }

        let streams: Vec<String> = symbols.iter()
            .map(|s| format!("{}@miniTicker", s.to_lowercase()))
            .collect();
        let url = format!(
            "wss://stream.binance.com:9443/stream?streams={}",
            streams.join("/")
        );

        let (ws_stream, _) = connect_async(&url).await
            .map_err(|e| format!("Binance WS 連接失敗: {}", e))?;

        let (write, read) = ws_stream.split();

        let handle = tokio::spawn(Self::run_ws_loop(url, symbols, sender, write, read));

        Ok(handle)
    }
}

impl BinanceWsProvider {
    async fn run_ws_loop(
        url: String,
        symbols: Vec<String>,
        sender: Arc<tokio::sync::broadcast::Sender<WsTickerUpdate>>,
        mut write: futures::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
            Message
        >,
        mut read: futures::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>
        >,
    ) {
        loop {
            match read.next().await {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text.to_string()) {
                        if let Some(update) = Self::parse_mini_ticker(&data["data"]) {
                            let _ = sender.send(update);
                        }
                    }
                }
                Some(Ok(Message::Ping(payload))) => {
                    if let Err(e) = write.send(Message::Pong(payload)).await {
                        eprintln!("Binance WS pong 發送失敗: {}", e);
                        break;
                    }
                }
                Some(Ok(Message::Close(_))) => {
                    eprintln!("Binance WS 連接已關閉，準備重連...");
                    break;
                }
                Some(Err(e)) => {
                    eprintln!("Binance WS 錯誤: {}，準備重連...", e);
                    break;
                }
                None => {
                    eprintln!("Binance WS stream 結束，準備重連...");
                    break;
                }
                _ => {}
            }
        }

        // 自動重連（指數退避）
        let mut attempt = 0u32;
        loop {
            if attempt >= MAX_RECONNECT_ATTEMPTS {
                eprintln!("Binance WS 重連失敗次數已達上限 ({})", MAX_RECONNECT_ATTEMPTS);
                break;
            }
            let delay = INITIAL_RECONNECT_DELAY_MS * 2u64.pow(attempt.min(6));
            eprintln!("Binance WS 第 {} 次重連，等待 {}ms...", attempt + 1, delay);
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;

            match connect_async(&url).await {
                Ok((new_ws, _)) => {
                    eprintln!("Binance WS 重連成功");
                    let (new_write, new_read) = new_ws.split();
                    Box::pin(Self::run_ws_loop(url, symbols, sender, new_write, new_read)).await;
                    return;
                }
                Err(e) => {
                    eprintln!("Binance WS 重連失敗: {}", e);
                    attempt += 1;
                }
            }
        }
    }
}

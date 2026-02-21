use super::traits::*;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Binance WebSocket streaming for real-time ticker data
pub struct BinanceWsProvider;

const MAX_RECONNECT_ATTEMPTS: u32 = 10;
const INITIAL_RECONNECT_DELAY_MS: u64 = 1000;

impl BinanceWsProvider {
    pub fn new() -> Self { Self }
}

#[async_trait::async_trait]
impl WebSocketProvider for BinanceWsProvider {
    async fn subscribe(
        &self,
        symbols: Vec<String>,
        sender: Arc<tokio::sync::broadcast::Sender<WsTickerUpdate>>,
    ) -> Result<(), String> {
        if symbols.is_empty() {
            return Ok(());
        }

        let streams: Vec<String> = symbols.iter()
            .map(|s| format!("{}@miniTicker", s.to_lowercase()))
            .collect();
        let url = format!(
            "wss://stream.binance.com:9443/stream?streams={}",
            streams.join("/")
        );

        Self::connect_with_reconnect(url, symbols, sender).await
    }
}

impl BinanceWsProvider {
    async fn connect_with_reconnect(
        url: String,
        symbols: Vec<String>,
        sender: Arc<tokio::sync::broadcast::Sender<WsTickerUpdate>>,
    ) -> Result<(), String> {
        let (ws_stream, _) = connect_async(&url).await
            .map_err(|e| format!("Binance WS 連接失敗: {}", e))?;

        let (mut write, mut read) = ws_stream.split();

        // 用於重連的 clone
        let url_clone = url.clone();
        let symbols_clone = symbols.clone();
        let sender_clone = sender.clone();

        tokio::spawn(async move {
            loop {
                match read.next().await {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text.to_string()) {
                            let d = &data["data"];
                            if d.is_null() { continue; }

                            let symbol = d["s"].as_str().unwrap_or("").to_string();
                            let parse_f64 = |key: &str| d[key].as_str().and_then(|s| s.parse::<f64>().ok());

                            let asset = AssetDataBuilder::new(&symbol, "binance")
                                .price(parse_f64("c").unwrap_or(0.0))
                                .currency("USDT")
                                .high_24h(parse_f64("h"))
                                .low_24h(parse_f64("l"))
                                .volume(parse_f64("v"))
                                .extra_f64("開盤價", parse_f64("o"))
                                .build();

                            let _ = sender.send(WsTickerUpdate {
                                symbol: symbol.clone(),
                                provider_id: "binance".to_string(),
                                data: asset,
                            });
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

                match connect_async(&url_clone).await {
                    Ok((new_ws, _)) => {
                        eprintln!("Binance WS 重連成功");
                        let (new_write, new_read) = new_ws.split();
                        // 遞迴啟動新的監聽循環
                        let url2 = url_clone.clone();
                        let syms2 = symbols_clone.clone();
                        let sender2 = sender_clone.clone();
                        tokio::spawn(async move {
                            Self::run_ws_loop(url2, syms2, sender2, new_write, new_read).await;
                        });
                        return;
                    }
                    Err(e) => {
                        eprintln!("Binance WS 重連失敗: {}", e);
                        attempt += 1;
                    }
                }
            }
        });

        Ok(())
    }

    async fn run_ws_loop(
        url: String,
        symbols: Vec<String>,
        sender: Arc<tokio::sync::broadcast::Sender<WsTickerUpdate>>,
        mut write: futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
            Message
        >,
        mut read: futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>
        >,
    ) {
        loop {
            match read.next().await {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text.to_string()) {
                        let d = &data["data"];
                        if d.is_null() { continue; }

                        let symbol = d["s"].as_str().unwrap_or("").to_string();
                        let parse_f64 = |key: &str| d[key].as_str().and_then(|s| s.parse::<f64>().ok());

                        let asset = AssetDataBuilder::new(&symbol, "binance")
                            .price(parse_f64("c").unwrap_or(0.0))
                            .currency("USDT")
                            .high_24h(parse_f64("h"))
                            .low_24h(parse_f64("l"))
                            .volume(parse_f64("v"))
                            .extra_f64("開盤價", parse_f64("o"))
                            .build();

                        let _ = sender.send(WsTickerUpdate {
                            symbol: symbol.clone(),
                            provider_id: "binance".to_string(),
                            data: asset,
                        });
                    }
                }
                Some(Ok(Message::Ping(payload))) => {
                    if let Err(e) = write.send(Message::Pong(payload)).await {
                        eprintln!("Binance WS pong 發送失敗: {}", e);
                        break;
                    }
                }
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => {
                    eprintln!("Binance WS 連接中斷，準備重連...");
                    break;
                }
                _ => {}
            }
        }

        // 重連
        let mut attempt = 0u32;
        loop {
            if attempt >= MAX_RECONNECT_ATTEMPTS {
                eprintln!("Binance WS 重連失敗次數已達上限");
                break;
            }
            let delay = INITIAL_RECONNECT_DELAY_MS * 2u64.pow(attempt.min(6));
            eprintln!("Binance WS 第 {} 次重連，等待 {}ms...", attempt + 1, delay);
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;

            match connect_async(&url).await {
                Ok((new_ws, _)) => {
                    eprintln!("Binance WS 重連成功");
                    let (new_write, new_read) = new_ws.split();
                    // 遞迴重連
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

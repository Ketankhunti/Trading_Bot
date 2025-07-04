// src/websocket_stream/mod.rs

//! This module provides a client for connecting to Binance's public WebSocket
//! market data streams (e.g., klines, aggregated trades, tickers).
//! It handles the connection and continuous reception of stream messages.

use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::Value;
use log::{info, error, debug};
use url::Url; // For URL parsing

/// Represents the client for connecting to public WebSocket market data streams.
pub struct MarketStreamClient {
    ws_base_url_market_stream: String, // Base URL for public market data streams
}

impl MarketStreamClient {
    /// Creates a new `MarketStreamClient` instance.
    ///
    /// # Arguments
    /// * `ws_base_url_market_stream` - The base URL for public market data WebSocket streams (e.g., "wss://fstream.binancefuture.com/ws").
    ///
    /// # Returns
    /// A new `MarketStreamClient` instance.
    pub fn new(ws_base_url_market_stream: String) -> Self {
        Self {
            ws_base_url_market_stream,
        }
    }

    /// Connects to a public WebSocket stream for live market data (e.g., Klines).
    /// This uses a separate WebSocket connection from the API calls.
    ///
    /// # Arguments
    /// * `stream_path` - The path for the WebSocket stream (e.g., "btcusdt@kline_1m").
    /// * `callback` - An asynchronous function that will be called with each received message.
    ///
    /// # Returns
    /// A `Result` indicating success or a `String` error if the connection fails.
    pub async fn connect_market_stream<F>(&self, stream_path: &str, mut callback: F) -> Result<(), String>
    where
        F: FnMut(Value) + Send + 'static,
    {
        let stream_url = format!("{}/{}", self.ws_base_url_market_stream, stream_path);
        info!("Connecting to public WebSocket stream: {}", stream_url);

        let url = Url::parse(&stream_url)
            .map_err(|e| format!("Failed to parse stream URL: {}", e))?;

        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| format!("Failed to connect to WebSocket: {}", e))?;

        info!("Public WebSocket connection established for {}", stream_url);

        let (_, mut read) = ws_stream.split();

        while let Some(message) = read.next().await {
            match message {
                Ok(msg) => {
                    if msg.is_text() {
                        let text = msg.to_text().map_err(|e| format!("Failed to convert WS message to text: {}", e))?;
                        match serde_json::from_str::<Value>(&text) {
                            Ok(json_value) => {
                                callback(json_value);
                            },
                            Err(e) => error!("Failed to parse WebSocket message as JSON: {} - {}", e, text),
                        }
                    } else if msg.is_ping() {
                        debug!("Received Ping: {:?}", msg.into_data());
                        // tokio-tungstenite automatically sends Pong for Ping
                    } else if msg.is_close() {
                        info!("Public WebSocket connection closed: {:?}", msg.into_close_frame());
                        break;
                    }
                },
                Err(e) => {
                    error!("Public WebSocket error: {}", e);
                    return Err(format!("Public WebSocket stream error: {}", e));
                }
            }
        }
        Ok(())
    }
}

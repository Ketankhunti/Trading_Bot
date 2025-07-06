// src/websocket_stream/mod.rs

//! This module provides a client for connecting to Binance's public WebSocket
//! market data streams (e.g., klines, aggregated trades, tickers).
//! It handles the connection, continuous reception of stream messages,
//! and dynamic subscription/unsubscription to streams.

use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use std::collections::HashMap;
use log::{info, error, debug, warn};

/// Represents a generic WebSocket message received from Binance.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum BinanceWsMessage {
    /// A successful subscription/unsubscription response
    #[serde(rename_all = "camelCase")]
    Result(SubscriptionResult),
    /// An error message from the WebSocket server
    #[serde(rename_all = "camelCase")]
    Error(WsError),
    /// Data from a specific stream
    #[serde(rename_all = "camelCase")]
    StreamData {
        stream: String,
        data: Value,
    },
    /// Raw JSON value for unknown messages
    Raw(Value),
}

/// Represents a successful subscription/unsubscription result.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SubscriptionResult {
    pub result: Option<Value>,
    pub id: u64,
}

/// Represents an error message from the WebSocket server.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WsError {
    pub code: i64,
    pub msg: String,
    pub id: Option<u64>,
}

/// Enum to represent different types of requests that the Market Stream listener task handles.
enum WsStreamRequest {
    /// Request to subscribe to new streams.
    Subscribe {
        id: u64,
        streams: Vec<String>,
        response_tx: oneshot::Sender<Result<Value, String>>,
    },
    /// Request to unsubscribe from streams.
    Unsubscribe {
        id: u64,
        streams: Vec<String>,
        response_tx: oneshot::Sender<Result<Value, String>>,
    },
    /// Request to list current subscriptions.
    ListSubscriptions {
        id: u64,
        response_tx: oneshot::Sender<Result<Value, String>>,
    },
    /// Request to set a property (e.g., combined stream payloads).
    SetProperty {
        id: u64,
        property: String,
        value: Value,
        response_tx: oneshot::Sender<Result<Value, String>>,
    },
    /// Request to get a property.
    GetProperty {
        id: u64,
        property: String,
        response_tx: oneshot::Sender<Result<Value, String>>,
    },
    /// Signal to send a raw message (e.g., for unsolicited pings/pongs if needed).
    SendRawMessage {
        message: Message,
    },
}

/// Represents the client for connecting to public WebSocket market data streams.
pub struct MarketStreamClient {
    ws_base_url_market_stream: String, // Base URL for public market data streams
    // Channel for sending requests to the WebSocket stream listener task
    ws_stream_request_sender: mpsc::Sender<WsStreamRequest>,
    // Handle to the WebSocket stream listener task
    _ws_stream_listener_handle: JoinHandle<()>,
    // Sender for parsed stream data to the consumer
    data_sender: mpsc::Sender<BinanceWsMessage>,
}

impl MarketStreamClient {
    /// Creates a new `MarketStreamClient` instance.
    ///
    /// # Arguments
    /// * `ws_base_url_market_stream` - The base URL for public market data WebSocket streams (e.g., "wss://fstream.binancefuture.com/ws").
    /// * `data_sender` - An `mpsc::Sender` to send parsed `BinanceWsMessage`s (stream data) to.
    ///
    /// # Returns
    /// A new `MarketStreamClient` instance.
    pub async fn new(
        ws_base_url_market_stream: String,
        data_sender: mpsc::Sender<BinanceWsMessage>,
    ) -> Self {
        let (ws_stream_request_sender, ws_stream_request_receiver) = mpsc::channel::<WsStreamRequest>(100);

        let ws_base_url_clone = ws_base_url_market_stream.clone();
        let data_sender_clone = data_sender.clone();

        let ws_stream_listener_handle = tokio::spawn(async move {
            Self::run_market_stream_listener(
                ws_stream_request_receiver,
                ws_base_url_clone,
                data_sender_clone,
            ).await;
        });

        Self {
            ws_base_url_market_stream,
            ws_stream_request_sender,
            _ws_stream_listener_handle: ws_stream_listener_handle,
            data_sender,
        }
    }

    /// Dedicated task to manage the WebSocket stream connection (for public market data).
    /// This function is spawned and runs independently.
    async fn run_market_stream_listener(
        mut ws_request_receiver: mpsc::Receiver<WsStreamRequest>,
        ws_base_url_market_stream: String,
        data_sender: mpsc::Sender<BinanceWsMessage>, // To send parsed stream data out
    ) {
        let mut pending_requests: HashMap<u64, oneshot::Sender<Result<Value, String>>> = HashMap::new();
        let mut ws_stream_opt = None;
        // `next_request_id` is managed by `get_next_request_id` now, no need for it here.

        loop {
            // Reconnect if stream is not established or disconnected
            if ws_stream_opt.is_none() {
                info!("Attempting to connect to Market Stream at {}", ws_base_url_market_stream);
                match connect_async(&ws_base_url_market_stream).await {
                    Ok((ws_stream, _)) => {
                        info!("Market Stream connection established.");
                        ws_stream_opt = Some(ws_stream);
                        // On reconnection, resubscribe to all active streams if managing state
                        // For simplicity, this example doesn't persist active subscriptions across reconnects.
                        // A more robust solution would store `streams` from `Subscribe` requests.
                    },
                    Err(e) => {
                        error!("Failed to connect to Market Stream: {}. Retrying in 5 seconds...", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        continue;
                    }
                }
            }

            let mut need_reconnect = false;
            {
                let ws_stream = ws_stream_opt.as_mut().unwrap();
                let (mut write, mut read) = ws_stream.split();

                tokio::select! {
                    // Handle outgoing requests from the client
                    req = ws_request_receiver.recv() => {
                        if let Some(ws_req) = req {
                            let (id, message_text, response_tx_opt) = match ws_req {
                                WsStreamRequest::Subscribe { id, streams, response_tx } => {
                                    let payload = json!({
                                        "method": "SUBSCRIBE",
                                        "params": streams,
                                        "id": id,
                                    }).to_string();
                                    (id, payload, Some(response_tx))
                                },
                                WsStreamRequest::Unsubscribe { id, streams, response_tx } => {
                                    let payload = json!({
                                        "method": "UNSUBSCRIBE",
                                        "params": streams,
                                        "id": id,
                                    }).to_string();
                                    (id, payload, Some(response_tx))
                                },
                                WsStreamRequest::ListSubscriptions { id, response_tx } => {
                                    let payload = json!({
                                        "method": "LIST_SUBSCRIPTIONS",
                                        "id": id,
                                    }).to_string();
                                    (id, payload, Some(response_tx))
                                },
                                WsStreamRequest::SetProperty { id, property, value, response_tx } => {
                                    let payload = json!({
                                        "method": "SET_PROPERTY",
                                        "params": [property, value],
                                        "id": id,
                                    }).to_string();
                                    (id, payload, Some(response_tx))
                                },
                                WsStreamRequest::GetProperty { id, property, response_tx } => {
                                    let payload = json!({
                                        "method": "GET_PROPERTY",
                                        "params": [property],
                                        "id": id,
                                    }).to_string();
                                    (id, payload, Some(response_tx))
                                },
                                WsStreamRequest::SendRawMessage { message } => {
                                    // This variant is for sending raw messages directly, not expecting a response via oneshot
                                    if let Err(e) = write.send(message).await {
                                        error!("Failed to send raw WebSocket message: {}", e);
                                        need_reconnect = true;
                                    }
                                    continue; // Continue to next select iteration
                                }
                            };

                            debug!("Sending Market Stream request (ID: {}): {}", id, message_text);
                            if let Err(e) = write.send(Message::Text(message_text.into())).await { // Use message_text directly
                                error!("Failed to send Market Stream message (ID: {}): {}", id, e);
                                if let Some(tx) = response_tx_opt { // Use response_tx_opt here
                                    let _ = tx.send(Err(format!("Failed to send WS message: {}", e)));
                                }
                                need_reconnect = true;
                                continue;
                            }
                            if let Some(tx) = response_tx_opt { // Use response_tx_opt here
                                pending_requests.insert(id, tx);
                            }
                        } else {
                            info!("Market Stream request channel closed. Exiting listener.");
                            need_reconnect = true;
                        }
                    },
                    // Handle incoming messages from the WebSocket
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                debug!("Received Market Stream message: {}", text);
                                match serde_json::from_str::<BinanceWsMessage>(&text) {
                                    Ok(parsed_msg) => {
                                        match parsed_msg {
                                            BinanceWsMessage::Result(res) => {
                                                if let Some(response_tx) = pending_requests.remove(&res.id) {
                                                    let _ = response_tx.send(Ok(res.result.unwrap_or_default()));
                                                } else {
                                                    warn!("Received unmatched SubscriptionResult (ID: {}): {:#?}", res.id, res);
                                                }
                                            },
                                            BinanceWsMessage::Error(err) => {
                                                if let Some(id) = err.id {
                                                    if let Some(response_tx) = pending_requests.remove(&id) {
                                                        let _ = response_tx.send(Err(format!("Market Stream Error (ID: {}): {}", id, err.msg)));
                                                    } else {
                                                        error!("Received unmatched WsError (ID: {}): {:#?}", id, err);
                                                    }
                                                } else {
                                                    error!("Received WsError without ID: {:#?}", err);
                                                }
                                            },
                                            // For actual stream data, send it to the consumer
                                            BinanceWsMessage::StreamData { stream, data } => {
                                                if let Err(e) = data_sender.send(BinanceWsMessage::StreamData { stream, data }).await {
                                                    error!("Failed to send stream data to consumer: {}", e);
                                                    // If consumer channel is closed, we might want to exit or reconnect
                                                    need_reconnect = true; // Consider consumer drop as a reason to reconnect or stop
                                                }
                                            },
                                            BinanceWsMessage::Raw(raw_val) => {
                                                // Handle raw unparsed messages, potentially send to consumer if generic handling is desired
                                                if let Err(e) = data_sender.send(BinanceWsMessage::Raw(raw_val)).await {
                                                    error!("Failed to send raw stream data to consumer: {}", e);
                                                    need_reconnect = true;
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => error!("Failed to parse Market Stream message as BinanceWsMessage: {} from text: {}", e, text),
                                }
                            },
                            Some(Ok(Message::Binary(_))) => {
                                debug!("Received Market Stream binary message (ignored)");
                            },
                            Some(Ok(Message::Frame(_))) => { // Added missing match arm for Message::Frame
                                debug!("Received Market Stream frame message (ignored)");
                            },
                            Some(Ok(Message::Ping(data))) => {
                                debug!("Received Market Stream Ping: {:?}", data);
                                // tokio-tungstenite automatically sends Pong for Ping
                            },
                            Some(Ok(Message::Pong(data))) => {
                                debug!("Received Market Stream Pong: {:?}", data);
                            },
                            Some(Ok(Message::Close(close_frame))) => {
                                info!("Market Stream connection closed by server: {:?}", close_frame);
                                need_reconnect = true;
                            },
                            Some(Err(e)) => {
                                error!("Market Stream read error: {}", e);
                                need_reconnect = true;
                            },
                            None => {
                                info!("Market Stream ended. Reconnecting...");
                                need_reconnect = true;
                            },
                        }
                    },
                    // Add a timeout for connection re-establishment or inactivity
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {
                        warn!("Market Stream connection inactive for 60 seconds, attempting reconnect.");
                        need_reconnect = true;
                    }
                }
            }
            if need_reconnect {
                ws_stream_opt = None;
                // On reconnect, clear pending requests as their channels might be stale
                for (_, tx) in pending_requests.drain() {
                    let _ = tx.send(Err("WebSocket connection lost during request.".to_string()));
                }
            }
        }
    }

    /// Helper to send a request to the WebSocket stream listener and await its response.
    async fn send_stream_request(&self, request: WsStreamRequest) -> Result<Value, String> {
        let (response_tx, response_rx) = oneshot::channel();
        let request_with_tx = match request {
            WsStreamRequest::Subscribe { id, streams, .. } => WsStreamRequest::Subscribe { id, streams, response_tx },
            WsStreamRequest::Unsubscribe { id, streams, .. } => WsStreamRequest::Unsubscribe { id, streams, response_tx },
            WsStreamRequest::ListSubscriptions { id, .. } => WsStreamRequest::ListSubscriptions { id, response_tx },
            WsStreamRequest::SetProperty { id, property, value, .. } => WsStreamRequest::SetProperty { id, property, value, response_tx },
            WsStreamRequest::GetProperty { id, property, .. } => WsStreamRequest::GetProperty { id, property, response_tx },
            WsStreamRequest::SendRawMessage { .. } => return Err("SendRawMessage does not expect a response.".to_string()),
        };

        self.ws_stream_request_sender.send(request_with_tx).await
            .map_err(|e| format!("Failed to send stream request to listener: {}", e))?;

        response_rx.await
            .map_err(|e| format!("Failed to receive response from stream listener: {}", e))?
    }

    /// Subscribes to one or more public market data streams.
    ///
    /// # Arguments
    /// * `streams` - A vector of stream names (e.g., `["btcusdt@kline_1m", "bnbusdt@aggTrade"]`).
    ///
    /// # Returns
    /// A `Result` containing the API response `Value` on success, or a `String` error.
    pub async fn subscribe(&self, streams: Vec<String>) -> Result<Value, String> {
        let id = self.get_next_request_id();
        self.send_stream_request(WsStreamRequest::Subscribe { id, streams, response_tx: oneshot::channel().0 }).await
    }

    /// Unsubscribes from one or more public market data streams.
    ///
    /// # Arguments
    /// * `streams` - A vector of stream names to unsubscribe from.
    ///
    /// # Returns
    /// A `Result` containing the API response `Value` on success, or a `String` error.
    pub async fn unsubscribe(&self, streams: Vec<String>) -> Result<Value, String> {
        let id = self.get_next_request_id();
        self.send_stream_request(WsStreamRequest::Unsubscribe { id, streams, response_tx: oneshot::channel().0 }).await
    }

    /// Lists the currently active subscriptions for this WebSocket connection.
    ///
    /// # Returns
    /// A `Result` containing a `Value` representing the list of subscribed streams, or a `String` error.
    pub async fn list_subscriptions(&self) -> Result<Value, String> {
        let id = self.get_next_request_id();
        self.send_stream_request(WsStreamRequest::ListSubscriptions { id, response_tx: oneshot::channel().0 }).await
    }

    /// Sets a property for the WebSocket connection (e.g., `combined`).
    ///
    /// # Arguments
    /// * `property` - The name of the property to set (e.g., "combined").
    /// * `value` - The value to set the property to (e.g., `json!(true)`).
    ///
    /// # Returns
    /// A `Result` containing the API response `Value` on success, or a `String` error.
    pub async fn set_property(&self, property: &str, value: Value) -> Result<Value, String> {
        let id = self.get_next_request_id();
        self.send_stream_request(WsStreamRequest::SetProperty { id, property: property.to_string(), value, response_tx: oneshot::channel().0 }).await
    }

    /// Retrieves the value of a property for the WebSocket connection (e.g., `combined`).
    ///
    /// # Arguments
    /// * `property` - The name of the property to get.
    ///
    /// # Returns
    /// A `Result` containing the property's `Value` on success, or a `String` error.
    pub async fn get_property(&self, property: &str) -> Result<Value, String> {
        let id = self.get_next_request_id();
        self.send_stream_request(WsStreamRequest::GetProperty { id, property: property.to_string(), response_tx: oneshot::channel().0 }).await
    }

    // Internal counter for generating unique request IDs for stream management
    // Note: This is a simplified approach. For production, consider an AtomicU64.
    fn get_next_request_id(&self) -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, Ordering::SeqCst)
    }
}

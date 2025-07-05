// src/websocket/mod.rs

//! This module provides the core WebSocket client for interacting with the Binance API.
//! It handles establishing and managing WebSocket connections for signed user API requests.
//! Public market data streams are handled by the `websocket_stream` module.

use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use url::Url;
use std::collections::{HashMap, BTreeMap}; // For managing pending requests and sorted params
use std::time::{SystemTime, UNIX_EPOCH}; // For timestamps in signed requests
use hmac::{Hmac, Mac}; // For HMAC signing
use sha2::Sha256; // For SHA256 hashing
use hex::encode; // For hex encoding the signature
use log::{info, error, debug, warn}; // For logging
use uuid::Uuid; // For generating unique request IDs

/// Represents a generic WebSocket message received from Binance.
/// This enum uses `untagged` to allow flexible deserialization based on message structure.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum BinanceWsMessage {
    /// A successful subscription/unsubscription response or generic API call result
    #[serde(rename_all = "camelCase")]
    Result(SubscriptionResult),
    /// An error message from the WebSocket server
    #[serde(rename_all = "camelCase")]
    Error(WsError),
    /// Data from a specific stream (e.g., aggTrade, kline, ticker, depth, user data)
    #[serde(rename_all = "camelCase")]
    StreamData {
        stream: String,
        data: Value, // Data will be further parsed based on 'stream'
    },
    /// Raw JSON value for unknown or unhandled messages
    Raw(Value),
}

/// Represents a successful subscription/unsubscription result or generic API call response.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SubscriptionResult {
    pub result: Option<Value>, // Can be null or an object
    pub id: u64, // Request ID
}

/// Represents an error message from the WebSocket server.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WsError {
    pub code: i64,
    pub msg: String,
    pub id: Option<u64>, // Optional request ID associated with the error
}

/// Enum to represent different types of WebSocket API requests that the listener task handles.
enum WsApiRequest {
    ApiCall {
        id: String,
        method: String,
        params: Option<Value>,
        response_tx: oneshot::Sender<Result<Value, String>>,
    },
}

/// Represents the WebSocket API Client.
/// This client manages a persistent WebSocket connection for signed API requests.
pub struct WebSocketClient {
    api_key: String,
    secret_key: String,
    ws_base_url_api: String, // Base URL for WebSocket API calls (signed requests like session.logon, account.status)
    // Channel for sending requests to the WebSocket API handler task
    ws_api_request_sender: mpsc::Sender<WsApiRequest>,
    // Handle to the WebSocket API listener task (for signed requests)
    _ws_api_listener_handle: JoinHandle<()>,
}

impl WebSocketClient {
    /// Creates a new WebSocketClient instance.
    ///
    /// # Arguments
    /// * `api_key` - Your Binance API Key.
    /// * `secret_key` - Your Binance Secret Key.
    /// * `ws_base_url_api` - The base URL for the WebSocket API for signed requests (e.g., "wss://testnet.binancefuture.com/ws-fapi/v1").
    ///
    /// # Returns
    /// A new `WebSocketClient` instance.
    pub async fn new(
        api_key: String,
        secret_key: String,
        ws_base_url_api: String,
    ) -> Self {
        let (ws_api_request_sender, ws_api_request_receiver) = mpsc::channel::<WsApiRequest>(100); // Buffer for WS API requests

        // Clone necessary parts to move into the spawned WebSocket API listener task
        let ws_api_base_url_clone = ws_base_url_api.clone();
        let api_key_clone = api_key.clone();
        let secret_key_clone = secret_key.clone();

        // Spawn the WebSocket API listener task
        let ws_api_listener_handle = tokio::spawn(async move {
            Self::run_websocket_api_listener(
                ws_api_request_receiver,
                ws_api_base_url_clone,
                api_key_clone,
                secret_key_clone,
            ).await;
        });

        Self {
            api_key,
            secret_key,
            ws_base_url_api,
            ws_api_request_sender,
            _ws_api_listener_handle: ws_api_listener_handle,
        }
    }

    /// Generates a Binance API signature using HMAC SHA256.
    ///
    /// # Arguments
    /// * `query_string` - The query string (parameters) to sign.
    fn sign_payload(&self, query_string: &str) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(query_string.as_bytes());
        encode(mac.finalize().into_bytes())
    }

    /// Sends a request over the WebSocket API connection and waits for its response.
    /// This method handles request ID generation, parameter signing, and response matching.
    ///
    /// # Arguments
    /// * `method` - The WebSocket API method (e.g., "session.logon", "v2/account.status").
    /// * `params` - Parameters for the method as a `serde_json::Value` object.
    ///
    /// # Returns
    /// A `Result` containing the parsed JSON `Value` of the result on success, or a `String` error.
    pub async fn request_websocket_api(&self, method: &str, mut params: Value) -> Result<Value, String> {
        let id = Uuid::new_v4().to_string(); // Generate unique ID for request

        // Add API key, timestamp, and signature to params for signed requests
        // The `session.logon` method also requires signing, as per docs.
        let requires_signature = method.starts_with("v2/") || method.ends_with("session.logon") || method.starts_with("order.");
        if requires_signature {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| format!("Failed to get timestamp: {}", e))?
                .as_millis();

            // Prepare parameters for signing: sort alphabetically and join
            // The `params` Value might contain numbers, which need to be converted to strings for signing.
            let mut signable_params: BTreeMap<String, String> = BTreeMap::new();
            if let Some(map) = params.as_object() {
                for (k, v) in map {
                    signable_params.insert(k.clone(), v.to_string().trim_matches('"').to_string());
                }
            }
            signable_params.insert("timestamp".to_string(), timestamp.to_string());
            signable_params.insert("apiKey".to_string(), self.api_key.clone());

            let query_string = signable_params.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<String>>()
                .join("&");

            let signature = self.sign_payload(&query_string);

            // Add the signed parameters back to the original `params` Value for the request payload
            if let Some(map) = params.as_object_mut() {
                map.insert("apiKey".to_string(), Value::String(self.api_key.clone()));
                map.insert("timestamp".to_string(), Value::Number(serde_json::Number::from(timestamp as i64)));
                map.insert("signature".to_string(), Value::String(signature));
            } else {
                return Err("Params must be a JSON object for signed requests".to_string());
            }
        }

        let (response_tx, response_rx) = oneshot::channel();
        let ws_req = WsApiRequest::ApiCall {
            id: id.clone(),
            method: method.to_string(),
            params: Some(params),
            response_tx,
        };

        self.ws_api_request_sender.send(ws_req).await
            .map_err(|e| format!("Failed to send WebSocket API request: {}", e))?;

        response_rx.await
            .map_err(|e| format!("Failed to receive WebSocket API response: {}", e))?
    }

    /// Dedicated task to manage the WebSocket API connection (for signed requests).
    /// This function is spawned and runs independently.
    async fn run_websocket_api_listener(
        mut ws_request_receiver: mpsc::Receiver<WsApiRequest>,
        ws_base_url_api: String,
        api_key: String, // Cloned for use in signing if necessary within listener
        secret_key: String, // Cloned for use in signing if necessary within listener
    ) {
        let mut pending_requests: HashMap<String, oneshot::Sender<Result<Value, String>>> = HashMap::new();
        let mut ws_stream_opt = None;
        let mut timeout_reconnect = false;

        // Helper to sign payload within the listener task if needed (e.g., for internal pings/pongs with custom payloads)
        let _sign_payload_internal = |query_string: &str, secret: &str| -> String {
            type HmacSha256 = Hmac<Sha256>;
            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .expect("HMAC can take key of any size");
            mac.update(query_string.as_bytes());
            encode(mac.finalize().into_bytes())
        };

        loop {
            // Reconnect if stream is not established or disconnected
            if ws_stream_opt.is_none() {
                info!("Attempting to connect to WebSocket API at {}", ws_base_url_api);
                match connect_async(&ws_base_url_api).await {
                    Ok((ws_stream, _)) => {
                        info!("WebSocket API connection established.");
                        ws_stream_opt = Some(ws_stream);
                    },
                    Err(e) => {
                        error!("Failed to connect to WebSocket API: {}. Retrying in 5 seconds...", e);
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
                        if let Some(WsApiRequest::ApiCall { id, method, params, response_tx }) = req {
                            let request_payload = serde_json::json!({
                                "id": id.clone(),
                                "method": method,
                                "params": params.unwrap_or_default(),
                            });
                            let message = Message::Text(request_payload.to_string().into());
                            debug!("Sending WS API request: {}", request_payload);
                            if let Err(e) = write.send(message).await {
                                error!("Failed to send WebSocket API message: {}", e);
                                // If sending fails, notify the caller immediately
                                let _ = response_tx.send(Err(format!("Failed to send WS API message: {}", e)));
                                need_reconnect = true;
                                continue;
                            }
                            pending_requests.insert(id, response_tx);
                        } else {
                            // Channel closed, listener should probably exit
                            info!("WebSocket API request channel closed. Exiting listener.");
                            need_reconnect = true;
                        }
                    },
                    // Handle incoming messages from the WebSocket
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                debug!("Received WS API message: {}", text);
                                match serde_json::from_str::<Value>(&text) {
                                    Ok(json_value) => {
                                        if let Some(id_val) = json_value.get("id") {
                                            // Handle cases where ID can be null or string/int as per docs
                                            let id = if let Some(s) = id_val.as_str() {
                                                s.to_string()
                                            } else if let Some(num) = id_val.as_u64() {
                                                num.to_string()
                                            } else {
                                                // If ID is null or other unexpected type, treat as unmatched
                                                info!("Received WS API response with unexpected ID type: {}", text);
                                                continue;
                                            };

                                            if let Some(response_tx) = pending_requests.remove(&id) {
                                                // Binance WS API responses have 'status' (e.g., 200) for success, or 'error' object
                                                if json_value.get("status").and_then(|s| s.as_u64()) == Some(200) {
                                                    let _ = response_tx.send(Ok(json_value.get("result").cloned().unwrap_or_default()));
                                                } else {
                                                    let error_msg = json_value.get("error").and_then(|e| e.get("msg").and_then(|m| m.as_str())).unwrap_or("Unknown error").to_string();
                                                    let _ = response_tx.send(Err(format!("WebSocket API error: {}", error_msg)));
                                                }
                                            } else {
                                                // This is likely a market data stream message or an unsolicited response
                                                // For now, just log it. If specific streams are needed, add a callback mechanism.
                                                info!("Unmatched WS API response or stream data: {}", text);
                                            }
                                        } else {
                                            // Message without an 'id', likely a stream update (e.g., kline, trade from a combined stream)
                                            // This listener is primarily for API calls. If combined streams are used,
                                            // this part would need to dispatch to a separate market data handler.
                                            info!("Received unsolicited WS message (no ID): {}", text);
                                        }
                                    },
                                    Err(e) => error!("Failed to parse WebSocket API message as JSON: {} - {}", e, text),
                                }
                            },
                            Some(Ok(Message::Binary(_))) => {
                                debug!("Received WS API binary message (ignored)");
                            },
                            Some(Ok(Message::Frame(_))) => {
                                debug!("Received WS API frame message (ignored)");
                            },
                            Some(Ok(Message::Ping(data))) => {
                                debug!("Received Ping: {:?}", data);
                                // tokio-tungstenite automatically sends Pong for Ping
                            },
                            Some(Ok(Message::Pong(data))) => {
                                debug!("Received Pong: {:?}", data);
                            },
                            Some(Ok(Message::Close(close_frame))) => {
                                info!("WebSocket API connection closed by server: {:?}", close_frame);
                                need_reconnect = true;
                            },
                            Some(Err(e)) => {
                                error!("WebSocket API read error: {}", e);
                                need_reconnect = true;
                            },
                            None => {
                                // Stream ended, connection closed
                                info!("WebSocket API stream ended. Reconnecting...");
                                need_reconnect = true;
                            },
                        }
                    },
                    // Add a timeout for connection re-establishment or inactivity
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {
                        timeout_reconnect = true;
                    }
                }
            }
            if need_reconnect {
                ws_stream_opt = None;
            }
            if timeout_reconnect && ws_stream_opt.is_none() {
                warn!("WebSocket API connection not established for 60 seconds, attempting reconnect.");
            }
        }
    }

    /// Authenticates the WebSocket API connection using `session.logon`.
    /// This is often the first signed request after establishing the WS connection.
    ///
    /// # Returns
    /// A `Result` containing the logon response `Value` on success, or a `String` error.
    pub async fn session_logon(&self) -> Result<Value, String> {
        info!("Attempting WebSocket session logon...");
        let params = serde_json::json!({}); // Params will be filled by request_websocket_api with apiKey, timestamp, signature
        self.request_websocket_api("session.logon", params).await
    }
}

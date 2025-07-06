
use axum::{
    routing::post,
    extract::{State, Json},
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use log::{info, error, debug};
use crate::websocket::WebSocketClient; // Still in AppState for potential future use or context
use std::sync::Arc;

/// Represents the expected JSON payload from a TradingView webhook alert.
/// You MUST configure your TradingView alert message to match this structure.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")] // Use camelCase for JSON fields
pub struct WebhookPayload {
    
    pub signal: String, // e.g., "buy", "sell", "close_long", "close_short"

}

/// The shared state for the Axum application.
/// This allows webhook handlers to access necessary clients (like WebSocketClient).
#[derive(Clone)]
pub struct AppState {
    pub ws_client: Arc<WebSocketClient>, // Wrapped in Arc for shared ownership
// Secret to validate incoming webhooks
}

async fn handle_webhook(
    State(state): State<AppState>,
    Json(payload): Json<WebhookPayload>,
) -> String {

    println!("Received webhook payload: {:?}", payload);

    "Webhook received and acknowledged.".to_string()
}

pub async fn run_webhook_listener(
    ws_client: WebSocketClient,
    listen_addr: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let app_state = AppState {
        ws_client: Arc::new(ws_client)
    };

    let app = Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    info!("TradingView Webhook listener starting on http://{}", listen_addr);

    axum::serve(listener, app).await?;

    Ok(())
}

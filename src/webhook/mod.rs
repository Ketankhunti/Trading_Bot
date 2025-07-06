// src/webhook_listener/mod.rs

//! This module provides an HTTP server to listen for TradingView webhook alerts.
//! It parses incoming JSON payloads and dispatches trading signals.
//! Upon receiving a buy/sell signal, it fetches the current market price and places a market order.
//! The webhook payload is simplified to only include symbol and signal, and secret validation is removed for now.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    routing::post,
    extract::{State, Json},
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use log::{debug, error, info, warn};

use crate::order::{OrderSide, OrderType, TimeInForce};
use crate::websocket::WebSocketClient; // To send orders to Binance via WS API
use crate::rest_api::RestClient; // To fetch current market price via REST API


#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")] // Use camelCase for JSON fields
pub struct WebhookPayload {
    pub symbol: String,
    pub signal: String, // e.g., "buy", "sell", "close_long", "close_short"
}

/// The shared state for the Axum application.
/// This allows webhook handlers to access both WebSocketClient and RestClient.
#[derive(Clone)]
pub struct AppState {
    pub ws_client: Arc<WebSocketClient>,
    pub rest_client: Arc<RestClient> // Added RestClient to AppState
    // pub webhook_secret: String, // Removed webhook_secret for now
}


async fn handle_webhook(
    State(state): State<AppState>,
    Json(payload): Json<WebhookPayload>,
) -> String {
    println!("Received webhook payload: {:?}", payload);

    let current_price_res = state.rest_client.get_current_price(&payload.symbol).await;
    let current_price = match current_price_res {
        Ok(ticker_price) => ticker_price.price.parse::<f64>().unwrap_or_default(),
        Err(e) => {
            error!("Failed to get current price for {}: {}", payload.symbol, e);
            return format!("Error: Could not get current price for {}", payload.symbol);
        }
    };
    if current_price <= 0.0 {
        error!("Fetched invalid current price for {}: {}", payload.symbol, current_price);
        return format!("Error: Invalid current price for {}", payload.symbol);
    }
    println!("Current market price for {}: {}", payload.symbol, current_price);

    // Determine quantity to trade. Using a fixed default quantity for now.
    // IMPORTANT: Adjust this default quantity based on your strategy and minimum notional values.
    let quantity_to_trade = 0.04; // Reduced quantity to fit within available balance (~4,740 USDT)

    // Ensure minimum notional value (e.g., 5 USDT for Binance Futures)
    let min_notional = 5.0; // This should ideally be fetched from exchange info
    if (quantity_to_trade * current_price) < min_notional {
        error!("Calculated notional value ({:.4}) for {} is below minimum {}. Order not placed.",
               quantity_to_trade * current_price, payload.symbol, min_notional);
        return format!("Error: Notional value too small ({:.4})", quantity_to_trade * current_price);
    }

    // Generate a short, unique client order ID using timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    // Use only last 6 digits of timestamp to keep ID short
    let short_timestamp = timestamp % 1000000;
    let client_order_id = format!("wh{}{}", payload.signal.chars().next().unwrap_or('x'), short_timestamp);

    // 3. Dispatch the order using WebSocketClient (Market Order)
    let order_result = match payload.signal.to_lowercase().as_str() {
        "buy" => {
            println!("Placing MARKET BUY order for {} quantity {} at price {}", payload.symbol, quantity_to_trade, current_price);
            state.ws_client.new_order(
                &payload.symbol,
                OrderSide::Buy,
                OrderType::Market, // Always a Market Order for this scenario
                quantity_to_trade,
                None, // No specific price for Market Order
                None, // No TimeInForce for Market Order (FOK/IOC might be implied by exchange for Market)
                Some(&client_order_id), // Use short client order ID
            ).await
        },
        "sell" => {
            println!("Placing MARKET SELL order for {} quantity {} at price {}", payload.symbol, quantity_to_trade, current_price);
            state.ws_client.new_order(
                &payload.symbol,
                OrderSide::Sell,
                OrderType::Market, // Always a Market Order for this scenario
                quantity_to_trade,
                None, // No specific price for Market Order
                None, // No TimeInForce for Market Order
                Some(&client_order_id), // Use short client order ID
            ).await
        },
        // You can add more complex signals here, e.g., to close positions
        "close_long" => {
            println!("Received CLOSE LONG signal for {}. Attempting to market sell current position.", payload.symbol);
            // In a real bot, you'd query your current position for 'symbol' and use that quantity
            // For simplicity, we'll assume a fixed quantity or rely on the webhook to send it.
            state.ws_client.new_order(
                &payload.symbol,
                OrderSide::Sell, // Sell to close a long position
                OrderType::Market,
                quantity_to_trade, // Using fixed quantity
                None,
                None,
                Some(&client_order_id), // Use short client order ID
            ).await
        },
        "close_short" => {
            println!("Received CLOSE SHORT signal for {}. Attempting to market buy current position.", payload.symbol);
            state.ws_client.new_order(
                &payload.symbol,
                OrderSide::Buy, // Buy to close a short position
                OrderType::Market,
                quantity_to_trade, // Using fixed quantity
                None,
                None,
                Some(&client_order_id), // Use short client order ID
            ).await
        },
        _ => {
            warn!("Received unknown signal: {}", payload.signal);
            return format!("Unknown signal: {}", payload.signal);
        }
    };

    match order_result {
        Ok(response) => {
            println!("Order placed successfully: {:?}", response);
            "Order placed successfully".to_string()
        },
        Err(e) => {
            error!("Failed to place order: {}", e);
            format!("Error placing order: {}", e)
        }
    }
}

pub async fn run_webhook_listener(
    ws_client: WebSocketClient,
    rest_client: RestClient, // Added RestClient
    listen_addr: &str,
    // webhook_secret: String, // Removed webhook_secret from arguments
) -> Result<(), Box<dyn std::error::Error>> {
    let app_state = AppState {
        ws_client: Arc::new(ws_client),
        rest_client: Arc::new(rest_client), // Pass RestClient to state
        // webhook_secret, // Removed webhook_secret from state initialization
    };

    let app = Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    info!("TradingView Webhook listener starting on http://{}", listen_addr);

    axum::serve(listener, app).await?;

    Ok(())
}

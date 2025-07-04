mod rest_client;
mod account_info;
mod tui_display;
mod order;
mod websocket;

use crate::rest_client::RestClient;
use crate::account_info::{AccountInfo, AssetBalance, PositionInfo};
use crate::order::*;
use tui_display::*;
use websocket::*;
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set in .env");
    let secret_key = env::var("BINANCE_SECRET_KEY").expect("BINANCE_SECRET_KEY not set in .env");
    let rest_base_url = env::var("BINANCE_REST_BASE_URL").unwrap_or_else(|_| "https://testnet.binancefuture.com".to_string());

    let client = RestClient::new(
        api_key.clone(),
        secret_key.clone(),
        rest_base_url,
    );

    println!("--- Testing Binance Futures API Client ---");

    match client.get_account_info().await {
        Ok(account_info) => {
            // Account info fetched successfully
        },
        Err(e) => eprintln!("Error fetching account info: {}", e),
    }

    println!("\nFetching Market Data (will display in TUI)...");

    println!("\nFetching Historical Orders (will display in TUI)...");

    match client.get_all_orders("BTCUSDT", None, Some(10)).await {
        Ok(orders) => {
            // Historical orders fetched successfully
        },
        Err(e) => eprintln!("Error getting all orders: {}", e),
    }

    println!("\nFetching Open Orders (will display in TUI)...");
    match client.get_open_orders(None).await {
        Ok(orders) => {
            display_struct_in_tui(&orders, "All Open Orders").await?;
        },
        Err(e) => eprintln!("Error getting open orders: {}", e),
    }

    let example_order_id = 5191956932;
    match client.query_order("BTCUSDT", Some(example_order_id), None).await {
        Ok(order) => {
            display_struct_in_tui(&order, &format!("Order ID: {}", example_order_id)).await?;
        },
        Err(e) => eprintln!("Error querying order {}: {}", example_order_id, e),
    }

    let ws_client = WebSocketClient::new(
        api_key,
        secret_key,
        "wss://testnet.binancefuture.com/ws-fapi/v1".to_string()
    ).await;

    match ws_client.new_order(
        "BNBUSDT",
        OrderSide::Buy,
        OrderType::Limit,
        0.02,
        Some(300.0),
        Some(TimeInForce::Gtc),
        Some("my_ws_test_order_123"),
    ).await {
        Ok(response) => {
            display_struct_in_tui(&response, "New WebSocket Order Placed").await?;
        },
        Err(e) => eprintln!("Error placing new WebSocket order (check testnet balance/rules): {}", e),
    }

    let order_id_to_cancel = 541097441;
    match ws_client.cancel_order(
        "BNBUSDT",
        Some(order_id_to_cancel),
        Some("my_ws_test_order_123"),
    ).await {
        Ok(cancel_response) => {
            display_struct_in_tui(&cancel_response, &format!("Canceled Order ID: {}", order_id_to_cancel)).await?;
        },
        Err(e) => eprintln!("Error canceling WebSocket order {}: {}", order_id_to_cancel, e),
    }

    Ok(())
}
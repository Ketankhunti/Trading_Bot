mod rest_client;
mod account_info;
mod tui_display;
mod order;


use crate::rest_client::RestClient;
use crate::account_info::{AccountInfo, AssetBalance, PositionInfo}; // Import specific structs
use crate::order::*;
use tui_display::*;
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    dotenv().ok();
    let api_key = env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set in .env");
    let secret_key = env::var("BINANCE_SECRET_KEY").expect("BINANCE_SECRET_KEY not set in .env");
    let rest_base_url = env::var("BINANCE_REST_BASE_URL").unwrap_or_else(|_| "https://testnet.binancefuture.com".to_string());

    // Initialize the BinanceClient with REST API base URL
    let client = RestClient::new(
        api_key,
        secret_key,
        rest_base_url,
    );

    println!("--- Testing Binance Futures API Client ---");

    // --- Test Account Info Module ---
    // println!("\nFetching Account Information (will display in TUI)...");
    match client.get_account_info().await {
        Ok(account_info) => {
            // Display the entire AccountInfo struct in the TUI
            
            display_struct_in_tui(&account_info, "Binance Futures Account Info").await?;
        },
        Err(e) => eprintln!("Error fetching account info: {}", e),
    }

    // --- Test Market Data Module ---
    println!("\nFetching Market Data (will display in TUI)...");

    // --- Test Order Module (REST for queries) ---
    println!("\nFetching Historical Orders (will display in TUI)...");

    // Test Get All Orders
    match client.get_all_orders("BTCUSDT", None, Some(10)).await {
        Ok(orders) => {
            print!("{:#?}",orders);
            display_struct_in_tui(&orders, "BTCUSDT All Historical Orders (Last 10)").await?;
        },
        Err(e) => eprintln!("Error getting all orders: {}", e),
    }

    // Test Get Open Orders
    println!("\nFetching Open Orders (will display in TUI)...");
    match client.get_open_orders(None).await { // Get all open orders
        Ok(orders) => {
            display_struct_in_tui(&orders, "All Open Orders").await?;
        },
        Err(e) => eprintln!("Error getting open orders: {}", e),
    }

    // Test Query Specific Order (requires an existing orderId or clientOrderId)
    // You'd typically get an order ID from a placed order or user data stream.
    // For demonstration, this will likely fail unless you manually place an order first.
    // println!("\nQuerying a Specific Order (will display in TUI)...");
    // let example_order_id: u64 = 0; // Replace with a real order ID from your testnet account
    // match rest_client.query_order("BTCUSDT", Some(example_order_id), None).await {
    //     Ok(order) => {
    //         display_struct_in_tui(&order, &format!("Order ID: {}", example_order_id)).await?;
    //     },
    //     Err(e) => eprintln!("Error querying order {}: {}", example_order_id, e),
    // }


    // --- Example: Placing an Order (WebSocket) ---
    // This part requires the WebSocketClient to be initialized and session.logon to be successful.
    // Uncomment and use with caution on testnet.
    /*
    println!("\nAttempting to place a new LIMIT BUY order (WebSocket API)...");
    // Ensure ws_client is initialized above:
    // let ws_client = WebSocketClient::new(...).await;

    // First, perform session logon if required by Binance Futures WS API for order methods
    // match ws_client.session_logon().await {
    //     Ok(logon_result) => println!("WebSocket Session Logon Result: {:#?}", logon_result),
    //     Err(e) => eprintln!("Error during WebSocket session logon: {}", e),
    // }

    // match ws_client.new_order(
    //     "BNBUSDT", // Symbol
    //     OrderSide::Buy,
    //     OrderType::Limit,
    //     0.01, // Quantity
    //     Some(300.0), // Price
    //     Some(TimeInForce::Gtc),
    //     Some("my_ws_test_order_123"),
    // ).await {
    //     Ok(response) => {
    //         display_struct_in_tui(&response, "New WebSocket Order Placed").await?;
    //     },
    //     Err(e) => eprintln!("Error placing new WebSocket order (check testnet balance/rules): {}", e),
    // }
    */

 Ok(())
}
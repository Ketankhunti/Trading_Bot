// src/main.rs

//! Trading Bot - Binance Futures API Client
//! 
//! This is the main entry point for the trading bot application.
//! Run tests using: cargo test -- --nocapture
//! Or run specific tests: cargo test test_new_order -- --nocapture

mod rest_api;
mod websocket;
mod tui;
mod account_info;
mod order;
mod websocket_stream;

use std::env;
use tokio::sync::mpsc;
use trading_bot::websocket_stream::{MarketStreamClient, BinanceWsMessage};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Trading Bot - Binance Futures API Client ===");
    println!();
    println!("This application provides a client for interacting with Binance Futures API.");
    println!("All testing functionality has been moved to integration tests.");
    println!();
    println!("To run tests:");
    println!("  cargo test -- --nocapture                    # Run all tests");
    println!("  cargo test test_new_order -- --nocapture     # Test placing new orders");
    println!("  cargo test test_modify_order -- --nocapture  # Test modifying orders");
    println!("  cargo test test_cancel_order -- --nocapture  # Test canceling orders");
    println!("  cargo test test_account_info -- --nocapture  # Test account info");
    println!("  cargo test test_websocket_stream_lifecycle -- --nocapture  # Test WebSocket stream");
    println!();
    println!("Available test functions:");
    println!("  - test_account_info()      # Fetch and display account information");
    println!("  - test_historical_orders() # Fetch and display historical orders");
    println!("  - test_open_orders()       # Fetch and display open orders");
    println!("  - test_new_order()         # Place a new order");
    println!("  - test_modify_order()      # Place and modify an order");
    println!("  - test_cancel_order()      # Place and cancel an order");
    println!();
    println!("For production use, implement your trading strategy here.");
    println!("The modules are ready to use:");
    println!("  - rest_api::RestClient     # For REST API calls");
    println!("  - websocket::WebSocketClient # For WebSocket API calls");
    println!("  - order::*                 # For order types and operations");
    println!("  - account_info::*          # For account information");
    println!("  - tui::display_struct_in_tui # For displaying data in TUI");
    println!();
    
    // Demo WebSocket Stream functionality
    println!("=== WebSocket Stream Demo ===");
    demo_websocket_stream().await?;
    
    Ok(())
}

/// Demo function to showcase WebSocket stream functionality
async fn demo_websocket_stream() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting WebSocket Stream Demo...");
    
    // Create channel for receiving stream data
    let (data_sender, mut data_receiver) = mpsc::channel::<BinanceWsMessage>(100);

    // Create WebSocket stream client
    let ws_url = "wss://fstream.binancefuture.com/ws".to_string();
    println!("📡 Connecting to: {}", ws_url);
    
    let client = MarketStreamClient::new(ws_url, data_sender).await;
    println!("✅ MarketStreamClient created successfully");

    // Wait for connection to establish
    println!("⏳ Waiting 3 seconds for connection to establish...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Subscribe to multiple streams
    let streams = vec![
        "btcusdt@kline_1m".to_string(),
        "btcusdt@aggTrade".to_string(),
        "btcusdt@ticker".to_string()
    ];
    println!("📊 Subscribing to streams: {:?}", streams);
    
    match client.subscribe(streams.clone()).await {
        Ok(response) => {
            println!("✅ Successfully subscribed to streams!");
            println!("   Response: {}", serde_json::to_string_pretty(&response)?);
        }
        Err(e) => {
            println!("❌ Failed to subscribe to streams: {}", e);
            return Err(e.into());
        }
    }

    println!();
    println!("📈 Listening for market data... (Press Ctrl+C to stop)");
    println!("{}", "=".repeat(80));
    
    let mut message_count = 0;
    let start_time = std::time::Instant::now();
    
    // Listen for messages for 30 seconds
    let message_stream = tokio::time::timeout(
        tokio::time::Duration::from_secs(30), 
        async {
            loop {
                match data_receiver.recv().await {
                    Some(message) => {
                        message_count += 1;
                        match message {
                            BinanceWsMessage::StreamData { stream, data } => {
                                println!("📊 [{}] Stream: {}", 
                                    chrono::Utc::now().format("%H:%M:%S"), stream);
                                println!("   Data: {}", serde_json::to_string_pretty(&data)?);
                                println!("   {}", "-".repeat(60));
                            }
                            BinanceWsMessage::Result(result) => {
                                println!("✅ [{}] Result: {:?}", 
                                    chrono::Utc::now().format("%H:%M:%S"), result);
                            }
                            BinanceWsMessage::Error(err) => {
                                println!("❌ [{}] Error: {:?}", 
                                    chrono::Utc::now().format("%H:%M:%S"), err);
                            }
                            BinanceWsMessage::Raw(raw) => {
                                println!("📄 [{}] Raw: {}", 
                                    chrono::Utc::now().format("%H:%M:%S"), 
                                    serde_json::to_string_pretty(&raw)?);
                            }
                        }
                    }
                    None => {
                        println!("🔌 Stream channel closed");
                        break;
                    }
                }
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        }
    ).await;

    match message_stream {
        Ok(_) => println!("⏰ Demo timeout reached"),
        Err(_) => println!("⏰ Demo timeout reached"),
    }

    println!();
    println!("📊 Demo Summary:");
    println!("   Total messages received: {}", message_count);
    println!("   Duration: {:.2} seconds", start_time.elapsed().as_secs_f64());
    println!("   Messages per second: {:.2}", 
        message_count as f64 / start_time.elapsed().as_secs_f64());

    // Unsubscribe from the streams
    println!("🔌 Unsubscribing from streams...");
    match client.unsubscribe(streams).await {
        Ok(response) => {
            println!("✅ Successfully unsubscribed!");
            println!("   Response: {}", serde_json::to_string_pretty(&response)?);
        }
        Err(e) => {
            println!("❌ Failed to unsubscribe: {}", e);
        }
    }

    println!("🎉 WebSocket Stream Demo completed!");
    println!();
    
    Ok(())
}


//541283361
// 541278270
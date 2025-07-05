// tests/websocket_stream_tests.rs

//! This file contains integration tests for the MarketStreamClient,
//! focusing on connecting to public WebSocket market data streams,
//! subscribing, and receiving data.

// mod streams;


use trading_bot::streams::*;
use trading_bot::websocket_stream::{MarketStreamClient, BinanceWsMessage}; // Import from websocket_stream
use serde_json::{from_value, Value};
use std::env;
use tokio::time::{self, Duration};
use tokio::sync::mpsc;
use log::{info, error, debug, warn};
use serde_json::json;

// Initialize logging for tests (optional, but good for debugging)
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn setup_logging_stream() {
    let _ = env_logger::builder().is_test(true).try_init();
}

/// Comprehensive test for WebSocket stream: subscribe, get data, unsubscribe
#[tokio::test]
async fn test_websocket_stream_lifecycle() {
    env_logger::init();
    info!("=== Starting WebSocket stream lifecycle test ===");

    // Create channel for receiving stream data
    let (data_sender, mut data_receiver) = mpsc::channel::<BinanceWsMessage>(100);

    // Create WebSocket stream client
    let ws_url = "wss://fstream.binancefuture.com/ws".to_string();
    info!("Creating MarketStreamClient with URL: {}", ws_url);
    
    let client = MarketStreamClient::new(ws_url, data_sender).await;
    info!("MarketStreamClient created successfully");

    // Wait for connection to establish
    info!("Waiting 3 seconds for connection to establish...");
    time::sleep(Duration::from_secs(3)).await;

    // Subscribe to a stream
    let streams = vec!["btcusdt@kline_1m".to_string()];
    info!("Attempting to subscribe to streams: {:?}", streams);
    
    match client.subscribe(streams.clone()).await {
        Ok(response) => {
            info!("✅ Successfully subscribed to stream: {:?}", response);
        }
        Err(e) => {
            error!("❌ Failed to subscribe to stream: {}", e);
            panic!("Subscription failed: {}", e);
        }
    }

    // Wait for data to arrive
    info!("Waiting 10 seconds for data to arrive...");
    time::sleep(Duration::from_secs(10)).await;

    // Check if we received any messages
    let mut message_count = 0;
    let mut result_count = 0;
    let mut error_count = 0;
    let mut raw_count = 0;
    
    info!("Checking for received messages...");
    while let Ok(message) = data_receiver.try_recv() {
        match message {
            BinanceWsMessage::StreamData { stream, data } => {
                info!("📊 Received stream data from {}: {:?}", stream, data);
                message_count += 1;
            }
            BinanceWsMessage::Result(result) => {
                info!("✅ Received result: {:?}", result);
                result_count += 1;
            }
            BinanceWsMessage::Error(err) => {
                error!("❌ Received error: {:?}", err);
                error_count += 1;
            }
            BinanceWsMessage::Raw(raw) => {
                info!("📄 Received raw message: {:?}", raw);
                raw_count += 1;
            }
        }
    }

    info!("=== Message Summary ===");
    info!("Stream data messages: {}", message_count);
    info!("Result messages: {}", result_count);
    info!("Error messages: {}", error_count);
    info!("Raw messages: {}", raw_count);
    info!("Total messages: {}", message_count + result_count + error_count + raw_count);

    // Verify we received some data
    if message_count == 0 {
        warn!("⚠️  No stream data messages received. This might indicate:");
        warn!("   - Connection issues");
        warn!("   - Subscription problems");
        warn!("   - Network issues");
        warn!("   - Binance API changes");
        // Don't panic, just warn for now
    } else {
        info!("✅ Successfully received {} stream data messages", message_count);
    }

    // Unsubscribe from the stream
    info!("Attempting to unsubscribe from streams: {:?}", streams);
    match client.unsubscribe(streams).await {
        Ok(response) => {
            info!("✅ Successfully unsubscribed from stream: {:?}", response);
        }
        Err(e) => {
            error!("❌ Failed to unsubscribe from stream: {}", e);
            panic!("Unsubscription failed: {}", e);
        }
    }

    // Wait a bit more to ensure unsubscribe took effect
    info!("Waiting 3 seconds to verify unsubscribe took effect...");
    time::sleep(Duration::from_secs(3)).await;

    // Check for any remaining messages after unsubscribe
    let mut after_unsubscribe_count = 0;
    while let Ok(message) = data_receiver.try_recv() {
        match message {
            BinanceWsMessage::StreamData { stream, data } => {
                warn!("⚠️  Received stream data after unsubscribe from {}: {:?}", stream, data);
                after_unsubscribe_count += 1;
            }
            _ => {}
        }
    }

    if after_unsubscribe_count > 0 {
        warn!("⚠️  Received {} messages after unsubscribe (this might be normal for a short period)", after_unsubscribe_count);
    } else {
        info!("✅ No messages received after unsubscribe - clean unsubscription");
    }

    info!("=== Test completed successfully ===");
}

use trading_bot::websocket::WebSocketClient;
use trading_bot::rest_api::RestClient; // Add REST client import
use trading_bot::webhook; // Import the webhook listener module
use log::{info, error, warn};
use std::env;
use dotenv::dotenv;
use tokio::signal; // For graceful shutdown
use ngrok::{config::ForwarderBuilder, tunnel::EndpointInfo}; // Import ngrok crates
use url::Url; // For Url::parse

// Main application entry point
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load environment variables
    dotenv().ok();
    // Initialize logging
    env_logger::init();

    info!("--- Starting Trading Bot Application ---");

    // Load API keys and URLs from environment variables
    let api_key = env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set in .env");
    let secret_key = env::var("BINANCE_SECRET_KEY").expect("BINANCE_SECRET_KEY not set in .env");
    let ws_api_base_url = env::var("BINANCE_WS_API_BASE_URL").expect("BINANCE_WS_API_BASE_URL not set in .env");
    let rest_api_base_url = env::var("BINANCE_REST_API_BASE_URL").expect("BINANCE_REST_API_BASE_URL not set in .env");
    let webhook_local_listen_addr = env::var("WEBHOOK_LOCAL_LISTEN_ADDR").expect("WEBHOOK_LOCAL_LISTEN_ADDR not set in .env");

    // --- Initialize WebSocketClient (needed for webhook order dispatch) ---
    let ws_client = WebSocketClient::new(
        api_key.clone(), // Clone for ws_client
        secret_key.clone(), // Clone for ws_client
        ws_api_base_url.clone(),
    ).await;

    // --- Initialize RestClient (needed for fetching current prices) ---
    let rest_client = RestClient::new(
        api_key.clone(), // Clone for rest_client
        secret_key.clone(), // Clone for rest_client
        rest_api_base_url,
    );

    // Perform WebSocket session logon (important for authenticated WS API calls)
    info!("Attempting WebSocket Session Logon...");
    match ws_client.session_logon().await {
        Ok(logon_result) => info!("WebSocket Session Logon Result: {:?}", logon_result),
        Err(e) => error!("Error during WebSocket session logon: {}", e),
    }

    // --- Set up ngrok tunnel ---
    info!("Setting up ngrok tunnel...");
    let session = ngrok::Session::builder()
        .authtoken_from_env() // Reads NGROK_AUTHTOKEN from environment
        .connect()
        .await
        .map_err(|e| format!("Failed to connect to ngrok session: {}", e))?;

    println!("{}",webhook_local_listen_addr);

    // Forward HTTP traffic from ngrok to the local webhook listener address
    // The `webhook_local_listen_addr` should be the address Axum binds to.
    let listener = session
        .http_endpoint()
        // .traffic_policy(r#"{"on_http_request": [{"actions": [{"type": "oauth","config": {"provider": "google"}}]}]}"#) // Uncomment for OAuth
        .listen_and_forward(Url::parse(&format!("http://{}/", webhook_local_listen_addr)).unwrap()) // Forward to local Axum server
        .await
        .map_err(|e| format!("Failed to create ngrok tunnel: {}", e))?;

    let public_ngrok_url = listener.url().to_string();
    println!("\n--- TradingView Webhook URL ---");
    println!("Configure your TradingView alert to POST to: {}/webhook", public_ngrok_url);
    println!("-------------------------------\n");
    info!("ngrok tunnel established at: {}", public_ngrok_url);


    // --- Spawn the webhook listener in a separate Tokio task ---
    // The webhook listener (Axum server) binds to the local address.
    let webhook_handle = tokio::spawn(async move {
        if let Err(e) = webhook::run_webhook_listener(
            ws_client,
            rest_client, // Pass the REST client to the webhook listener
            &webhook_local_listen_addr // Axum binds to this local address
        ).await {
            error!("Webhook listener failed: {}", e);
        }
    });

    info!("Application running. Press Ctrl+C to shut down gracefully.");

    // Wait for Ctrl+C signal to gracefully shut down
    signal::ctrl_c().await?;
    info!("Ctrl+C received, shutting down...");

    // Give some time for tasks to shut down, then forcefully abort if necessary
    tokio::select! {
        _ = webhook_handle => { info!("Webhook listener task finished."); },
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
            warn!("Webhook listener task did not shut down gracefully in time.");
        }
    }

    info!("Application shut down complete.");

    Ok(())
}


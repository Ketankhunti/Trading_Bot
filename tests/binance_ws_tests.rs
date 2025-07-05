use trading_bot::rest_api::RestClient;
use trading_bot::websocket::WebSocketClient;
use trading_bot::order::{OrderSide, OrderType, TimeInForce};
use trading_bot::tui::display_struct_in_tui;

const API_KEY: &str = "ae01d811bd0704d1fe996f9c1ea63ed241a4a7373ad6bbeafd8ac13e9bf5a5ec";
const SECRET_KEY: &str = "92f455172c46236d33e9ff6a505403d735937885a90c0f819738475bc6672c0c";
const REST_BASE_URL: &str = "https://testnet.binancefuture.com";
const WS_API_BASE_URL: &str = "wss://testnet.binancefuture.com/ws-fapi/v1";

#[tokio::test]
async fn test_account_info() {
    let rest_client = RestClient::new(
        API_KEY.to_string(),
        SECRET_KEY.to_string(),
        REST_BASE_URL.to_string(),
    );
    let account_info = rest_client.get_account_info().await.expect("Failed to fetch account info");
    display_struct_in_tui(&account_info, "Binance Futures Account Info (REST)").await.unwrap();
}

#[tokio::test]
async fn test_historical_orders() {
    let rest_client = RestClient::new(
        API_KEY.to_string(),
        SECRET_KEY.to_string(),
        REST_BASE_URL.to_string(),
    );
    let orders = rest_client.get_all_orders("BTCUSDT", None, Some(10)).await.expect("Failed to get all orders");
    display_struct_in_tui(&orders, "BTCUSDT All Historical Orders (Last 10) (REST)").await.unwrap();
}

#[tokio::test]
async fn test_open_orders() {
    let rest_client = RestClient::new(
        API_KEY.to_string(),
        SECRET_KEY.to_string(),
        REST_BASE_URL.to_string(),
    );
    let orders = rest_client.get_open_orders(None).await.expect("Failed to get open orders");
    display_struct_in_tui(&orders, "All Open Orders (REST)").await.unwrap();
}

#[tokio::test]
async fn test_new_order() {
    let ws_client = WebSocketClient::new(
        API_KEY.to_string(),
        SECRET_KEY.to_string(),
        WS_API_BASE_URL.to_string(),
    ).await;
    
    let order_symbol = "BNBUSDT";
    let initial_price = 300.0;
    let initial_quantity = 0.02;
    
    println!("Placing new order...");
    let response = ws_client.new_order(
        order_symbol,
        OrderSide::Buy,
        OrderType::Limit,
        initial_quantity,
        Some(initial_price),
        Some(TimeInForce::Gtc),
        Some("test_new_order_123"),
    ).await.expect("Failed to place new order");
    
    display_struct_in_tui(&response, "New WebSocket Order Placed").await.unwrap();
    println!("Order placed successfully with ID: {}", response.order_id);
}

#[tokio::test]
async fn test_modify_order() {
    let rest_client = RestClient::new(
        API_KEY.to_string(),
        SECRET_KEY.to_string(),
        REST_BASE_URL.to_string(),
    );
    let ws_client = WebSocketClient::new(
        API_KEY.to_string(),
        SECRET_KEY.to_string(),
        WS_API_BASE_URL.to_string(),
    ).await;
    
    let order_symbol = "BNBUSDT";
    let initial_price = 300.0;
    let initial_quantity = 0.02;
    
    // First, place an order to modify
    println!("Placing order for modification test...");
    let response = ws_client.new_order(
        order_symbol,
        OrderSide::Buy,
        OrderType::Limit,
        initial_quantity,
        Some(initial_price),
        Some(TimeInForce::Gtc),
        Some("test_modify_order_123"),
    ).await.expect("Failed to place order for modification");
    
    let order_id = response.order_id;
    println!("Order placed with ID: {}", order_id);
    
    // Wait for order to be active
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Query order status
    let order_status = rest_client.query_order(order_symbol, Some(order_id), None)
        .await.expect("Failed to query order");
    
    display_struct_in_tui(&order_status, &format!("Order {} Status Before Modification", order_id)).await.unwrap();
    
    if order_status.status == "NEW" || order_status.status == "PARTIALLY_FILLED" {
        println!("Modifying order...");
        let new_price = 305.0;
        let new_quantity = 0.03;
        
        let modified_response = ws_client.modify_order(
            order_symbol,
            OrderSide::Buy,
            Some(order_id),
            None,
            Some(new_quantity),
            Some(new_price),
            None, None, None,
            Some("test_modify_order_123_amend"),
        ).await.expect("Failed to modify order");
        
        display_struct_in_tui(&modified_response, &format!("Modified Order ID: {}", order_id)).await.unwrap();
        println!("Order modified successfully!");
    } else {
        println!("Order is not in a modifiable state (Status: {})", order_status.status);
    }
}

#[tokio::test]
async fn test_cancel_order() {
    let ws_client = WebSocketClient::new(
        API_KEY.to_string(),
        SECRET_KEY.to_string(),
        WS_API_BASE_URL.to_string(),
    ).await;
    
    let order_symbol = "BNBUSDT";
    let initial_price = 300.0;
    let initial_quantity = 0.02;
    
    // First, place an order to cancel
    println!("Placing order for cancellation test...");
    let response = ws_client.new_order(
        order_symbol,
        OrderSide::Buy,
        OrderType::Limit,
        initial_quantity,
        Some(initial_price),
        Some(TimeInForce::Gtc),
        Some("test_cancel_order_123"),
    ).await.expect("Failed to place order for cancellation");
    
    let order_id = response.order_id;
    println!("Order placed with ID: {}", order_id);
    
    // Wait for order to be active
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Cancel the order
    println!("Canceling order...");
    let cancel_response = ws_client.cancel_order(
        order_symbol,
        Some(order_id),
        None,
    ).await.expect("Failed to cancel order");
    
    display_struct_in_tui(&cancel_response, &format!("Canceled Order ID: {}", order_id)).await.unwrap();
    println!("Order canceled successfully!");
}

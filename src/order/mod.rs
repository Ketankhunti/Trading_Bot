// src/order/mod.rs

//! This module provides functionalities for querying orders on the Binance Futures API
//! using REST endpoints. These operations typically require authenticated (signed) requests.
//! Active order management (placement, cancellation) would be handled by a separate WebSocket client.

use serde::{Deserialize, Serialize};
use crate::rest_client::RestClient; // Import the RestClient for queries
use serde_json::{json, Value};  // Import Value for deserialization from generic JSON
use std::io; // Import std::io for io::Error and io::ErrorKind (for custom error messages)
use crate::websocket::WebSocketClient; // Import the WebSocketClient for order placement and cancellation

/// Enum representing the type of order.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Limit,
    Market,
    StopLoss,
    StopLossLimit,
    TakeProfit,
    TakeProfitLimit,
    LimitMaker,
}

/// Enum representing the side of the order (BUY or SELL).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Enum representing the time in force for an order.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeInForce {
    Gtc, // Good Till Cancel
    Ioc, // Immediate Or Cancel
    Fok, // Fill Or Kill
}

/// Represents the response received after placing a new order.
/// This struct maps to the response from `order.place` WebSocket API call
/// or `/fapi/v1/order` REST API call.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewOrderResponse {
    pub symbol: String,
    pub order_id: u64,
    pub order_list_id: Option<i64>, // Made optional to handle cases where it's not present (e.g., non-OCO orders)
    pub client_order_id: String,
    pub price: String,
    pub orig_qty: String,
    #[serde(rename = "executedQty")]
    pub executed_qty: String,
    #[serde(rename = "cumQty")] // Cumulative filled quantity
    pub cum_qty: String, // Added this field
    #[serde(rename = "cumQuote")] // Cumulative filled quote quantity
    pub cum_quote: String,
    pub status: String, // e.g., "NEW", "FILLED", "PARTIALLY_FILLED"
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub side: String,
    pub stop_price: String,
    pub reduce_only: bool,
    pub position_side: String,
    pub close_position: bool,
    pub update_time: u64, // Changed from 'time' to 'update_time' to match actual response
    pub avg_price: String,
    pub orig_type: String,
    pub working_type: String,
    pub price_protect: bool,
    pub price_match: String,
    pub self_trade_prevention_mode: String,
    pub good_till_date: u64,

    // Fields that are optional/conditionally present, especially for TRAILING_STOP_MARKET
    pub activate_price: Option<String>,
    pub price_rate: Option<String>,
}
/// Represents the response received after canceling an order.
/// Maps to the response from `order.cancel` WebSocket API call or `/fapi/v1/order` REST API call.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderResponse {
    pub symbol: String,
    pub orig_client_order_id: Option<String>, // Made optional
    pub order_id: u64,
    pub order_list_id: Option<i64>, // Made optional since it's missing in the response
    pub client_order_id: String,
    #[serde(rename = "cumQty")] // Cumulative filled quantity
    pub cum_qty: String,
    #[serde(rename = "cumQuote")] // Cumulative filled quote quantity
    pub cum_quote: String,
    pub executed_qty: String,
    pub orig_qty: String,
    pub orig_type: String,
    pub price: String,
    pub reduce_only: bool,
    pub side: String,
    pub position_side: String,
    pub status: String,
    pub stop_price: String,
    pub close_position: bool,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub activate_price: Option<String>, // Optional for TRAILING_STOP_MARKET
    pub price_rate: Option<String>, // Optional for TRAILING_STOP_MARKET
    pub update_time: u64,
    pub working_type: String,
    pub price_protect: bool,
    pub price_match: String,
    pub self_trade_prevention_mode: String,
    pub good_till_date: u64,
}


/// Represents an existing order's details when queried.
/// Maps to the response from `/fapi/v1/order` (REST) or `/fapi/v1/allOrders`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub symbol: String,
    pub order_id: u64,
    pub order_list_id: Option<i64>, // Made optional to handle cases where it's not present (e.g., allOrders)
    pub client_order_id: String,
    pub price: String,
    pub orig_qty: String,
    pub executed_qty: String,
    #[serde(rename = "cumQuote")] // Corrected field name based on schema
    pub cum_quote: String,
    pub status: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub side: String,
    pub stop_price: String,
    pub time: u64, // Reverted to `time` as per schema
    pub update_time: u64,
    pub avg_price: String, // New field from schema
    pub close_position: bool, // New field from schema
    pub good_till_date: u64, // New field from schema
    pub orig_type: String, // New field from schema
    pub position_side: String, // New field from schema
    pub price_match: String, // New field from schema
    pub price_protect: bool, // New field from schema
    pub reduce_only: bool, // New field from schema
    pub self_trade_prevention_mode: String, // New field from schema
    pub working_type: String, // New field from schema

    // Fields that are optional/conditionally present in the /fapi/v1/allOrders response
    pub iceberg_qty: Option<String>, // Made optional
    pub is_working: Option<bool>, // Made optional
    pub orig_quote_order_qty: Option<String>, // Made optional
    pub activate_price: Option<String>, // New field from schema, optional
    pub price_rate: Option<String>, // New field from schema, optional
}

// Note: NewOrderResponse and CancelOrderResponse structs,
// and their associated new_order and cancel_order functions,
// are removed from this file as they are intended for WebSocket API.

impl RestClient { // Order querying and historical data via REST API
    /// Queries the status of a specific order on Binance Futures using REST API.
    ///
    /// This method calls the `/fapi/v1/order` endpoint using a signed GET request.
    ///
    /// # Arguments
    /// * `symbol` - The trading pair symbol.
    /// * `order_id` - Optional. The order ID to query.
    /// * `orig_client_order_id` - Optional. The client order ID to query.
    ///
    /// # Returns
    /// A `Result` containing `Order` details on success, or a `String` error
    /// if the request fails or JSON deserialization fails.
    pub async fn query_order(
        &self,
        symbol: &str,
        order_id: Option<u64>,
        orig_client_order_id: Option<&str>,
    ) -> Result<Order, String> {
        let endpoint = "/fapi/v1/order"; // Correct endpoint for Futures orders
        let symbol_uppercase = symbol.to_uppercase(); // Store the owned String
        let mut params = vec![
            ("symbol", symbol_uppercase.as_str()), // Use as_str() on the owned String
            ("recvWindow", "5000"),
        ];

        let order_id_str = order_id.map(|id| id.to_string()); // Store the owned String
        if let Some(ref id_str) = order_id_str { // Use ref to borrow the String
            params.push(("orderId", id_str.as_str())); // Use as_str() on the owned String
        } else if let Some(client_id) = orig_client_order_id {
            params.push(("origClientOrderId", client_id));
        } else {
            return Err("Missing required order ID or client order ID for query.".to_string());
        }

        let response_value: Value = self.get_signed_rest_request(endpoint, params).await?; // Use GET for querying

        serde_json::from_value(response_value)
            .map_err(|e| format!("Failed to parse order query response JSON: {}", e))
    }

    /// Retrieves all open orders for a given symbol on Binance Futures using REST API,
    /// or all symbols if none is provided.
    ///
    /// This method calls the `/fapi/v1/openOrders` endpoint using a signed GET request.
    ///
    /// # Arguments
    /// * `symbol` - Optional. The trading pair symbol to filter open orders.
    ///
    /// # Returns
    /// A `Result` containing a `Vec<Order>` on success, or a `String` error
    /// if the request fails or JSON deserialization fails.
    pub async fn get_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>, String> {
        let endpoint = "/fapi/v1/openOrders"; // Correct endpoint for Futures open orders
        let mut params = vec![("recvWindow", "5000")];

        let symbol_uppercase_opt = symbol.map(|s| s.to_uppercase()); // Store the owned String
        if let Some(ref s_uppercase) = symbol_uppercase_opt { // Use ref to borrow the String
            params.push(("symbol", s_uppercase.as_str())); // Use as_str() on the owned String
        }

        let response_value: Value = self.get_signed_rest_request(endpoint, params).await?;

        serde_json::from_value(response_value)
            .map_err(|e| format!("Failed to parse open orders JSON: {}", e))
    }

    /// Retrieves all historical orders for a given symbol on Binance Futures using REST API.
    ///
    /// This method calls the `/fapi/v1/allOrders` endpoint using a signed GET request.
    ///
    /// # Arguments
    /// * `symbol` - The trading pair symbol.
    /// * `order_id` - Optional. If provided, returns orders >= this orderId.
    /// * `limit` - Optional. Default 500; max 1000.
    ///
    /// # Returns
    /// A `Result` containing a `Vec<Order>` on success, or a `String` error
    /// if the request fails or JSON deserialization fails.
    pub async fn get_all_orders(
        &self,
        symbol: &str,
        order_id: Option<u64>,
        limit: Option<u16>,
    ) -> Result<Vec<Order>, String> {
        let endpoint = "/fapi/v1/allOrders"; // Correct endpoint for Futures all orders
        let symbol_uppercase = symbol.to_uppercase(); // Store the owned String
        let mut params = vec![
            ("symbol", symbol_uppercase.as_str()), // Use as_str() on the owned String
            ("recvWindow", "5000"),
        ];

        let order_id_str = order_id.map(|id| id.to_string()); // Store the owned String
        if let Some(ref id_str) = order_id_str { // Use ref to borrow the String
            params.push(("orderId", id_str.as_str())); // Use as_str() on the owned String
        }
        let limit_str = limit.map(|l| l.to_string()); // Store the owned String
        if let Some(ref l_str) = limit_str { // Use ref to borrow the String
            params.push(("limit", l_str.as_str())); // Use as_str() on the owned String
        }

        let response_value: Value = self.get_signed_rest_request(endpoint, params).await?;
        // print!("{}",response_value.to_string());

        serde_json::from_value(response_value)
            .map_err(|e| format!("Failed to parse all orders JSON: {}", e))
    }

    // Add other REST-based order functions here, such as:
    // - Querying historical trades
    // - Querying account trade list
}


impl WebSocketClient { // Order placement and cancellation via WebSocket API
    /// Places a new order on Binance Futures using WebSocket API.
    ///
    /// This method calls the `order.place` WebSocket API method.
    ///
    /// # Arguments
    /// * `symbol` - The trading pair symbol (e.g., "BTCUSDT").
    /// * `side` - The order side (`OrderSide::Buy` or `OrderSide::Sell`).
    /// * `order_type` - The type of order (`OrderType::Limit`, `OrderType::Market`, etc.).
    /// * `quantity` - The amount of the base asset to buy/sell.
    /// * `price` - Optional. The price for `LIMIT` orders.
    /// * `time_in_force` - Optional. The time in force for `LIMIT` orders.
    /// * `new_client_order_id` - Optional. A unique ID for the order.
    ///
    /// # Returns
    /// A `Result` containing `NewOrderResponse` on success, or a `String` error
    /// if the request fails or JSON deserialization fails.
    pub async fn new_order( // Renamed to new_order_ws to distinguish from REST version
        &self,
        symbol: &str,
        side: OrderSide,
        order_type: OrderType,
        quantity: f64,
        price: Option<f64>,
        time_in_force: Option<TimeInForce>,
        new_client_order_id: Option<&str>,
    ) -> Result<NewOrderResponse, String> {
        let method = "order.place";
        let mut params = json!({
            "symbol": symbol.to_uppercase(),
            "side": serde_json::to_string(&side).unwrap().trim_matches('"'),
            "type": serde_json::to_string(&order_type).unwrap().trim_matches('"'),
            "quantity": quantity.to_string(), // Quantity as string
        });

        if let Some(p) = price {
            params["price"] = json!(p.to_string()); // Price as string
        }
        if let Some(tif) = time_in_force {
            params["timeInForce"] = json!(serde_json::to_string(&tif).unwrap().trim_matches('"'));
        }
        if let Some(id) = new_client_order_id {
            params["newClientOrderId"] = json!(id);
        }

        let response_value: Value = self.request_websocket_api(method, params).await?;

        // print!("{}",response_value.to_string());

        serde_json::from_value(response_value)
            .map_err(|e| format!("Failed to parse new order response JSON: {}", e))
    }

    /// Cancels an active order on Binance Futures using WebSocket API.
    ///
    /// This method calls the `order.cancel` WebSocket API method.
    ///
    /// # Arguments
    /// * `symbol` - The trading pair symbol.
    /// * `order_id` - Optional. The order ID to cancel.
    /// * `orig_client_order_id` - Optional. The client order ID to cancel.
    ///
    /// # Returns
    /// A `Result` containing `CancelOrderResponse` on success, or a `String` error
    /// if the request fails or JSON deserialization fails.
    pub async fn cancel_order( // Renamed to cancel_order_ws
        &self,
        symbol: &str,
        order_id: Option<u64>,
        orig_client_order_id: Option<&str>,
    ) -> Result<CancelOrderResponse, String> {
        let method = "order.cancel";
        let mut params = json!({
            "symbol": symbol.to_uppercase(),
        });

        if let Some(id) = order_id {
            params["orderId"] = json!(id);
        } else if let Some(client_id) = orig_client_order_id {
            params["origClientOrderId"] = json!(client_id);
        } else {
            return Err("Missing required order ID or client order ID for cancellation.".to_string());
        }

        let response_value: Value = self.request_websocket_api(method, params).await?;

        print!("{}", response_value.to_string());

        serde_json::from_value(response_value)
            .map_err(|e| format!("Failed to parse cancel order response JSON: {}", e))
    }

    // You can add other WebSocket-based order methods here, e.g., order.cancelAll
    // For now, modify order would typically be a cancel-and-replace using new_order/cancel_order combination.
}
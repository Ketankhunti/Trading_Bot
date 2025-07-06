// src/order/mod.rs

//! This module provides functionalities for querying orders on the Binance Futures API
//! using REST endpoints. These operations typically require authenticated (signed) requests.
//! Active order management (placement, cancellation) would be handled by a separate WebSocket client.

use serde::{Deserialize, Serialize};
use crate::rest_api::*; // Import the RestClient for queries
use serde_json::{json, Value};  // Import Value for deserialization from generic JSON
 // Import std::io for io::Error and io::ErrorKind (for custom error messages)
use crate::websocket::WebSocketClient; // Import the WebSocketClient for order placement and cancellation

/// Enum representing the type of order.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Enum representing the time in force for an order.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
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

/// Represents the response received after modifying an order.
/// Maps to the response from `order.modify` WebSocket API call.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifyOrderResponse {
    pub symbol: String,
    pub order_id: u64,
    pub order_list_id: Option<i64>,
    pub client_order_id: String, // This is the NEW client order ID
    pub orig_client_order_id: Option<String>, // This is the ORIGINAL client order ID (optional)
    pub price: String,
    pub orig_qty: String,
    #[serde(rename = "executedQty")]
    pub executed_qty: String,
    #[serde(rename = "cumQty")]
    pub cum_qty: String,
    #[serde(rename = "cumQuote")]
    pub cum_quote: String,
    pub status: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub side: String,
    pub stop_price: String,
    pub reduce_only: bool,
    pub position_side: String,
    pub close_position: bool,
    pub update_time: u64,
    pub avg_price: String,
    pub orig_type: String,
    pub working_type: String,
    pub price_protect: bool,
    pub price_match: String,
    pub self_trade_prevention_mode: String,
    pub good_till_date: u64,
    pub activate_price: Option<String>,
    pub price_rate: Option<String>,
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

        // --- 1. Balance Check ---
        let quote_asset = if symbol.ends_with("USDT") {
            "USDT"
        } else if symbol.ends_with("BUSD") {
            "BUSD"
        } else {
            // Add other quote assets as needed or handle unknown
            return Err(format!("Unsupported quote asset for symbol: {}", symbol));
        };

        // Call the new helper function in account_info to get available balance
        let available_balance_quote = match self.get_asset_balance(quote_asset).await? {
            Some(asset_balance) => asset_balance.available_balance.parse::<f64>()
                .map_err(|e| format!("Failed to parse available balance: {}", e))?,
            None => return Err(format!("Asset {} not found in account balance", quote_asset)),
        };

        let order_price = if let Some(price)  = price {
            price
        }else{
            // For market orders, we need to fetch the current price
            match self.get_current_price(symbol).await {
                Ok(ticker_price) => ticker_price.price.parse::<f64>()
                    .map_err(|e| format!("Failed to parse current price: {}", e))?,
                Err(e) => return Err(format!("Failed to get current price for {}: {}", symbol, e)),
            }
        };


        let estimated_cost = quantity * order_price;
        // Assuming a fixed commission rate for simplicity. In a real bot, fetch from exchange info.
        const COMMISSION_RATE: f64 = 0.0004; // 0.04%
        let total_cost_with_commission = estimated_cost * (1.0 + COMMISSION_RATE);

        // Debug prints for balance check
        println!("[DEBUG] Symbol: {} | Side: {:?} | Order Type: {:?}", symbol, side, order_type);
        println!("[DEBUG] Available balance for {}: {:.8}", quote_asset, available_balance_quote);
        println!("[DEBUG] Order quantity: {:.8} | Order price: {:.8}", quantity, order_price);
        println!("[DEBUG] Estimated cost: {:.8} | Total with commission: {:.8}", estimated_cost, total_cost_with_commission);

        if available_balance_quote < total_cost_with_commission {
            println!("[DEBUG] Insufficient funds: required {:.8}, available {:.8}", total_cost_with_commission, available_balance_quote);
            return Err(format!(
                "Insufficient funds for order. Required: {:.4} {} (including commission). Available: {:.4} {}",
                total_cost_with_commission, quote_asset, available_balance_quote, quote_asset
            ));
        }

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

        serde_json::from_value(response_value)
            .map_err(|e| format!("Failed to parse cancel order response JSON: {}", e))
    }

    pub async fn modify_order(
        &self,
        symbol: &str,
        side: OrderSide,
        order_id: Option<u64>,
        orig_client_order_id: Option<&str>,
        quantity: Option<f64>,
        price: Option<f64>,
        stop_price: Option<f64>,
        activation_price: Option<f64>,
        callback_rate: Option<f64>,
        new_client_order_id: Option<&str>,
    ) -> Result<ModifyOrderResponse, String> {
        // Balance check for buy orders (only if price and quantity are being modified)
        if side == OrderSide::Buy && (price.is_some() || quantity.is_some()) {
            let quote_asset = if symbol.ends_with("USDT") {
                "USDT"
            } else if symbol.ends_with("BUSD") {
                "BUSD"
            } else {
                // Add other quote assets as needed or handle unknown
                return Err(format!("Unsupported quote asset for symbol: {}", symbol));
            };

            // Get available balance for the quote asset
            let available_balance_quote = match self.get_asset_balance(quote_asset).await? {
                Some(asset_balance) => asset_balance.available_balance.parse::<f64>()
                    .map_err(|e| format!("Failed to parse available balance: {}", e))?,
                None => return Err(format!("Asset {} not found in account balance", quote_asset)),
            };

            // Calculate estimated cost based on modified parameters
            let order_price = price.unwrap_or(0.0); // Use modified price if available
            let order_quantity = quantity.unwrap_or(0.0); // Use modified quantity if available
            
            if order_price > 0.0 && order_quantity > 0.0 {
                let estimated_cost = order_quantity * order_price;
                // Assuming a fixed commission rate for simplicity. In a real bot, fetch from exchange info.
                const COMMISSION_RATE: f64 = 0.0004; // 0.04%
                let total_cost_with_commission = estimated_cost * (1.0 + COMMISSION_RATE);

                if available_balance_quote < total_cost_with_commission {
                    return Err(format!(
                        "Insufficient funds for order modification. Required: {:.4} {} (including commission). Available: {:.4} {}",
                        total_cost_with_commission, quote_asset, available_balance_quote, quote_asset
                    ));
                }
            }
        }

        let method = "order.modify";
        let mut params = json!({
            "symbol": symbol.to_uppercase(),
            "side": serde_json::to_string(&side).unwrap().trim_matches('"'),
        });

        // Identify the order to amend
        if let Some(id) = order_id {
            params["orderId"] = json!(id);
        } else if let Some(client_id) = orig_client_order_id {
            params["origClientOrderId"] = json!(client_id);
        } else {
            return Err("Missing required order ID or original client order ID for modification.".to_string());
        }

        // Add optional modification parameters
        if let Some(qty) = quantity {
            params["quantity"] = json!(qty.to_string());
        }
        if let Some(p) = price {
            params["price"] = json!(p.to_string());
        }
        if let Some(sp) = stop_price {
            params["stopPrice"] = json!(sp.to_string());
        }
        if let Some(ap) = activation_price {
            params["activationPrice"] = json!(ap.to_string());
        }
        if let Some(cr) = callback_rate {
            params["callbackRate"] = json!(cr.to_string());
        }
        if let Some(new_id) = new_client_order_id {
            params["newClientOrderId"] = json!(new_id);
        }

        // Ensure at least one modification parameter is provided
        if quantity.is_none() && price.is_none() && stop_price.is_none() && activation_price.is_none() && callback_rate.is_none() {
            return Err("At least one of quantity, price, stopPrice, activationPrice, or callbackRate must be provided for modification.".to_string());
        }

        let response_value: Value = self.request_websocket_api(method, params).await?;

        serde_json::from_value(response_value)
            .map_err(|e| format!("Failed to parse modify order response JSON: {}", e))
    }

}
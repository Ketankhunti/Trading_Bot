// src/websocket/user_data.rs

//! This module defines the data structures for user data streams from Binance.
//! These streams provide real-time updates on account balances, orders, and other
//! user-specific events.

use serde::{Deserialize, Serialize};

/// Represents a generic user data stream message.
/// The actual data will be parsed into specific structs based on the event type (`e`).
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)] // Allows deserialization into different types based on content
pub enum UserDataStream {
    /// Account Update event (`e: "outboundAccountPosition"`)
    #[serde(rename_all = "camelCase")]
    AccountUpdate(AccountUpdateEvent),
    /// Order Update event (`e: "executionReport"`)
    #[serde(rename_all = "camelCase")]
    OrderUpdate(OrderUpdateEvent),
    /// Balance Update event (`e: "balanceUpdate"`)
    #[serde(rename_all = "camelCase")]
    BalanceUpdate(BalanceUpdateEvent),
    // Add other user data stream types as needed, e.g., for OCO orders.
}

/// Represents an Account Update event (`outboundAccountPosition`).
/// This event is pushed every time the account balance changes.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountUpdateEvent {
    #[serde(rename = "e")]
    pub event_type: String, // outboundAccountPosition
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "u")]
    pub last_account_update_time: u64,
    #[serde(rename = "B")]
    pub balances: Vec<AccountBalance>, // Array of updated balances
}

/// Represents a single asset balance within an `AccountUpdateEvent`.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountBalance {
    #[serde(rename = "a")]
    pub asset: String,
    #[serde(rename = "f")]
    pub free: String,
    #[serde(rename = "l")]
    pub locked: String,
}

/// Represents an Order Update event (`executionReport`).
/// This event is pushed every time an order status changes.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderUpdateEvent {
    #[serde(rename = "e")]
    pub event_type: String, // executionReport
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "c")]
    pub client_order_id: String,
    #[serde(rename = "S")]
    pub side: String, // BUY or SELL
    #[serde(rename = "o")]
    pub order_type: String, // LIMIT, MARKET, etc.
    #[serde(rename = "f")]
    pub time_in_force: String, // GTC, IOC, FOK
    #[serde(rename = "q")]
    pub original_quantity: String,
    #[serde(rename = "p")]
    pub original_price: String,
    #[serde(rename = "P")]
    pub stop_price: String,
    #[serde(rename = "F")]
    pub iceberg_quantity: String,
    #[serde(rename = "g")]
    pub order_list_id: i64, // -1 for non-OCO, otherwise ID
    #[serde(rename = "C")]
    pub original_client_order_id: String, // Used for cancel/replace orders
    #[serde(rename = "x")]
    pub current_execution_type: String, // NEW, CANCELED, TRADE, EXPIRED, REJECTED
    #[serde(rename = "X")]
    pub current_order_status: String, // NEW, PARTIALLY_FILLED, FILLED, CANCELED, PENDING_CANCEL, REJECTED, EXPIRED
    #[serde(rename = "r")]
    pub order_reject_reason: String, // For REJECTED orders
    #[serde(rename = "i")]
    pub order_id: u64,
    #[serde(rename = "l")]
    pub last_executed_quantity: String,
    #[serde(rename = "z")]
    pub cumulative_filled_quantity: String,
    #[serde(rename = "L")]
    pub last_executed_price: String,
    #[serde(rename = "n")]
    pub commission_amount: String,
    #[serde(rename = "N")]
    pub commission_asset: String,
    #[serde(rename = "T")]
    pub trade_time: u64,
    #[serde(rename = "t")]
    pub trade_id: u64,
    #[serde(rename = "I")]
    pub ignore_a: u64, // Ignored
    #[serde(rename = "w")]
    pub is_order_on_book: bool,
    #[serde(rename = "m")]
    pub is_maker_side: bool,
    #[serde(rename = "M")]
    pub ignore_b: bool, // Ignored
    #[serde(rename = "O")]
    pub order_creation_time: u64,
    #[serde(rename = "Z")]
    pub cumulative_quote_asset_transacted_quantity: String,
    #[serde(rename = "Q")]
    pub original_quote_order_quantity: String,
    #[serde(rename = "N")]
    pub quote_asset_commission: Option<String>, // Optional for some events
    #[serde(rename = "u")]
    pub last_update_time: u64,
}

/// Represents a Balance Update event (`balanceUpdate`).
/// This event is pushed when a balance is updated (e.g., due to deposit/withdrawal).
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BalanceUpdateEvent {
    #[serde(rename = "e")]
    pub event_type: String, // balanceUpdate
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "a")]
    pub asset: String,
    #[serde(rename = "d")]
    pub balance_delta: String, // The amount of the change
    #[serde(rename = "T")]
    pub clear_time: u64, // The time of the balance clear
}

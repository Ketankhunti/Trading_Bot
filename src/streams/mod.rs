// src/websocket/agg_trade.rs

//! This module defines the data structure for the aggregated trade stream (`<symbol>@aggTrade`).


use serde::{Deserialize, Serialize};

/// Represents an aggregated trade stream message (`<symbol>@aggTrade`).
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AggTradeStream {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "a")]
    pub agg_trade_id: u64,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "f")]
    pub first_trade_id: u64,
    #[serde(rename = "l")]
    pub last_trade_id: u64,
    #[serde(rename = "T")]
    pub trade_time: u64,
    #[serde(rename = "m")]
        pub maker: bool,
    #[serde(rename = "M")]
    pub ignore: bool,
}





/// Represents an update to the order book depth stream (`<symbol>@depth` or `<symbol>@depth<levels>`).
/// This can be used for both diff depth and partial depth streams, depending on how it's populated.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DepthStream {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "U")]
    pub first_update_id: u64, // First update ID in event
    #[serde(rename = "u")]
    pub final_update_id: u64, // Final update ID in event
    #[serde(rename = "b")]
    pub bids: Vec<DepthLevel>, // Bids to be updated/inserted
    #[serde(rename = "a")]
    pub asks: Vec<DepthLevel>, // Asks to be updated/inserted
}

/// Represents a single price level in the order book (bid or ask).
/// The inner vector contains [price, quantity].
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)] // Deserialize from an array of values
pub enum DepthLevel {
    Array(String, String), // [price, quantity]
}

// You can add more specific depth types if needed, e.g.,
// for combined streams or specific partial depth snapshots.
// src/websocket/ticker.rs



/// Represents a 24-hour rolling window ticker statistics stream message (`<symbol>@ticker`).
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TickerStream {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price_change: String,
    #[serde(rename = "P")]
    pub price_change_percent: String,
    #[serde(rename = "w")]
    pub weighted_avg_price: String,
    #[serde(rename = "x")]
    pub first_trade_price: String,
    #[serde(rename = "c")]
    pub last_price: String,
    #[serde(rename = "Q")]
    pub last_quantity: String,
    #[serde(rename = "b")]
    pub best_bid_price: String,
    #[serde(rename = "B")]
    pub best_bid_quantity: String,
    #[serde(rename = "a")]
    pub best_ask_price: String,
    #[serde(rename = "A")]
    pub best_ask_quantity: String,
    #[serde(rename = "o")]
    pub open_price: String,
    #[serde(rename = "h")]
    pub high_price: String,
    #[serde(rename = "l")]
    pub low_price: String,
    #[serde(rename = "v")]
    pub total_traded_base_asset_volume: String,
    #[serde(rename = "q")]
    pub total_traded_quote_asset_volume: String,
    #[serde(rename = "O")]
    pub statistics_open_time: u64,
    #[serde(rename = "C")]
    pub statistics_close_time: u64,
    #[serde(rename = "F")]
    pub first_trade_id: u64,
    #[serde(rename = "L")]
    pub last_trade_id: u64,
    #[serde(rename = "n")]
    pub total_number_of_trades: u64,
}

// You can add more specific ticker types if needed, e.g.,
// for individual symbol mini-tickers or all market tickers.

// src/websocket/user_data.rs



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
// src/websocket/kline.rs


/// Represents a kline (candlestick) stream message (`<symbol>@kline_<interval>`).
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KlineStream {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "k")]
    pub kline: KlineData,
}

/// Represents the actual kline data within a `KlineStream` message.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KlineData {
    #[serde(rename = "t")]
    pub open_time: u64,
    #[serde(rename = "T")]
    pub close_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "i")]
    pub interval: String,
    #[serde(rename = "f")]
    pub first_trade_id: u64,
    #[serde(rename = "L")]
    pub last_trade_id: u64,
    #[serde(rename = "o")]
    pub open: String,
    #[serde(rename = "c")]
    pub close: String,
    #[serde(rename = "h")]
    pub high: String,
    #[serde(rename = "l")]
    pub low: String,
    #[serde(rename = "v")]
    pub volume: String,
    #[serde(rename = "n")]
    pub number_of_trades: u64,
    #[serde(rename = "x")]
    pub is_closed: bool,
    #[serde(rename = "q")]
    pub quote_asset_volume: String,
    #[serde(rename = "V")]
    pub taker_buy_base_asset_volume: String,
    #[serde(rename = "Q")]
    pub taker_buy_quote_asset_volume: String,
    #[serde(rename = "B")]
    pub ignore: String, // This field is often ignored/unused in Binance kline data
}

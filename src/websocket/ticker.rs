// src/websocket/ticker.rs

//! This module defines the data structures for various ticker streams from Binance.
//! This includes 24-hour rolling window statistics.

use serde::{Deserialize, Serialize};

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

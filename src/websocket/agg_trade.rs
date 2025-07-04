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

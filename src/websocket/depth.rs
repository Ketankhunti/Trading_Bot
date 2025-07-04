// src/websocket/depth.rs

//! This module defines the data structures for order book depth streams from Binance.
//! This includes partial book depth and diff depth streams.

use serde::{Deserialize, Serialize};

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

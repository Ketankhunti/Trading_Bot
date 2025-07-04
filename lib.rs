// src/lib.rs

//! A modular Rust client for interacting with the Binance API.
//!
//! This library aims to provide a clean and organized way to access
//! Binance's various API endpoints, separated into logical modules
//! for account information, market data, and order management.

// Re-export the core client module
pub mod rest_client;

// Re-export other modules as they are created
pub mod account_info;
pub mod market_data;
pub mod order;

// Re-export stream-specific data structures from their new files
pub mod agg_trade;
pub mod kline;
pub mod ticker;
pub mod depth;
pub mod user_data;

pub mod websocket;
pub mod websocket_stream;
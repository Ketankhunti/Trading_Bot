mod rest_api;
mod websocket;
mod tui;
mod account_info;
mod order;
mod websocket_stream;
mod strategy;
mod streams;
mod market_data;

use rest_api::*;
use websocket::*;
use tui::*;
use account_info::*;
use order::*;
use websocket_stream::*;
use strategy::*;
use streams::*;
use market_data::*;
use csv::Reader;
use chrono::{DateTime, Utc, NaiveDateTime};

use log::{info, debug, error, warn}; // For logging within the backtest
use std::env; // Import to read environment variables
use dotenv::dotenv; // Import dotenv to load .env file
use serde_json::from_value; // Import for JSON deserialization


// #[tokio::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
       strategy::run();

       Ok(())
}

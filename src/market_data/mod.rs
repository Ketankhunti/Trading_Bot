// src/market_data/mod.rs

//! This module provides functionalities for retrieving various types of market data
//! from the Binance API using REST endpoints, including current prices,
//! 24-hour ticker statistics, and historical candlestick data.

use serde::Deserialize;
use crate::rest_api::RestClient; // Import the core RestClient
use serde_json::Value; // Import Value for deserialization from generic JSON

/// Represents a single ticker price for a symbol.
/// Maps to the response from `/fapi/v1/ticker/price`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TickerPrice {
    pub symbol: String,
    pub price: String, // Use String for decimal numbers
}

/// Represents a 24-hour ticker statistics for a symbol.
/// Maps to the response from `/fapi/v1/ticker/24hr`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ticker24hr {
    #[serde(rename = "symbol")]
    pub symbol: String,
    #[serde(rename = "priceChange")]
    pub price_change: String,
    #[serde(rename = "priceChangePercent")]
    pub price_change_percent: String,
    #[serde(rename = "weightedAvgPrice")]
    pub weighted_avg_price: String,
    #[serde(rename = "lastPrice")]
    pub last_price: String,
    #[serde(rename = "lastQty")]
    pub last_qty: String,
    #[serde(rename = "openPrice")]
    pub open_price: String,
    #[serde(rename = "highPrice")]
    pub high_price: String,
    #[serde(rename = "lowPrice")]
    pub low_price: String,
    #[serde(rename = "volume")]
    pub volume: String,
    #[serde(rename = "quoteVolume")]
    pub quote_volume: String,
    #[serde(rename = "openTime")]
    pub open_time: u64,
    #[serde(rename = "closeTime")]
    pub close_time: u64,
    #[serde(rename = "firstId")]
    pub first_id: i64, // First tradeId
    #[serde(rename = "lastId")]
    pub last_id: i64, // Last tradeId
    #[serde(rename = "count")]
    pub count: u64, // Number of trades
    // Note: The Binance Futures 24hr ticker response has slightly different fields
    // compared to Spot. These fields are based on Futures API.
}


/// Represents a single candlestick (K-line) data point.
/// Maps to the array elements returned by `/fapi/v1/klines`.
#[derive(Debug, Deserialize)]
#[serde(untagged)] // Use untagged to deserialize from an array of values
pub enum Candlestick {
    Array(
        u64,    // Open time
        String, // Open
        String, // High
        String, // Low
        String, // Close
        String, // Volume
        u64,    // Close time
        String, // Quote asset volume
        u64,    // Number of trades
        String, // Taker buy base asset volume
        String, // Taker buy quote asset volume
        String, // Ignore
    ),
}

/// Enum for Candlestick intervals.
#[derive(Debug, Clone, Copy)]
pub enum KlineInterval {
    #[allow(dead_code)] M1,
    #[allow(dead_code)] M3,
    #[allow(dead_code)] M5,
    #[allow(dead_code)] M15,
    #[allow(dead_code)] M30,
    #[allow(dead_code)] H1,
    #[allow(dead_code)] H2,
    #[allow(dead_code)] H4,
    #[allow(dead_code)] H6,
    #[allow(dead_code)] H8,
    #[allow(dead_code)] H12,
    #[allow(dead_code)] D1,
    #[allow(dead_code)] D3,
    #[allow(dead_code)] W1,
    #[allow(dead_code)] MN1,
}

impl ToString for KlineInterval {
    fn to_string(&self) -> String {
        match self {
            KlineInterval::M1 => "1m".to_string(),
            KlineInterval::M3 => "3m".to_string(),
            KlineInterval::M5 => "5m".to_string(),
            KlineInterval::M15 => "15m".to_string(),
            KlineInterval::M30 => "30m".to_string(),
            KlineInterval::H1 => "1h".to_string(),
            KlineInterval::H2 => "2h".to_string(),
            KlineInterval::H4 => "4h".to_string(),
            KlineInterval::H6 => "6h".to_string(),
            KlineInterval::H8 => "8h".to_string(),
            KlineInterval::H12 => "12h".to_string(),
            KlineInterval::D1 => "1d".to_string(),
            KlineInterval::D3 => "3d".to_string(),
            KlineInterval::W1 => "1w".to_string(),
            KlineInterval::MN1 => "1M".to_string(),
        }
    }
}


impl RestClient {
    /// Fetches the current average price for a given symbol using REST API.
    ///
    /// This method calls the `/fapi/v1/avgPrice` endpoint.
    ///
    /// # Arguments
    /// * `symbol` - The trading pair symbol (e.g., "BTCUSDT").
    ///
    /// # Returns
    /// A `Result` containing `TickerPrice` on success, or a `String` error
    /// if the request fails or JSON deserialization fails.
    pub async fn get_current_price(&self, symbol: &str) -> Result<TickerPrice, String> {
        let endpoint = "/fapi/v1/avgPrice";
        let symbol_uppercase = symbol.to_uppercase();
        let params = vec![("symbol", symbol_uppercase.as_str())];
        let response_value: Value = self.get_unsigned_rest_request(endpoint, params).await?;

        serde_json::from_value(response_value)
            .map_err(|e| format!("Failed to parse current price JSON: {}", e))
    }

    /// Fetches the 24-hour ticker statistics for a given symbol using REST API.
    ///
    /// This method calls the `/fapi/v1/ticker/24hr` endpoint.
    ///
    /// # Arguments
    /// * `symbol` - The trading pair symbol (e.g., "BTCUSDT").
    ///
    /// # Returns
    /// A `Result` containing `Ticker24hr` on success, or a `String` error
    /// if the request fails or JSON deserialization fails.
    pub async fn get_24hr_ticker_stats(&self, symbol: &str) -> Result<Ticker24hr, String> {
        let endpoint = "/fapi/v1/ticker/24hr";
        let symbol_uppercase = symbol.to_uppercase();
        let params = vec![("symbol", symbol_uppercase.as_str())];
        let response_value: Value = self.get_unsigned_rest_request(endpoint, params).await?;

        serde_json::from_value(response_value)
            .map_err(|e| format!("Failed to parse 24hr ticker stats JSON: {}", e))
    }

    /// Fetches candlestick (K-line) data for a given symbol and interval using REST API.
    ///
    /// This method calls the `/fapi/v1/klines` endpoint.
    ///
    /// # Arguments
    /// * `symbol` - The trading pair symbol (e.g., "BTCUSDT").
    /// * `interval` - The candlestick interval (e.g., `KlineInterval::M1`, `KlineInterval::H4`).
    /// * `limit` - Optional. The number of candlesticks to retrieve (default 500, max 1000).
    /// * `start_time` - Optional. Start time in milliseconds.
    /// * `end_time` - Optional. End time in milliseconds.
    ///
    /// # Returns
    /// A `Result` containing a `Vec<Candlestick>` on success, or a `String` error
    /// if the request fails or JSON deserialization fails.
    pub async fn get_klines(
        &self,
        symbol: &str,
        interval: KlineInterval,
        limit: Option<u16>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> Result<Vec<Candlestick>, String> {
        let endpoint = "/fapi/v1/klines";
        let symbol_uppercase = symbol.to_uppercase(); // Store the owned String
        let interval_str = interval.to_string(); // Store the owned String

        let mut params = vec![
            ("symbol", symbol_uppercase.as_str()),
            ("interval", interval_str.as_str()),
        ];

        let limit_str = limit.map(|l| l.to_string());
        if let Some(ref l_str) = limit_str {
            params.push(("limit", l_str.as_str()));
        }
        let start_time_str = start_time.map(|st| st.to_string());
        if let Some(ref st_str) = start_time_str {
            params.push(("startTime", st_str.as_str()));
        }
        let end_time_str = end_time.map(|et| et.to_string());
        if let Some(ref et_str) = end_time_str {
            params.push(("endTime", et_str.as_str()));
        }

        let response_value: Value = self.get_unsigned_rest_request(endpoint, params).await?;

        serde_json::from_value(response_value)
            .map_err(|e| format!("Failed to parse klines JSON: {}", e))
    }

    // You can add other market data functions here, such as:
    // - get_order_book(symbol: &str, limit: Option<u16>)
    // - get_recent_trades(symbol: &str, limit: Option<u16>)
    // - get_historical_trades(symbol: &str, limit: Option<u16>, from_id: Option<u64>)
    // - get_exchange_info()
}

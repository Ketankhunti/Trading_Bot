// src/rest_client.rs

//! This module defines the core `RestClient` responsible for
//! handling generic HTTP REST API requests. It provides low-level
//! functionalities for signed and unsigned GET and POST requests,
//! managing connections, authentication (signing), and basic request/response dispatch.

use reqwest::{Client, Response, Error, Url};
use serde_json::Value;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use hex::encode;
use std::time::{SystemTime, UNIX_EPOCH};
use log::{info, error, debug}; // For logging

/// Represents the Binance REST API Client.
/// This client handles REST API calls.
pub struct RestClient {
    api_key: String,
    secret_key: String,
    http_client: Client,
    rest_base_url: String,
}

impl RestClient {
    /// Creates a new RestClient instance.
    ///
    /// # Arguments
    /// * `api_key` - Your Binance API Key.
    /// * `secret_key` - Your Binance Secret Key.
    /// * `rest_base_url` - The base URL for the REST API (e.g., "https://testnet.binancefuture.com").
    ///
    /// # Returns
    /// A new `RestClient` instance.
    pub fn new(
        api_key: String,
        secret_key: String,
        rest_base_url: String,
    ) -> Self {
        Self {
            api_key,
            secret_key,
            http_client: Client::new(),
            rest_base_url,
        }
    }

    /// Generates a Binance API signature using HMAC SHA256.
    ///
    /// # Arguments
    /// * `query_string` - The query string (parameters) to sign.
    fn sign_payload(&self, query_string: &str) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(query_string.as_bytes());
        encode(mac.finalize().into_bytes())
    }

    /// Makes a signed GET request to the Binance REST API.
    /// This method is used for authenticated endpoints requiring a signature.
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint (e.g., "/fapi/v2/account"). This should include the API version.
    /// * `params` - Query parameters as a vector of (key, value) tuples.
    ///
    /// # Returns
    /// A `Result` containing the parsed JSON `Value` on success, or a `String` error.
    pub async fn get_signed_rest_request(&self, endpoint: &str, params: Vec<(&str, &str)>) -> Result<Value, String> {
        let mut url = Url::parse(&format!("{}{}", self.rest_base_url, endpoint))
            .map_err(|e| format!("Failed to parse URL: {}", e))?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Failed to get timestamp: {}", e))?
            .as_millis()
            .to_string();

        let mut query_pairs: Vec<String> = params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        query_pairs.push(format!("timestamp={}", timestamp));

        let query_string = query_pairs.join("&");
        let signature = self.sign_payload(&query_string);

        url.set_query(Some(&format!("{}&signature={}", query_string, signature)));

        debug!("Signed REST GET request URL: {}", url);

        let response = self.http_client.get(url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("Failed to send REST GET request: {}", e))?;

        if response.status().is_success() {
            response.json::<Value>()
                .await
                .map_err(|e| format!("Failed to parse JSON REST response: {}", e))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "No response body".to_string());
            Err(format!("REST API GET request failed with status {}: {}", status, text))
        }
    }

    /// Makes an unsigned GET request to the Binance REST API.
    /// Used for public endpoints like market data that do not require authentication.
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint (e.g., "/fapi/v1/ticker/price").
    /// * `params` - Query parameters as a vector of (key, value) tuples.
    ///
    /// # Returns
    /// A `Result` containing the parsed JSON `Value` on success, or a `String` error.
    pub async fn get_unsigned_rest_request(&self, endpoint: &str, params: Vec<(&str, &str)>) -> Result<Value, String> {
        let mut url = Url::parse(&format!("{}{}", self.rest_base_url, endpoint))
            .map_err(|e| format!("Failed to parse URL: {}", e))?;

        let query_pairs: Vec<String> = params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        if !query_pairs.is_empty() {
            url.set_query(Some(&query_pairs.join("&")));
        }

        debug!("Unsigned REST GET request URL: {}", url);

        let response = self.http_client.get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to send REST GET request: {}", e))?;

        if response.status().is_success() {
            response.json::<Value>()
                .await
                .map_err(|e| format!("Failed to parse JSON REST response: {}", e))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "No response body".to_string());
            Err(format!("REST API GET request failed with status {}: {}", status, text))
        }
    }

    /// Makes a signed POST request to the Binance REST API.
    /// This method is used for authenticated endpoints requiring a signature, typically for actions like placing orders.
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint (e.g., "/fapi/v1/order"). This should include the API version.
    /// * `params` - Form parameters as a vector of (key, value) tuples. These will be sent as query parameters for signing.
    ///
    /// # Returns
    /// A `Result` containing the parsed JSON `Value` on success, or a `String` error.
    pub async fn post_signed_rest_request(&self, endpoint: &str, params: Vec<(&str, &str)>) -> Result<Value, String> {
        let url = format!("{}{}", self.rest_base_url, endpoint);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Failed to get timestamp: {}", e))?
            .as_millis()
            .to_string();

        let mut query_pairs: Vec<String> = params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        query_pairs.push(format!("timestamp={}", timestamp));

        let query_string = query_pairs.join("&");
        let signature = self.sign_payload(&query_string);

        // For POST requests, parameters (including timestamp and signature) are typically sent as query parameters
        let final_url = format!("{}?{}&signature={}", url, query_string, signature);

        debug!("Signed REST POST request URL: {}", final_url);

        let response = self.http_client.post(&final_url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("Failed to send REST POST request: {}", e))?;

        if response.status().is_success() {
            response.json::<Value>()
                .await
                .map_err(|e| format!("Failed to parse JSON REST response: {}", e))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "No response body".to_string());
            Err(format!("REST API POST request failed with status {}: {}", status, text))
        }
    }

    /// Makes an unsigned POST request to the Binance REST API.
    /// Used for public endpoints that accept POST requests without authentication.
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint.
    /// * `params` - Form parameters as a vector of (key, value) tuples. These will be sent as query parameters.
    ///
    /// # Returns
    /// A `Result` containing the parsed JSON `Value` on success, or a `String` error.
    pub async fn post_unsigned_rest_request(&self, endpoint: &str, params: Vec<(&str, &str)>) -> Result<Value, String> {
        let url = format!("{}{}", self.rest_base_url, endpoint);

        let query_string = params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
            .join("&");

        let final_url = if query_string.is_empty() {
            url
        } else {
            format!("{}?{}", url, query_string)
        };

        debug!("Unsigned REST POST request URL: {}", final_url);

        let response = self.http_client.post(&final_url)
            .send()
            .await
            .map_err(|e| format!("Failed to send REST POST request: {}", e))?;

        if response.status().is_success() {
            response.json::<Value>()
                .await
                .map_err(|e| format!("Failed to parse JSON REST response: {}", e))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "No response body".to_string());
            Err(format!("REST API POST request failed with status {}: {}", status, text))
        }
    }
}

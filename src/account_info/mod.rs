// src/account_info/mod.rs

//! This module provides functionalities for retrieving account-specific data
//! from the Binance Futures API.

use serde::{Deserialize, Serialize};
use crate::rest_client::RestClient; // Import the core BinanceClient
use serde_json::Value; // Import Value for deserialization from generic JSON

/// Represents the overall account information for Binance Futures.
/// This struct maps to the response from the `/fapi/v3/account` endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")] // Maps camelCase JSON fields to snake_case Rust fields
pub struct AccountInfo {
    pub total_initial_margin: String,            // total initial margin required with current mark price
    pub total_maint_margin: String,  	           // total maintenance margin required
    pub total_wallet_balance: String,           // total wallet balance
    pub total_unrealized_profit: String,         // total unrealized profit
    pub total_margin_balance: String,           // total margin balance
    pub total_position_initial_margin: String,    // initial margin required for positions with current mark price
    pub total_open_order_initial_margin: String,   // initial margin required for open orders with current mark price
    pub total_cross_wallet_balance: String,      // crossed wallet balance
    pub total_cross_un_pnl: String,	           // unrealized profit of crossed positions
    pub available_balance: String,             // available balance
    pub max_withdraw_amount: String,             // maximum amount for transfer out
    pub assets: Vec<AssetBalance>,               // Array of asset balances
    pub positions: Vec<PositionInfo>,            // Array of position details
    // Removed can_trade, can_withdraw, can_deposit, and top-level update_time
    // as they were not present in the provided JSON response example for /fapi/v3/account.
    // If these appear in other responses or modes, they would need to be added back as Option<T>.
}

/// Represents the balance details of a single asset in the Futures account.
/// This is a sub-structure within the `assets` array of `AccountInfo`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetBalance {
    pub asset: String,                           // asset name
    pub wallet_balance: String,                  // wallet balance
    pub unrealized_profit: String,               // unrealized profit
    pub margin_balance: String,                  // margin balance
    pub maint_margin: String,	                 // maintenance margin required
    pub initial_margin: String,                  // total initial margin required with current mark price
    pub position_initial_margin: String,         // initial margin required for positions with current mark price
    pub open_order_initial_margin: String,       // initial margin required for open orders with current mark price
    pub cross_wallet_balance: String,            // crossed wallet balance
    pub cross_un_pnl: String,                    // unrealized profit of crossed positions
    pub available_balance: String,               // available balance
    pub max_withdraw_amount: String,             // maximum amount for transfer out
    pub update_time: u64,                        // last update time for this asset
    #[serde(default)] // Use default to handle absence in single-asset mode
    pub margin_available: Option<bool>,          // whether the asset can be used as margin in Multi-Assets mode (optional)
}

/// Represents the details of a single position in the Futures account.
/// This is a sub-structure within the `positions` array of `AccountInfo`.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PositionInfo {
    pub symbol: String,                          // trading pair symbol
    pub position_side: String,                   // position side (BOTH, LONG, SHORT)
    pub position_amt: String,                    // position amount
    pub unrealized_profit: String,               // unrealized profit
    pub isolated_margin: String,                 // isolated margin
    pub notional: String,                        // notional value of the position
    pub isolated_wallet: String,                 // isolated wallet balance
    pub initial_margin: String,                  // initial margin required with current mark price
    pub maint_margin: String,                    // maintenance margin required
    pub update_time: u64,                        // last update time
}


impl RestClient {
    /// Fetches the current account information for the authenticated user on Binance Futures.
    ///
    /// This method calls the `/fapi/v3/account` endpoint, which requires
    /// a signed private request.
    ///
    /// # Returns
    /// A `Result` containing `AccountInfo` on success, or a `String` error
    /// if the request fails (e.g., network error, API error, or JSON deserialization error).
    pub async fn get_account_info(&self) -> Result<AccountInfo, String> {
        // Use the correct endpoint for Binance Futures Account Information
        let endpoint = "/fapi/v3/account";
        // No additional parameters are typically needed for this endpoint
        let response_value: Value = self.get_signed_rest_request(endpoint, vec![]).await?;

        // Deserialize the generic JSON Value into the specific AccountInfo struct
        serde_json::from_value(response_value)
            .map_err(|e| format!("Failed to parse account info JSON: {}", e))
    }

    /// Fetches the current available balance for a specific asset in the Futures account.
    ///
    /// This method internally calls `get_account_info` and then filters
    /// the assets to find the requested asset.
    ///
    /// # Arguments
    /// * `asset` - The symbol of the asset (e.g., "BTC", "USDT").
    ///
    /// # Returns
    /// A `Result` containing `Option<AssetBalance>` on success. `None` is returned
    /// if the asset is not found in the account balances.
    /// Returns a `String` error if the underlying `get_account_info` call fails.
    pub async fn get_asset_balance(&self, asset: &str) -> Result<Option<AssetBalance>, String> {
        let account_info = self.get_account_info().await?;
        let balance = account_info.assets.into_iter().find(|b| b.asset == asset.to_uppercase());
        Ok(balance)
    }

    // You can add more account-related functions here, such as:
    // - get_position_information()
    // - get_commission_rate(symbol: &str)
}

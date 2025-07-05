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

// Backtesting specific parameters
const BACKTEST_SYMBOL: &str = "BNBUSDT";
const BACKTEST_KLINES_LIMIT: u16 = 1000; // Number of historical klines to fetch for backtesting
const STARTING_CAPITAL: f64 = 10000.0; // Starting capital for backtest simulation (e.g., in USDT)
const COMMISSION_RATE: f64 = 0.0004; // Example commission rate (0.04% for maker/taker on Futures)
const EXCEL_FILE_PATH: &str = "btc_15m_data_2018_to_2025.csv"; // Path to your CSV file

#[derive(Debug, Clone)]
struct BacktestPortfolio {
    pub cash: f64, // Current cash balance (in quote currency, e.g., USDT)
    pub position: f64, // Quantity of the base asset held (e.g., BNB if trading BNBUSDT)
    pub entry_price: Option<f64>, // Average entry price of the current position
    pub pnl_realized: f64, // Total realized Profit & Loss
    pub commissions_paid: f64, // Total commissions paid
    // Add more metrics like trade history, max drawdown, etc., as needed
}

impl BacktestPortfolio {
    fn new(starting_cash: f64) -> Self {
        Self {
            cash: starting_cash,
            position: 0.0,
            entry_price: None,
            pnl_realized: 0.0,
            commissions_paid: 0.0,
        }
    }

    // Simulate placing an order and its fill
    // This is a very basic fill model for backtesting: assume market orders fill at close price
    fn execute_order(&mut self, action: &OrderAction, current_kline: &KlineData) {
        let current_close_price = current_kline.close.parse::<f64>().unwrap_or_default();
        if current_close_price <= 0.0 {
            error!("Cannot execute order with non-positive close price.");
            return;
        }

        match action {
            OrderAction::Buy { symbol, quantity, price: _, order_type, client_order_id: _ } => {
                let qty_to_buy = *quantity;
                let cost = qty_to_buy * current_close_price;
                let commission = cost * COMMISSION_RATE;

                if self.cash >= cost + commission {
                    self.cash -= (cost + commission);
                    self.position += qty_to_buy;
                    self.commissions_paid += commission;

                    // Update average entry price
                    self.entry_price = Some(
                        (self.entry_price.unwrap_or(0.0) * (self.position - qty_to_buy).max(0.0) + cost) / self.position
                    ); // Simplified avg entry
                    info!("BUY Filled for {}: {} at {}. Cash left: {}", symbol, qty_to_buy, current_close_price, self.cash);
                } else {
                    debug!("Insufficient cash to buy {} {}. Has {}.", qty_to_buy, symbol, self.cash);
                }
            },
            OrderAction::Sell { symbol, quantity, price: _, order_type, client_order_id: _ } => {
                let qty_to_sell = *quantity;
                let proceeds = qty_to_sell * current_close_price;
                let commission = proceeds * COMMISSION_RATE;

                if self.position >= qty_to_sell {
                    self.cash += (proceeds - commission);
                    self.position -= qty_to_sell;
                    self.commissions_paid += commission;

                    if let Some(entry_p) = self.entry_price {
                        let pnl_this_trade = (current_close_price - entry_p) * qty_to_sell;
                        self.pnl_realized += pnl_this_trade;
                        info!("SELL Filled for {}: {} at {}. Realized PnL: {}. Cash left: {}", symbol, qty_to_sell, current_close_price, pnl_this_trade, self.cash);
                    } else {
                         info!("SELL Filled for {}: {} at {}. Cash left: {}", symbol, qty_to_sell, current_close_price, self.cash);
                    }
                    if self.position == 0.0 {
                        self.entry_price = None; // Reset entry price if position closed
                    }
                } else {
                    debug!("Insufficient position to sell {} {}. Has {}.", qty_to_sell, symbol, self.position);
                }
            },
            OrderAction::ClosePosition { symbol, side: _, quantity, order_type, client_order_id: _ } => {
                // This typically means closing the entire current position
                let qty_to_close = *quantity;
                 if self.position >= qty_to_close {
                    let proceeds = qty_to_close * current_close_price;
                    let commission = proceeds * COMMISSION_RATE;

                    self.cash += (proceeds - commission);
                    self.position -= qty_to_close;
                    self.commissions_paid += commission;

                    if let Some(entry_p) = self.entry_price {
                        let pnl_this_trade = (current_close_price - entry_p) * qty_to_close;
                        self.pnl_realized += pnl_this_trade;
                        info!("CLOSE Position Filled for {}: {} at {}. Realized PnL: {}. Cash left: {}", symbol, qty_to_close, current_close_price, pnl_this_trade, self.cash);
                    } else {
                         info!("CLOSE Position Filled for {}: {} at {}. Cash left: {}", symbol, qty_to_close, current_close_price, self.cash);
                    }
                    self.entry_price = None; // Position is closed
                 } else {
                    debug!("No position to close for {}. Has {}.", symbol, self.position);
                 }
            },
            OrderAction::Cancel { .. } | OrderAction::Hold => {
                debug!("Action: {:?}. No execution simulation needed.", action);
            }
        }
    }
}

fn load_klines_from_csv(file_path: &str) -> Result<Vec<KlineData>, String> {
    info!("Loading klines from CSV file: {}", file_path);
    let mut reader = csv::Reader::from_path(file_path)
        .map_err(|e| format!("Failed to open CSV file: {}", e))?;

    let mut klines: Vec<KlineData> = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| format!("Failed to read CSV record: {}", e))?;
        
        // Assuming CSV columns: Open time, Open, High, Low, Close, Volume, Close time, Quote asset volume, Number of trades, Taker buy base asset volume, Taker buy quote asset volume, Ignore
        if record.len() >= 12 {
            // Parse datetime strings to timestamps
            let open_time = NaiveDateTime::parse_from_str(&record[0], "%Y-%m-%d %H:%M:%S%.f")
                .map_err(|_| format!("Invalid Open time format: {}", record[0].to_string()))?
                .timestamp_millis() as u64;
            
            let open = record[1].to_string();
            let high = record[2].to_string();
            let low = record[3].to_string();
            let close = record[4].to_string();
            let volume = record[5].to_string();
            
            let close_time = NaiveDateTime::parse_from_str(&record[6], "%Y-%m-%d %H:%M:%S%.f")
                .map_err(|_| format!("Invalid Close time format: {}", record[6].to_string()))?
                .timestamp_millis() as u64;
            
            let quote_asset_volume = record[7].to_string();
            let number_of_trades = record[8].parse::<u64>().map_err(|_| "Invalid Number of trades")?;
            let taker_buy_base_asset_volume = record[9].to_string();
            let taker_buy_quote_asset_volume = record[10].to_string();
            let ignore = record[11].to_string();

            klines.push(KlineData {
                open_time,
                open,
                high,
                low,
                close,
                volume,
                close_time,
                symbol: BACKTEST_SYMBOL.to_string(), // Symbol is not in CSV, use constant
                interval: KlineInterval::H1.to_string(), // Interval is not in CSV, use constant
                first_trade_id: 0, // Not in CSV, default
                last_trade_id: 0, // Not in CSV, default
                number_of_trades,
                is_closed: true, // Assuming historical data is closed
                quote_asset_volume,
                taker_buy_base_asset_volume,
                taker_buy_quote_asset_volume,
                ignore,
            });
        } else {
            warn!("Skipping row with insufficient columns: {:?}", record);
        }
    }
    info!("Successfully loaded {} klines from CSV.", klines.len());
    Ok(klines)
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("--- Starting Excel Backtest for {} ---", BACKTEST_SYMBOL);

    // 1. Load Historical Data from Excel
    let historical_klines: Vec<KlineData> = load_klines_from_csv("E:\\Trading_Bot\\btc_15m_data_2018_to_2025.csv")?;

    if historical_klines.is_empty() {
        return Err("No historical data available for backtesting from Excel.".into());
    }

    // 2. Initialize Strategy and Portfolio
    let strategy_params = StrategyParameters::default(); // Use default parameters for now
    let strategy = MomentumStrategy::new(strategy_params.clone());
    let mut portfolio = BacktestPortfolio::new(STARTING_CAPITAL);

    info!("Backtest starting with initial capital: {}", portfolio.cash);

    // Keep a rolling window of klines for indicator calculations
    let mut kline_window: Vec<KlineData> = Vec::new();
    let required_window_size = strategy_params.slow_ma_period.max(strategy_params.rsi_period).max(strategy_params.atr_period) + 1;

    // 3. Run Backtest Loop
    for (i, current_kline) in historical_klines.iter().enumerate() {
        // Add current kline to the window
        kline_window.push(current_kline.clone());

        // Ensure window has enough data for indicators before evaluating
        if kline_window.len() < required_window_size {
            debug!("Skipping evaluation, not enough data in window ({} of {}).", kline_window.len(), required_window_size);
            continue; // Skip evaluation until enough data
        }

        // Slice the `kline_window` to ensure correct period is used for evaluation
        let evaluation_klines = &kline_window[kline_window.len() - required_window_size ..];

        // Get current price (using the close of the current kline for simplicity)
        let current_price = current_kline.close.parse::<f64>().unwrap_or_default();
        if current_price <= 0.0 {
            error!("Invalid current price in kline {}: {}", current_kline.close_time, current_kline.close);
            continue;
        }

        // Evaluate strategy
        let action = strategy.evaluate(
            BACKTEST_SYMBOL,
            evaluation_klines,
            portfolio.position,
            portfolio.cash,
            current_price,
        );

        debug!("Time: {} - Action: {:?}", current_kline.close_time, action);

        // Simulate order execution and update portfolio based on action
        portfolio.execute_order(&action, current_kline);

        // To prevent window from growing indefinitely, keep only the necessary recent data
        if kline_window.len() > required_window_size * 2 { // Keep a slightly larger buffer
            kline_window.drain(0..kline_window.len() - required_window_size);
        }
    }

    // 4. Finalize Portfolio (e.g., close any open positions)
    if portfolio.position > 0.0 {
        info!("Closing remaining position at end of backtest.");
        let last_kline = historical_klines.last().expect("No last kline for closing position.");
        let current_price = last_kline.close.parse::<f64>().unwrap_or_default();
        portfolio.execute_order(&OrderAction::ClosePosition {
            symbol: BACKTEST_SYMBOL.to_string(),
            side: crate::order::OrderSide::Sell, // Assuming it's a long position
            quantity: portfolio.position,
            order_type: crate::order::OrderType::Market,
            client_order_id: None,
        }, last_kline);
    }


    // 5. Display Backtest Results
    println!("\n--- Backtest Results ---");
    println!("Initial Capital: {:.2}", STARTING_CAPITAL);
    println!("Final Cash: {:.2}", portfolio.cash);
    println!("Realized PnL: {:.2}", portfolio.pnl_realized);
    println!("Commissions Paid: {:.2}", portfolio.commissions_paid);
    println!("Final Portfolio Value (Cash + Current Position): {:.2}",
             portfolio.cash + portfolio.position * historical_klines.last().map(|k| k.close.parse::<f64>().unwrap_or_default()).unwrap_or_default());

    // Display portfolio state using TUI
    display_struct_in_tui(&portfolio, "Final Backtest Portfolio State (Excel Data)").await?;

    Ok(())
}

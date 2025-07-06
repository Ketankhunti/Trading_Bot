use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::cmp::max;

// --- Configuration ---
const FAST_EMA_PERIOD: usize = 21;
const SLOW_EMA_PERIOD: usize = 55;
const RISK_REWARD_RATIO: f64 = 3.0; // Target a profit of 3x our risk.
const ACCOUNT_BALANCE: f64 = 5000.0; // Starting account balance for simulation.
const RISK_PERCENTAGE: f64 = 0.01; // We risk 1% of our account on each trade.

/// Represents a single candlestick data point from the official Binance CSV.
#[derive(Debug, Deserialize)]
struct Candle {
    #[serde(rename = "Open time")]
    timestamp: String,
    #[serde(rename = "Open")]
    open: f64,
    #[serde(rename = "High")]
    high: f64,
    #[serde(rename = "Low")]
    low: f64,
    #[serde(rename = "Close")]
    close: f64,
    #[serde(rename = "Volume")]
    volume: f64,
    #[serde(rename = "Close time")]
    close_time: String,
    #[serde(rename = "Quote asset volume")]
    quote_asset_volume: f64,
    #[serde(rename = "Number of trades")]
    number_of_trades: u32,
    #[serde(rename = "Taker buy base asset volume")]
    taker_buy_base_asset_volume: f64,
    #[serde(rename = "Taker buy quote asset volume")]
    taker_buy_quote_asset_volume: f64,
    #[serde(rename = "Ignore")]
    ignore: f64,
}


/// Represents an active trade, holding all necessary information.
#[derive(Debug)]
struct Trade {
    entry_price: f64,
    stop_loss: f64,
    take_profit: f64,
    position_size_btc: f64,
    risk_amount_usd: f64,
}

/// Main function to orchestrate the backtest.
pub fn run() -> Result<(), Box<dyn Error>> {
    println!("--- Starting Backtest (Full Metrics) ---");
    println!("Strategy: {}/{} EMA Crossover, {} a:1 Reward/Risk", FAST_EMA_PERIOD, SLOW_EMA_PERIOD, RISK_REWARD_RATIO);
    println!("Risk per trade: {}%", RISK_PERCENTAGE * 100.0);
    println!("------------------------------------------------");

    // 1. Load historical data from a CSV file.
    let candles = load_data("./btc_4h_data_2018_to_2025.csv")?;
    if candles.len() <= SLOW_EMA_PERIOD {
        panic!("Not enough historical data to perform the backtest.");
    }

    // 2. Calculate the EMAs for the entire dataset.
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let fast_emas = calculate_ema(&closes, FAST_EMA_PERIOD);
    let slow_emas = calculate_ema(&closes, SLOW_EMA_PERIOD);

    // 3. Run the backtesting simulation.
    run_simulation(&candles, &fast_emas, &slow_emas);

    Ok(())
}

/// Executes the main trading simulation loop.
fn run_simulation(candles: &[Candle], fast_emas: &[f64], slow_emas: &[f64]) {
    let mut current_trade: Option<Trade> = None;
    let mut balance = ACCOUNT_BALANCE;
    
    // Performance metrics
    let mut trade_history: Vec<f64> = Vec::new();
    let mut peak_balance = ACCOUNT_BALANCE;
    let mut max_drawdown = 0.0;
    
    // NEW: Metrics for losing streak calculation
    let mut consecutive_losses = 0;
    let mut max_consecutive_losses = 0;

    // We start the loop after the initial EMA calculation period.
    for i in SLOW_EMA_PERIOD..candles.len() {
        let current_candle = &candles[i];
        let previous_candle = &candles[i-1];
        
        // --- Trade Management ---
        if let Some(trade) = &current_trade {
            let mut trade_closed = false;
            let mut pnl = 0.0;

            // Check for Stop Loss
            if current_candle.low <= trade.stop_loss {
                pnl = (trade.stop_loss - trade.entry_price) * trade.position_size_btc;
                println!("[{}] STOP LOSS triggered at ${:.2}. P/L: ${:.2}", current_candle.timestamp, trade.stop_loss, pnl);
                trade_closed = true;
            } 
            // Check for Take Profit
            else if current_candle.high >= trade.take_profit {
                pnl = (trade.take_profit - trade.entry_price) * trade.position_size_btc;
                 println!("[{}] TAKE PROFIT hit at ${:.2}. P/L: ${:.2}", current_candle.timestamp, trade.take_profit, pnl);
                trade_closed = true;
            }

            if trade_closed {
                balance += pnl;
                trade_history.push(pnl);
                current_trade = None;
                
                // NEW: Update losing streak logic
                if pnl < 0.0 {
                    consecutive_losses += 1;
                } else {
                    max_consecutive_losses = max(max_consecutive_losses, consecutive_losses);
                    consecutive_losses = 0;
                }
                
                // Update drawdown metrics
                if balance > peak_balance {
                    peak_balance = balance;
                }
                let drawdown = (peak_balance - balance) / peak_balance;
                if drawdown > max_drawdown {
                    max_drawdown = drawdown;
                }
            }
        }

        // --- Entry Logic ---
        if current_trade.is_none() {
            let is_uptrend = fast_emas[i] > slow_emas[i];
            let pulled_back = previous_candle.close < fast_emas[i-1];
            let recovered = current_candle.close > fast_emas[i];

            if is_uptrend && pulled_back && recovered {
                let entry_price = current_candle.close;
                let stop_loss = current_candle.low;
                let risk_per_btc = entry_price - stop_loss;

                if risk_per_btc > 0.0 {
                    let risk_amount_usd = balance * RISK_PERCENTAGE;
                    let position_size_btc = risk_amount_usd / risk_per_btc;
                    let take_profit = entry_price + (risk_per_btc * RISK_REWARD_RATIO);
                    
                    let new_trade = Trade {
                        entry_price,
                        stop_loss,
                        take_profit,
                        position_size_btc,
                        risk_amount_usd,
                    };

                    println!("\n[{}] ==> ENTRY SIGNAL. Price: ${:.2}", current_candle.timestamp, new_trade.entry_price);
                    println!("    Stop: ${:.2}, Target: ${:.2}, Risking: ${:.2}\n", new_trade.stop_loss, new_trade.take_profit, new_trade.risk_amount_usd);
                    
                    current_trade = Some(new_trade);
                }
            }
        }
    }
    
    // Final check for losing streak in case the simulation ends on one.
    max_consecutive_losses = max(max_consecutive_losses, consecutive_losses);
    
    // --- Final Performance Report ---
    print_performance_report(&trade_history, balance, max_drawdown, max_consecutive_losses);
}


/// Calculates the Exponential Moving Average (EMA) for a series of values.
fn calculate_ema(data: &[f64], period: usize) -> Vec<f64> {
    let mut emas = vec![0.0; data.len()];
    let multiplier = 2.0 / (period as f64 + 1.0);
    let sum: f64 = data[0..period].iter().sum();
    emas[period - 1] = sum / period as f64;
    for i in period..data.len() {
        emas[i] = (data[i] - emas[i - 1]) * multiplier + emas[i - 1];
    }
    emas
}

/// Loads and parses historical price data from a CSV file.
fn load_data(file_path: &str) -> Result<Vec<Candle>, Box<dyn Error>> {
    let file = File::open(file_path)
        .map_err(|_| format!("Error: Could not find or open the file '{}'. Please ensure it's in the correct directory.", file_path))?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    let mut candles = Vec::with_capacity(10_000);
    for result in rdr.deserialize() {
        let candle: Candle = result?;
        candles.push(candle);
    }
    Ok(candles)
}

/// Prints a summary of the backtest's performance.
fn print_performance_report(history: &[f64], final_balance: f64, max_drawdown: f64, max_consecutive_losses: u32) {
    let total_trades = history.len();
    if total_trades == 0 {
        println!("\n--- No Trades Executed ---");
        return;
    }
    
    let winning_trades: Vec<f64> = history.iter().filter(|&&pnl| pnl > 0.0).cloned().collect();
    let losing_trades: Vec<f64> = history.iter().filter(|&&pnl| pnl < 0.0).cloned().collect();
    
    let win_rate = (winning_trades.len() as f64 / total_trades as f64) * 100.0;
    let total_pnl = history.iter().sum::<f64>();
    
    let gross_profit: f64 = winning_trades.iter().sum();
    let gross_loss: f64 = losing_trades.iter().sum::<f64>().abs();
    
    let profit_factor = if gross_loss > 0.0 { gross_profit / gross_loss } else { f64::INFINITY };

    // NEW: Calculate Average R/R Ratio
    let avg_win = if !winning_trades.is_empty() { gross_profit / winning_trades.len() as f64 } else { 0.0 };
    let avg_loss = if !losing_trades.is_empty() { gross_loss / losing_trades.len() as f64 } else { 0.0 };
    let realized_rr_ratio = if avg_loss > 0.0 { avg_win / avg_loss } else { f64::INFINITY };

    println!("\n--- Backtest Performance Report ---");
    println!("{:<25} | {:>15}", "Metric", "Value");
    println!("{:-<43}", "");
    println!("{:<25} | {:>15}", "Total Trades", total_trades);
    println!("{:<25} | {:>15}", "Winning Trades", winning_trades.len());
    println!("{:<25} | {:>15}", "Losing Trades", losing_trades.len());
    println!("{:<25} | {:>14.2}%", "Win Rate", win_rate);
    println!("{:<25} | ${:>14.2}", "Net Profit/Loss", total_pnl);
    println!("{:<25} | {:>15.2}", "Profit Factor", profit_factor);
    println!("{:<25} | {:>15.2}:1", "Avg. R/R Ratio", realized_rr_ratio); // NEW
    println!("{:<25} | {:>14.2}%", "Max Drawdown", max_drawdown * 100.0);
    println!("{:<25} | {:>15}", "Longest Losing Streak", max_consecutive_losses); // NEW
    println!("{:<25} | ${:>14.2}", "Starting Balance", ACCOUNT_BALANCE);
    println!("{:<25} | ${:>14.2}", "Final Balance", final_balance);
    println!("{:-<43}", "");
}

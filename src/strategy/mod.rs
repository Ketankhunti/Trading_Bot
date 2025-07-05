//! This module defines the Volatility and Momentum Trading Strategy.
//! It includes structures for strategy parameters, methods for calculating
//! technical indicators, and the core logic for generating trading signals.

use crate::streams::KlineData; // Import KlineData from streams module
use crate::order::{OrderSide, OrderType, TimeInForce}; // Not recommended
use log::{debug, info}; // Import log macros

/// Represents the action a trading strategy decides to take.
#[derive(Debug, PartialEq, Clone)]
pub enum OrderAction {
    /// Place a new buy order.
    Buy {
        symbol: String,
        quantity: f64,
        price: Option<f64>, // For Limit orders
        order_type: OrderType,
        client_order_id: Option<String>,
    },
    /// Place a new sell order.
    Sell {
        symbol: String,
        quantity: f64,
        price: Option<f64>, // For Limit orders
        order_type: OrderType,
        client_order_id: Option<String>,
    },
    /// Cancel an existing order.
    Cancel {
        symbol: String,
        order_id: Option<u64>,
        client_order_id: Option<String>,
    },
    /// Hold current positions, no action taken.
    Hold,
    /// Close an existing position (e.g., market sell if long, market buy if short).
    ClosePosition {
        symbol: String,
        side: OrderSide, // Which side of the position to close (e.g., if long, sell; if short, buy)
        quantity: f64,
        order_type: OrderType,
        client_order_id: Option<String>,
    },
}

/// Configuration parameters for the Volatility and Momentum Strategy.
#[derive(Debug, Clone)]
pub struct StrategyParameters {
    pub fast_ma_period: usize,  // Period for the fast Moving Average (e.g., 10)
    pub slow_ma_period: usize,  // Period for the slow Moving Average (e.g., 50)
    pub rsi_period: usize,      // Period for the Relative Strength Index (e.g., 14)
    pub rsi_overbought: f64,    // RSI level for overbought (e.g., 70.0)
    pub rsi_oversold: f64,      // RSI level for oversold (e.g., 30.0)
    pub atr_period: usize,      // Period for Average True Range (e.g., 14)
    pub atr_multiplier: f64,    // Multiplier for ATR to set stop-loss/take-profit (e.g., 1.5 or 2.0)
    pub risk_per_trade_percent: f64, // Percentage of capital to risk per trade (e.g., 0.01 for 1%)
    pub max_position_size_percent: f64, // Max percentage of capital to allocate to a single position
}

impl Default for StrategyParameters {
    fn default() -> Self {
        Self {
            fast_ma_period: 10,
            slow_ma_period: 50,
            rsi_period: 14,
            rsi_overbought: 70.0,
            rsi_oversold: 30.0,
            atr_period: 14,
            atr_multiplier: 1.5,
            risk_per_trade_percent: 0.01, // 1%
            max_position_size_percent: 0.05, // 5%
        }
    }
}

/// The Volatility and Momentum Trading Strategy implementation.
#[derive(Debug, Clone)]
pub struct MomentumStrategy {
    params: StrategyParameters,
    // State to maintain across klines (e.g., previous kline data for ATR, MA calculations)
    // For a real-time strategy, you'd need to store a rolling window of klines.
    // For backtesting, this would be managed by the backtesting engine.
}

impl MomentumStrategy {
    /// Creates a new `MomentumStrategy` instance with given parameters.
    pub fn new(params: StrategyParameters) -> Self {
        Self { params }
    }

    /// Calculates the Simple Moving Average (SMA) for a given period.
    ///
    /// # Arguments
    /// * `klines` - A slice of `KlineData` representing historical candles.
    /// * `period` - The period for the SMA calculation.
    ///
    /// # Returns
    /// The SMA value, or `None` if there aren't enough data points.
    pub fn calculate_sma(klines: &[KlineData], period: usize) -> Option<f64> {
        if klines.len() < period {
            return None;
        }
        let sum: f64 = klines
            .iter()
            .rev() // Start from the most recent kline
            .take(period)
            .filter_map(|k| k.close.parse::<f64>().ok())
            .sum();
        Some(sum / period as f64)
    }

    /// Calculates the Relative Strength Index (RSI).
    ///
    /// # Arguments
    /// * `klines` - A slice of `KlineData` representing historical candles.
    /// * `period` - The period for the RSI calculation.
    ///
    /// # Returns
    /// The RSI value (0-100), or `None` if there aren't enough data points.
    pub fn calculate_rsi(klines: &[KlineData], period: usize) -> Option<f64> {
        if klines.len() <= period {
            return None;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        // Calculate initial average gain/loss over the first 'period' candles
        let initial_klines = &klines[klines.len() - period - 1..]; // Need period + 1 for initial change
        let mut prev_close = initial_klines[0].close.parse::<f64>().ok()?;

        for i in 1..=period {
            let current_close = initial_klines[i].close.parse::<f64>().ok()?;
            let change = current_close - prev_close;
            if change > 0.0 {
                gains += change;
            } else {
                losses += change.abs();
            }
            prev_close = current_close;
        }

        let mut avg_gain = gains / period as f64;
        let mut avg_loss = losses / period as f64;

        // Calculate subsequent RSI values (only need the last one for the signal)
        for i in period + 1 .. klines.len() {
            let current_close = klines[i].close.parse::<f64>().ok()?;
            let change = current_close - prev_close;

            let current_gain = if change > 0.0 { change } else { 0.0 };
            let current_loss = if change < 0.0 { change.abs() } else { 0.0 };

            avg_gain = ((avg_gain * (period - 1) as f64) + current_gain) / period as f64;
            avg_loss = ((avg_loss * (period - 1) as f64) + current_loss) / period as f64;

            prev_close = current_close;
        }

        if avg_loss == 0.0 {
            return Some(100.0); // No losses, RSI is 100
        }

        let rs = avg_gain / avg_loss;
        Some(100.0 - (100.0 / (1.0 + rs)))
    }

    /// Calculates the True Range for a single candle.
    fn calculate_true_range(current: &KlineData, previous_close: f64) -> Option<f64> {
        let high = current.high.parse::<f64>().ok()?;
        let low = current.low.parse::<f64>().ok()?;
        let close = current.close.parse::<f64>().ok()?;

        let h_l = high - low;
        let h_pc = (high - previous_close).abs();
        let l_pc = (low - previous_close).abs();

        Some(h_l.max(h_pc).max(l_pc))
    }

    /// Calculates the Average True Range (ATR).
    ///
    /// # Arguments
    /// * `klines` - A slice of `KlineData` representing historical candles.
    /// * `period` - The period for the ATR calculation.
    ///
    /// # Returns
    /// The ATR value, or `None` if there aren't enough data points.
    pub fn calculate_atr(klines: &[KlineData], period: usize) -> Option<f64> {
        if klines.len() <= period {
            return None;
        }

        let mut true_ranges = Vec::new();
        let initial_klines = &klines[klines.len() - period - 1 ..]; // Need period + 1 for initial TR

        // Calculate initial True Ranges
        let mut prev_close = initial_klines[0].close.parse::<f64>().ok()?;
        for i in 1..=period {
            true_ranges.push(Self::calculate_true_range(&initial_klines[i], prev_close)?);
            prev_close = initial_klines[i].close.parse::<f64>().ok()?;
        }

        let mut atr = true_ranges.iter().sum::<f64>() / period as f64;

        // Calculate subsequent ATR values (Smoothed Moving Average of True Range)
        for i in period + 1 .. klines.len() {
            let current_kline = &klines[i];
            let prev_close_for_tr = klines[i-1].close.parse::<f64>().ok()?; // Previous candle's close
            let current_tr = Self::calculate_true_range(current_kline, prev_close_for_tr)?;
            atr = ((atr * (period - 1) as f64) + current_tr) / period as f64;
        }

        Some(atr)
    }

    /// Evaluates the strategy based on the latest market data and returns an `OrderAction`.
    ///
    /// # Arguments
    /// * `symbol` - The trading pair symbol (e.g., "BNBUSDT").
    /// * `klines` - A slice of `KlineData` representing the recent historical candles.
    ///              This should be sufficient to cover all indicator periods.
    /// * `current_position_quantity` - The current quantity of the asset held (0 if no position).
    /// * `available_balance` - The current available balance in the quote asset (e.g., USDT).
    /// * `current_price` - The latest market price of the asset.
    ///
    /// # Returns
    /// An `OrderAction` indicating whether to Buy, Sell, or Hold.
    pub fn evaluate(
        &self,
        symbol: &str,
        klines: &[KlineData],
        current_position_quantity: f64,
        available_balance: f64,
        current_price: f64,
    ) -> OrderAction {
        // Ensure enough data for all indicators
        let required_klines = self.params.slow_ma_period.max(self.params.rsi_period).max(self.params.atr_period) + 1;
        if klines.len() < required_klines {
            debug!("Not enough kline data for strategy evaluation. Needed: {}, Got: {}", required_klines, klines.len());
            return OrderAction::Hold;
        }

        // Get the most recent klines for calculation (ensure they are ordered oldest to newest)
        let recent_klines = &klines[klines.len() - required_klines..];

        let fast_ma = Self::calculate_sma(&recent_klines[recent_klines.len() - self.params.fast_ma_period..], self.params.fast_ma_period);
        let slow_ma = Self::calculate_sma(&recent_klines[recent_klines.len() - self.params.slow_ma_period..], self.params.slow_ma_period);
        let rsi = Self::calculate_rsi(recent_klines, self.params.rsi_period);
        let atr = Self::calculate_atr(recent_klines, self.params.atr_period);

        debug!("Strategy Eval for {}: FastMA={:?}, SlowMA={:?}, RSI={:?}, ATR={:?}", symbol, fast_ma, slow_ma, rsi, atr);

        // Check if all indicators are available
        let (fast_ma_val, slow_ma_val, rsi_val, atr_val) = match (fast_ma, slow_ma, rsi, atr) {
            (Some(f), Some(s), Some(r), Some(a)) => (f, s, r, a),
            _ => {
                debug!("One or more indicators could not be calculated. Holding.");
                return OrderAction::Hold;
            }
        };

        let current_kline_close = recent_klines.last().unwrap().close.parse::<f64>().unwrap_or(0.0);

        // --- Entry Logic (Long) ---
        // If no position is open
        if current_position_quantity <= 0.0 {
            let buy_signal = fast_ma_val > slow_ma_val // Fast MA above Slow MA (bullish trend)
                             && rsi_val > self.params.rsi_oversold // RSI not oversold
                             && rsi_val < self.params.rsi_overbought; // RSI not overbought (avoid buying at peak)

            if buy_signal {
                // Calculate position size based on risk management
                // Simplified: risk 1% of available_balance, stop-loss at ATR_multiplier * ATR below current price
                let stop_loss_distance = atr_val * self.params.atr_multiplier;
                let stop_loss_price = current_kline_close - stop_loss_distance;

                if stop_loss_price <= 0.0 { // Avoid negative stop-loss price
                    debug!("Calculated stop loss price is non-positive. Holding.");
                    return OrderAction::Hold;
                }

                let risk_per_unit = current_kline_close - stop_loss_price;
                if risk_per_unit <= 0.0 { // Avoid division by zero or negative risk
                    debug!("Calculated risk per unit is non-positive. Holding.");
                    return OrderAction::Hold;
                }

                let max_risk_amount = available_balance * self.params.risk_per_trade_percent;
                let calculated_quantity = max_risk_amount / risk_per_unit;

                // Ensure minimum notional value (e.g., 5 USDT for Binance Futures)
                let min_notional = 5.0; // This should ideally come from exchange info
                let notional_value = calculated_quantity * current_price;
                if notional_value < min_notional {
                    debug!("Calculated quantity results in notional value {} below minimum {}. Holding.", notional_value, min_notional);
                    return OrderAction::Hold;
                }


                // Ensure quantity is positive and within reasonable limits
                if calculated_quantity > 0.0 && calculated_quantity * current_price <= available_balance * self.params.max_position_size_percent {
                    info!("BUY Signal for {}. Quantity: {}, Price: {}", symbol, calculated_quantity, current_price);
                    return OrderAction::Buy {
                        symbol: symbol.to_string(),
                        quantity: calculated_quantity,
                        price: Some(current_price), // Use current price for market or limit entry
                        order_type: OrderType::Market, // Or Limit with calculated entry price
                        client_order_id: None, // Let exchange generate or provide a unique one
                    };
                } else {
                    debug!("Calculated quantity is not valid or exceeds max position size. Holding.");
                }
            }
        }
        // --- Exit Logic (Long) ---
        // If a position is open
        else {
            let sell_signal = fast_ma_val < slow_ma_val // Fast MA below Slow MA (bearish trend reversal)
                              || rsi_val >= self.params.rsi_overbought; // RSI is overbought

            if sell_signal {
                info!("SELL Signal for {}. Closing position.", symbol);
                return OrderAction::ClosePosition {
                    symbol: symbol.to_string(),
                    side: OrderSide::Sell, // Assuming we are closing a long position
                    quantity: current_position_quantity, // Close the entire position
                    order_type: OrderType::Market,
                    client_order_id: None,
                };
            }
        }

        OrderAction::Hold
    }
}

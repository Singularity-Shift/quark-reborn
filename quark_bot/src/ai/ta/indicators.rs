use super::types::*;
use anyhow::Result;

/// Calculate Simple Moving Average
fn calculate_sma(prices: &[f64], period: usize) -> Vec<f64> {
    if prices.len() < period {
        return vec![];
    }
    
    let mut sma_values = Vec::new();
    
    for i in (period - 1)..prices.len() {
        let sum: f64 = prices[(i + 1 - period)..=i].iter().sum();
        sma_values.push(sum / period as f64);
    }
    
    sma_values
}

/// Calculate RSI (Relative Strength Index)
fn calculate_rsi(prices: &[f64], period: usize) -> Vec<f64> {
    if prices.len() < period + 1 {
        return vec![];
    }
    
    let mut gains = Vec::new();
    let mut losses = Vec::new();
    
    // Calculate price changes
    for i in 1..prices.len() {
        let change = prices[i] - prices[i - 1];
        if change > 0.0 {
            gains.push(change);
            losses.push(0.0);
        } else {
            gains.push(0.0);
            losses.push(-change);
        }
    }
    
    let mut rsi_values = Vec::new();
    
    // Calculate RSI for each period
    for i in (period - 1)..gains.len() {
        let avg_gain: f64 = gains[(i + 1 - period)..=i].iter().sum::<f64>() / period as f64;
        let avg_loss: f64 = losses[(i + 1 - period)..=i].iter().sum::<f64>() / period as f64;
        
        if avg_loss == 0.0 {
            rsi_values.push(100.0);
        } else {
            let rs = avg_gain / avg_loss;
            let rsi = 100.0 - (100.0 / (1.0 + rs));
            rsi_values.push(rsi);
        }
    }
    
    rsi_values
}

/// Calculate Bollinger Bands
fn calculate_bollinger_bands(prices: &[f64], period: usize, std_dev: f64) -> Vec<(f64, f64, f64)> {
    let sma_values = calculate_sma(prices, period);
    let mut bands = Vec::new();
    
    for (i, &sma) in sma_values.iter().enumerate() {
        let start_idx = i + period - 1;
        if start_idx < prices.len() {
            let price_slice = &prices[(start_idx + 1 - period)..=start_idx];
            let variance: f64 = price_slice.iter()
                .map(|&price| (price - sma).powi(2))
                .sum::<f64>() / period as f64;
            let std_deviation = variance.sqrt();
            
            let upper_band = sma + (std_dev * std_deviation);
            let lower_band = sma - (std_dev * std_deviation);
            
            bands.push((lower_band, sma, upper_band));
        }
    }
    
    bands
}

/// MA Crossover Analysis (20/50 Golden/Death Cross) tuned for <=100 candles
pub fn calculate_ma_crossover(candles: &[OhlcvCandle]) -> Result<TaAnalysis> {
    if candles.len() < 50 {
        return Err(anyhow::anyhow!("Insufficient data for MA crossover analysis (need at least 50 candles)"));
    }

    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let sma20 = calculate_sma(&closes, 20);
    let sma50 = calculate_sma(&closes, 50);

    if sma20.is_empty() || sma50.is_empty() {
        return Err(anyhow::anyhow!("Failed to calculate moving averages"));
    }

    let current_price = closes.last().unwrap_or(&0.0);
    let current_sma20 = sma20.last().unwrap_or(&0.0);
    let current_sma50 = sma50.last().unwrap_or(&0.0);

    // Determine trend
    let current_trend = if current_sma20 > current_sma50 {
        Trend::Bullish
    } else if current_sma20 < current_sma50 {
        Trend::Bearish
    } else {
        Trend::Neutral
    };

    let mut signals = Vec::new();

    // Check for recent crossover
    if sma20.len() >= 2 && sma50.len() >= 2 {
        let prev_sma20 = sma20[sma20.len() - 2];
        let prev_sma50 = sma50[sma50.len() - 2];

        if prev_sma20 <= prev_sma50 && current_sma20 > current_sma50 {
            signals.push(Signal {
                signal_type: "Golden Cross".to_string(),
                strength: SignalStrength::Strong,
                description: "20-MA crossed above 50-MA - Bullish momentum".to_string(),
                price_level: Some(*current_price),
            });
        } else if prev_sma20 >= prev_sma50 && current_sma20 < current_sma50 {
            signals.push(Signal {
                signal_type: "Death Cross".to_string(),
                strength: SignalStrength::Strong,
                description: "20-MA crossed below 50-MA - Bearish momentum".to_string(),
                price_level: Some(*current_price),
            });
        }
    }

    // Price relative to MAs
    if current_price > current_sma20 && current_price > current_sma50 {
        signals.push(Signal {
            signal_type: "Price Above MAs".to_string(),
            strength: SignalStrength::Moderate,
            description: "Price trading above both MAs".to_string(),
            price_level: Some(*current_price),
        });
    } else if current_price < current_sma20 && current_price < current_sma50 {
        signals.push(Signal {
            signal_type: "Price Below MAs".to_string(),
            strength: SignalStrength::Moderate,
            description: "Price trading below both MAs".to_string(),
            price_level: Some(*current_price),
        });
    }

    let key_levels = KeyLevels {
        support: Some(current_sma20.min(*current_sma50)),
        resistance: Some(current_sma20.max(*current_sma50)),
        current_price: *current_price,
    };

    let summary = format!(
        "20-MA: ${:.4} | 50-MA: ${:.4} | Current: ${:.4}\nTrend: {} | Gap vs 20-MA: {:.2}%",
        current_sma20,
        current_sma50,
        current_price,
        current_trend,
        ((current_price - current_sma20) / current_sma20 * 100.0)
    );

    Ok(TaAnalysis {
        method: "20/50 Moving Average Crossover".to_string(),
        timeframe: "".to_string(),
        current_trend,
        signals,
        key_levels,
        summary,
        candle_count: candles.len(),
    })
}

/// RSI Divergence Analysis tuned for 100-candle window (14-period RSI)
pub fn calculate_rsi_divergence(candles: &[OhlcvCandle]) -> Result<TaAnalysis> {
    if candles.len() < 30 {
        return Err(anyhow::anyhow!("Insufficient data for RSI divergence analysis (need at least 30 candles)"));
    }
    
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let highs: Vec<f64> = candles.iter().map(|c| c.high).collect();
    let lows: Vec<f64> = candles.iter().map(|c| c.low).collect();
    
    let rsi_values = calculate_rsi(&closes, 14);
    
    if rsi_values.is_empty() {
        return Err(anyhow::anyhow!("Failed to calculate RSI"));
    }
    
    let current_price = closes.last().unwrap_or(&0.0);
    let current_rsi = rsi_values.last().unwrap_or(&50.0);
    
    // Determine trend based on RSI
    let current_trend = if *current_rsi > 70.0 {
        Trend::Bullish
    } else if *current_rsi < 30.0 {
        Trend::Bearish
    } else {
        Trend::Neutral
    };
    
    let mut signals = Vec::new();
    
    // RSI overbought/oversold
    if *current_rsi > 80.0 {
        signals.push(Signal {
            signal_type: "RSI Overbought".to_string(),
            strength: SignalStrength::Strong,
            description: format!("RSI extremely overbought at {:.1} - Consider taking profits", current_rsi),
            price_level: Some(*current_price),
        });
    } else if *current_rsi > 70.0 {
        signals.push(Signal {
            signal_type: "RSI Overbought".to_string(),
            strength: SignalStrength::Moderate,
            description: format!("RSI overbought at {:.1} - Caution on new longs", current_rsi),
            price_level: Some(*current_price),
        });
    }
    
    if *current_rsi < 20.0 {
        signals.push(Signal {
            signal_type: "RSI Oversold".to_string(),
            strength: SignalStrength::Strong,
            description: format!("RSI extremely oversold at {:.1} - Look for bounce", current_rsi),
            price_level: Some(*current_price),
        });
    } else if *current_rsi < 30.0 {
        signals.push(Signal {
            signal_type: "RSI Oversold".to_string(),
            strength: SignalStrength::Moderate,
            description: format!("RSI oversold at {:.1} - Potential buying opportunity", current_rsi),
            price_level: Some(*current_price),
        });
    }
    
    // Simple divergence check (compare last 10 periods)
    if rsi_values.len() >= 10 && closes.len() >= 10 {
        let recent_rsi = &rsi_values[rsi_values.len()-10..];
        let recent_prices = &closes[closes.len()-10..];
        
        let rsi_trend = recent_rsi.last().unwrap() - recent_rsi.first().unwrap();
        let price_trend = recent_prices.last().unwrap() - recent_prices.first().unwrap();
        
        if price_trend < 0.0 && rsi_trend > 0.0 {
            signals.push(Signal {
                signal_type: "Bullish Divergence".to_string(),
                strength: SignalStrength::Moderate,
                description: "Price falling while RSI rising - Potential reversal".to_string(),
                price_level: Some(*current_price),
            });
        } else if price_trend > 0.0 && rsi_trend < 0.0 {
            signals.push(Signal {
                signal_type: "Bearish Divergence".to_string(),
                strength: SignalStrength::Moderate,
                description: "Price rising while RSI falling - Potential reversal".to_string(),
                price_level: Some(*current_price),
            });
        }
    }
    
    let key_levels = KeyLevels {
        support: lows.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).copied(),
        resistance: highs.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).copied(),
        current_price: *current_price,
    };
    
    let summary = format!(
        "RSI(14): {:.1} | Status: {} | Momentum: {}",
        current_rsi,
        if *current_rsi > 70.0 { "Overbought" } 
        else if *current_rsi < 30.0 { "Oversold" } 
        else { "Neutral" },
        if *current_rsi > 50.0 { "Bullish" } else { "Bearish" }
    );
    
    Ok(TaAnalysis {
        method: "RSI Divergence (14-period)".to_string(),
        timeframe: "".to_string(),
        current_trend,
        signals,
        key_levels,
        summary,
        candle_count: candles.len(),
    })
}

/// Bollinger Bands Squeeze Analysis (20-SMA, 2σ)
pub fn calculate_bollinger_squeeze(candles: &[OhlcvCandle]) -> Result<TaAnalysis> {
    if candles.len() < 30 {
        return Err(anyhow::anyhow!("Insufficient data for Bollinger Bands analysis (need at least 30 candles)"));
    }
    
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let volumes: Vec<f64> = candles.iter().map(|c| c.volume).collect();
    
    let bands = calculate_bollinger_bands(&closes, 20, 2.0);
    
    if bands.is_empty() {
        return Err(anyhow::anyhow!("Failed to calculate Bollinger Bands"));
    }
    
    let current_price = closes.last().unwrap_or(&0.0);
    let current_volume = volumes.last().unwrap_or(&0.0);
    let (current_lower, current_middle, current_upper) = bands.last().unwrap();
    
    // Calculate band width for squeeze detection
    let band_width = (current_upper - current_lower) / current_middle * 100.0;
    
    // Historical band width for comparison
    let avg_band_width = if bands.len() >= 20 {
        let recent_widths: Vec<f64> = bands.iter()
            .rev()
            .take(20)
            .map(|(lower, middle, upper)| (upper - lower) / middle * 100.0)
            .collect();
        recent_widths.iter().sum::<f64>() / recent_widths.len() as f64
    } else {
        band_width
    };
    
    let current_trend = if *current_price > *current_middle {
        Trend::Bullish
    } else if *current_price < *current_middle {
        Trend::Bearish
    } else {
        Trend::Neutral
    };
    
    let mut signals = Vec::new();
    
    // Squeeze detection
    if band_width < avg_band_width * 0.7 {
        signals.push(Signal {
            signal_type: "Bollinger Squeeze".to_string(),
            strength: SignalStrength::Strong,
            description: format!("Bands squeezing (width: {:.2}%) - Breakout imminent", band_width),
            price_level: Some(*current_price),
        });
    }
    
    // Band position signals
    if *current_price > *current_upper {
        signals.push(Signal {
            signal_type: "Band Breakout".to_string(),
            strength: SignalStrength::Strong,
            description: "Price broke above upper band - Strong bullish momentum".to_string(),
            price_level: Some(*current_price),
        });
    } else if *current_price < *current_lower {
        signals.push(Signal {
            signal_type: "Band Breakdown".to_string(),
            strength: SignalStrength::Strong,
            description: "Price broke below lower band - Strong bearish momentum".to_string(),
            price_level: Some(*current_price),
        });
    } else if *current_price > current_middle + (current_upper - current_middle) * 0.5 {
        signals.push(Signal {
            signal_type: "Upper Band Approach".to_string(),
            strength: SignalStrength::Moderate,
            description: "Price approaching upper band - Potential resistance".to_string(),
            price_level: Some(*current_upper),
        });
    } else if *current_price < current_middle - (current_middle - current_lower) * 0.5 {
        signals.push(Signal {
            signal_type: "Lower Band Approach".to_string(),
            strength: SignalStrength::Moderate,
            description: "Price approaching lower band - Potential support".to_string(),
            price_level: Some(*current_lower),
        });
    }
    
    // Volume confirmation
    let avg_volume = if volumes.len() >= 20 {
        volumes.iter().rev().take(20).sum::<f64>() / 20.0
    } else {
        *current_volume
    };
    
    if *current_volume > avg_volume * 1.5 {
        signals.push(Signal {
            signal_type: "Volume Spike".to_string(),
            strength: SignalStrength::Moderate,
            description: format!("Volume spike detected ({:.1}x average) - Confirms breakout", current_volume / avg_volume),
            price_level: Some(*current_price),
        });
    }
    
    let key_levels = KeyLevels {
        support: Some(*current_lower),
        resistance: Some(*current_upper),
        current_price: *current_price,
    };
    
    let summary = format!(
        "BB(20,2): Lower ${:.4} | Middle ${:.4} | Upper ${:.4}\n\
        Width: {:.2}% | Position: {} | Squeeze: {}",
        current_lower,
        current_middle,
        current_upper,
        band_width,
        if *current_price > *current_middle { "Above Middle" } else { "Below Middle" },
        if band_width < avg_band_width * 0.7 { "YES ⚡" } else { "NO" }
    );
    
    Ok(TaAnalysis {
        method: "Bollinger Bands Squeeze & Pop (20-SMA, 2σ)".to_string(),
        timeframe: "".to_string(),
        current_trend,
        signals,
        key_levels,
        summary,
        candle_count: candles.len(),
    })
} 
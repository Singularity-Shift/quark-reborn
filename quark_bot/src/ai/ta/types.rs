#[derive(Debug, Clone)]
pub struct OhlcvCandle {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone)]
pub enum Trend {
    Bullish,
    Bearish,
    Neutral,
}

#[derive(Debug, Clone)]
pub enum SignalStrength {
    Strong,
    Moderate,
    Weak,
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub signal_type: String,
    pub strength: SignalStrength,
    pub description: String,
    pub price_level: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct KeyLevels {
    pub support: Option<f64>,
    pub resistance: Option<f64>,
    pub current_price: f64,
}

#[derive(Debug, Clone)]
pub struct TaAnalysis {
    pub method: String,
    pub timeframe: String,
    pub current_trend: Trend,
    pub signals: Vec<Signal>,
    pub key_levels: KeyLevels,
    pub summary: String,
    pub candle_count: usize,
}

impl std::fmt::Display for Trend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Trend::Bullish => write!(f, "BULLISH 🟢"),
            Trend::Bearish => write!(f, "BEARISH 🔴"),
            Trend::Neutral => write!(f, "NEUTRAL 🟡"),
        }
    }
}

impl std::fmt::Display for SignalStrength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalStrength::Strong => write!(f, "🔥 STRONG"),
            SignalStrength::Moderate => write!(f, "⚡ MODERATE"),
            SignalStrength::Weak => write!(f, "💫 WEAK"),
        }
    }
} 
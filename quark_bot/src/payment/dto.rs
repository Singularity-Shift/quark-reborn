use quark_core::helpers::dto::CoinVersion;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentPrefs {
    pub label: String,
    pub currency: String,
    pub version: CoinVersion,
}

impl From<(String, String, CoinVersion)> for PaymentPrefs {
    fn from((label, currency, version): (String, String, CoinVersion)) -> Self {
        Self {
            label,
            currency,
            version,
        }
    }
}

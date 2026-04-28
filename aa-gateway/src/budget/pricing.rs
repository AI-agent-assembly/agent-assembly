//! LLM pricing table — per-model USD cost per 1,000 tokens.

use rust_decimal::Decimal;

/// USD cost per 1,000 tokens for one direction (input or output).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PricingEntry {
    /// USD per 1,000 input tokens.
    #[serde(with = "rust_decimal::serde::str")]
    pub input_per_1k_usd: Decimal,
    /// USD per 1,000 output tokens.
    #[serde(with = "rust_decimal::serde::str")]
    pub output_per_1k_usd: Decimal,
}

/// Error loading the pricing JSON config.
#[derive(Debug)]
pub enum PricingLoadError {
    Json(serde_json::Error),
}

impl std::fmt::Display for PricingLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PricingLoadError::Json(e) => write!(f, "pricing JSON error: {e}"),
        }
    }
}

impl std::error::Error for PricingLoadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pricing_load_error_displays_message() {
        let raw = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let err = PricingLoadError::Json(raw);
        assert!(err.to_string().contains("pricing JSON error"));
    }

    #[test]
    fn pricing_entry_stores_rates() {
        fn d(s: &str) -> rust_decimal::Decimal {
            s.parse().unwrap()
        }
        let entry = PricingEntry {
            input_per_1k_usd: d("0.005"),
            output_per_1k_usd: d("0.015"),
        };
        assert_eq!(entry.input_per_1k_usd, d("0.005"));
        assert_eq!(entry.output_per_1k_usd, d("0.015"));
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

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

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

/// Flat JSON record used only for deserialization.
#[derive(serde::Deserialize)]
struct PricingJsonRow {
    provider: crate::budget::types::Provider,
    model: crate::budget::types::Model,
    #[serde(with = "rust_decimal::serde::str")]
    input_per_1k_usd: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    output_per_1k_usd: Decimal,
}

/// In-memory table mapping `(Provider, Model)` to pricing.
#[derive(Debug, Clone)]
pub struct PricingTable {
    entries: std::collections::HashMap<(crate::budget::types::Provider, crate::budget::types::Model), PricingEntry>,
}

impl PricingTable {
    /// Build the default embedded pricing table (2024 list prices).
    pub fn default_table() -> Self {
        use crate::budget::types::{Model, Provider};
        fn d(s: &str) -> Decimal {
            s.parse().expect("embedded literal")
        }

        let rows: &[(Provider, Model, &str, &str)] = &[
            (Provider::OpenAi, Model::Gpt4o, "0.005", "0.015"),
            (Provider::OpenAi, Model::Gpt4, "0.03", "0.06"),
            (Provider::OpenAi, Model::Gpt35Turbo, "0.0005", "0.0015"),
            (Provider::Anthropic, Model::Claude3Opus, "0.015", "0.075"),
            (Provider::Anthropic, Model::Claude3Sonnet, "0.003", "0.015"),
            (Provider::Anthropic, Model::Claude3Haiku, "0.00025", "0.00125"),
            (Provider::Cohere, Model::CommandRPlus, "0.003", "0.015"),
            (Provider::Cohere, Model::CommandR, "0.0005", "0.0015"),
        ];

        let entries = rows
            .iter()
            .map(|(prov, model, inp, out)| {
                (
                    (*prov, *model),
                    PricingEntry {
                        input_per_1k_usd: d(inp),
                        output_per_1k_usd: d(out),
                    },
                )
            })
            .collect();

        Self { entries }
    }

    /// Load pricing overrides from a JSON string, merging on top of the defaults.
    pub fn load_from_json_str(json: &str) -> Result<Self, PricingLoadError> {
        let rows: Vec<PricingJsonRow> = serde_json::from_str(json).map_err(PricingLoadError::Json)?;
        let mut table = Self::default_table();
        for row in rows {
            table.entries.insert(
                (row.provider, row.model),
                PricingEntry {
                    input_per_1k_usd: row.input_per_1k_usd,
                    output_per_1k_usd: row.output_per_1k_usd,
                },
            );
        }
        Ok(table)
    }

    /// Load from a file path. Returns `default_table()` silently on any I/O or parse error.
    pub fn load_from_file(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(json) => Self::load_from_json_str(&json).unwrap_or_else(|e| {
                eprintln!("aa-gateway: pricing.json parse error ({e}); using defaults");
                Self::default_table()
            }),
            Err(_) => Self::default_table(),
        }
    }

    /// Look up pricing for a `(provider, model)` pair.
    pub fn entry(
        &self,
        provider: crate::budget::types::Provider,
        model: crate::budget::types::Model,
    ) -> Option<&PricingEntry> {
        self.entries.get(&(provider, model))
    }
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
    fn load_from_file_falls_back_to_defaults_on_missing_file() {
        let path = std::path::Path::new("/nonexistent/path/pricing.json");
        let table = PricingTable::load_from_file(path);
        use crate::budget::types::{Model, Provider};
        assert!(table.entry(Provider::OpenAi, Model::Gpt4o).is_some());
    }

    #[test]
    fn load_from_json_str_overrides_gpt4o_input_price() {
        use crate::budget::types::{Model, Provider};
        fn d(s: &str) -> rust_decimal::Decimal {
            s.parse().unwrap()
        }
        let json = r#"[
          { "provider": "open_ai", "model": "gpt4o",
            "input_per_1k_usd": "0.999", "output_per_1k_usd": "0.015" }
        ]"#;
        let table = PricingTable::load_from_json_str(json).unwrap();
        let entry = table.entry(Provider::OpenAi, Model::Gpt4o).unwrap();
        assert_eq!(entry.input_per_1k_usd, d("0.999"));
        // Non-overridden models keep defaults
        assert!(table.entry(Provider::Anthropic, Model::Claude3Opus).is_some());
    }

    #[test]
    fn default_table_contains_all_eight_models() {
        use crate::budget::types::{Model, Provider};
        let table = PricingTable::default_table();
        for (prov, model) in [
            (Provider::OpenAi, Model::Gpt4o),
            (Provider::OpenAi, Model::Gpt4),
            (Provider::OpenAi, Model::Gpt35Turbo),
            (Provider::Anthropic, Model::Claude3Opus),
            (Provider::Anthropic, Model::Claude3Sonnet),
            (Provider::Anthropic, Model::Claude3Haiku),
            (Provider::Cohere, Model::CommandRPlus),
            (Provider::Cohere, Model::CommandR),
        ] {
            assert!(table.entry(prov, model).is_some(), "{prov:?}/{model:?} missing");
        }
    }

    #[test]
    fn default_table_gpt4o_has_correct_rates() {
        use crate::budget::types::{Model, Provider};
        fn d(s: &str) -> rust_decimal::Decimal {
            s.parse().unwrap()
        }
        let table = PricingTable::default_table();
        let entry = table.entry(Provider::OpenAi, Model::Gpt4o).unwrap();
        assert_eq!(entry.input_per_1k_usd, d("0.005"));
        assert_eq!(entry.output_per_1k_usd, d("0.015"));
    }

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

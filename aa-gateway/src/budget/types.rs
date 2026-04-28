//! Core domain types for the budget tracking engine.

/// LLM provider identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// OpenAI (GPT-* models).
    OpenAi,
    /// Anthropic (Claude models).
    Anthropic,
    /// Cohere (Command models).
    Cohere,
}

/// LLM model identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Model {
    // OpenAI
    Gpt4o,
    Gpt4,
    Gpt35Turbo,
    // Anthropic
    Claude3Opus,
    Claude3Sonnet,
    Claude3Haiku,
    // Cohere
    CommandRPlus,
    CommandR,
}

/// Result returned by [`super::tracker::BudgetTracker::record_usage`].
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetStatus {
    /// Spend is below the 80% alert threshold.
    WithinBudget { spent_usd: f64, remaining_usd: f64 },
    /// Spend crossed 80% or 95% of the daily limit.
    ThresholdAlert { pct: u8 },
    /// Daily limit reached or exceeded — caller should block the LLM call.
    LimitExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_variants_are_distinct() {
        assert_eq!(Provider::OpenAi, Provider::OpenAi);
        assert_ne!(Provider::OpenAi, Provider::Anthropic);
        assert_ne!(Provider::OpenAi, Provider::Cohere);
        assert_ne!(Provider::Anthropic, Provider::Cohere);
    }

    #[test]
    fn model_variants_are_distinct() {
        assert_eq!(Model::Gpt4o, Model::Gpt4o);
        assert_ne!(Model::Gpt4o, Model::Gpt4);
        assert_ne!(Model::Claude3Opus, Model::Claude3Haiku);
        assert_ne!(Model::CommandRPlus, Model::CommandR);
    }

    #[test]
    fn budget_status_within_budget_holds_values() {
        let s = BudgetStatus::WithinBudget {
            spent_usd: 5.0,
            remaining_usd: 45.0,
        };
        match s {
            BudgetStatus::WithinBudget {
                spent_usd,
                remaining_usd,
            } => {
                assert!((spent_usd - 5.0).abs() < f64::EPSILON);
                assert!((remaining_usd - 45.0).abs() < f64::EPSILON);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn budget_status_threshold_alert_holds_pct() {
        let s = BudgetStatus::ThresholdAlert { pct: 80 };
        assert_eq!(s, BudgetStatus::ThresholdAlert { pct: 80 });
        assert_ne!(s, BudgetStatus::ThresholdAlert { pct: 95 });
    }
}

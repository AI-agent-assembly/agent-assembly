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

/// Per-agent accumulated spend for a single UTC calendar day.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BudgetState {
    /// Total USD spent today using exact decimal arithmetic.
    #[serde(with = "rust_decimal::serde::str")]
    pub spent_usd: rust_decimal::Decimal,
    /// UTC calendar date this state is valid for.
    pub date: chrono::NaiveDate,
}

impl BudgetState {
    /// Create a fresh zero-spend state stamped with today's UTC date.
    pub fn new_today() -> Self {
        Self {
            spent_usd: rust_decimal::Decimal::ZERO,
            date: chrono::Utc::now().date_naive(),
        }
    }

    /// Reset `spent_usd` to zero if `date` is before today UTC. No-op if same day.
    pub fn maybe_reset(&mut self) {
        let today = chrono::Utc::now().date_naive();
        if self.date < today {
            self.spent_usd = rust_decimal::Decimal::ZERO;
            self.date = today;
        }
    }
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

    #[test]
    fn budget_state_new_today_has_zero_spend() {
        use chrono::Utc;
        use rust_decimal::Decimal;
        let state = BudgetState::new_today();
        assert_eq!(state.spent_usd, Decimal::ZERO);
        assert_eq!(state.date, Utc::now().date_naive());
    }

    #[test]
    fn budget_state_maybe_reset_clears_old_date() {
        use chrono::Utc;
        use rust_decimal::Decimal;
        let mut state = BudgetState {
            spent_usd: Decimal::new(500, 2), // 5.00
            date: Utc::now().date_naive() - chrono::Duration::days(1),
        };
        state.maybe_reset();
        assert_eq!(state.spent_usd, Decimal::ZERO);
        assert_eq!(state.date, Utc::now().date_naive());
    }

    #[test]
    fn budget_state_maybe_reset_same_day_is_noop() {
        use chrono::Utc;
        use rust_decimal::Decimal;
        let amount = Decimal::new(500, 2); // 5.00
        let mut state = BudgetState {
            spent_usd: amount,
            date: Utc::now().date_naive(),
        };
        state.maybe_reset();
        assert_eq!(state.spent_usd, amount);
    }
}

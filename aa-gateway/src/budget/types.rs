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
}

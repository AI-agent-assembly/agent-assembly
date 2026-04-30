//! Output format selection for CLI commands.

use clap::ValueEnum;

/// Output format for list and get commands.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable table (default).
    #[default]
    Table,
    /// Machine-readable JSON.
    Json,
    /// Machine-readable YAML.
    Yaml,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_table() {
        assert!(matches!(OutputFormat::default(), OutputFormat::Table));
    }

    #[test]
    fn value_variants_contains_all_formats() {
        let variants = OutputFormat::value_variants();
        assert_eq!(variants.len(), 3);
    }
}

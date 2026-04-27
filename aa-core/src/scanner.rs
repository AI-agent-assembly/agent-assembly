//! Credential leak detection using Aho-Corasick multi-pattern scanning.
//!
//! Only compiled when the `std` feature is enabled. The [`CredentialScanner`]
//! is pre-compiled at construction time so each call to [`CredentialScanner::scan`]
//! pays zero pattern-compilation cost.

use aho_corasick::AhoCorasick;

// ---------------------------------------------------------------------------
// AC literal patterns — order matters: earlier index wins on same-position match.
// sk-ant- must precede sk- so Anthropic keys are not misclassified as OpenAI keys.
// ---------------------------------------------------------------------------

const AC_PATTERNS: &[&str] = &[
    "sk-ant-",                               // 0  AnthropicKey
    "sk-",                                   // 1  OpenAiKey
    "AKIA",                                  // 2  AwsAccessKey
    "\"type\": \"service_account\"",         // 3  GcpServiceAccount
    "DefaultEndpointsProtocol=",             // 4  AzureConnectionString
    "ghp_",                                  // 5  GitHubPat
    "ghs_",                                  // 6  GitHubAppToken
    "xoxb-",                                 // 7  SlackBotToken
    "xoxp-",                                 // 8  SlackUserToken
    "xoxa-",                                 // 9  SlackOAuthToken
    "postgres://",                           // 10 PostgresUrl
    "mysql://",                              // 11 MysqlUrl
    "mongodb://",                            // 12 MongodbUrl
    "-----BEGIN RSA PRIVATE KEY-----",       // 13 RsaPrivateKey
    "-----BEGIN EC PRIVATE KEY-----",        // 14 EcPrivateKey
    "-----BEGIN OPENSSH PRIVATE KEY-----",   // 15 OpensshPrivateKey
    "-----BEGIN PRIVATE KEY-----",           // 16 PrivateKey
    "-----BEGIN PGP PRIVATE KEY BLOCK-----", // 17 PgpPrivateKey
];

/// Maps AC pattern index → [`CredentialKind`].
const AC_KINDS: &[CredentialKind] = &[
    CredentialKind::AnthropicKey,          // 0
    CredentialKind::OpenAiKey,             // 1
    CredentialKind::AwsAccessKey,          // 2
    CredentialKind::GcpServiceAccount,     // 3
    CredentialKind::AzureConnectionString, // 4
    CredentialKind::GitHubPat,             // 5
    CredentialKind::GitHubAppToken,        // 6
    CredentialKind::SlackBotToken,         // 7
    CredentialKind::SlackUserToken,        // 8
    CredentialKind::SlackOAuthToken,       // 9
    CredentialKind::PostgresUrl,           // 10
    CredentialKind::MysqlUrl,              // 11
    CredentialKind::MongodbUrl,            // 12
    CredentialKind::RsaPrivateKey,         // 13
    CredentialKind::EcPrivateKey,          // 14
    CredentialKind::OpensshPrivateKey,     // 15
    CredentialKind::PrivateKey,            // 16
    CredentialKind::PgpPrivateKey,         // 17
];

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Category of a detected credential or sensitive value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CredentialKind {
    // API keys
    AnthropicKey,
    AwsAccessKey,
    GcpServiceAccount,
    OpenAiKey,
    // Cloud credentials
    AzureConnectionString,
    // Auth tokens
    GitHubAppToken,
    GitHubPat,
    SlackBotToken,
    SlackOAuthToken,
    SlackUserToken,
    // Database URLs
    MongodbUrl,
    MysqlUrl,
    PostgresUrl,
    // Private keys
    EcPrivateKey,
    OpensshPrivateKey,
    PgpPrivateKey,
    PrivateKey,
    RsaPrivateKey,
    // PII
    CreditCardLuhn,
    EmailAddress,
    SsnPattern,
    // Generic
    GenericHighEntropy,
}

impl CredentialKind {
    /// Returns the string used in the `[REDACTED:<kind>]` label.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AnthropicKey          => "AnthropicKey",
            Self::AwsAccessKey          => "AwsAccessKey",
            Self::AzureConnectionString => "AzureConnectionString",
            Self::CreditCardLuhn        => "CreditCardLuhn",
            Self::EcPrivateKey          => "EcPrivateKey",
            Self::EmailAddress          => "EmailAddress",
            Self::GcpServiceAccount     => "GcpServiceAccount",
            Self::GenericHighEntropy    => "GenericHighEntropy",
            Self::GitHubAppToken        => "GitHubAppToken",
            Self::GitHubPat             => "GitHubPat",
            Self::MongodbUrl            => "MongodbUrl",
            Self::MysqlUrl              => "MysqlUrl",
            Self::OpenAiKey             => "OpenAiKey",
            Self::OpensshPrivateKey     => "OpensshPrivateKey",
            Self::PgpPrivateKey         => "PgpPrivateKey",
            Self::PostgresUrl           => "PostgresUrl",
            Self::PrivateKey            => "PrivateKey",
            Self::RsaPrivateKey         => "RsaPrivateKey",
            Self::SlackBotToken         => "SlackBotToken",
            Self::SlackOAuthToken       => "SlackOAuthToken",
            Self::SlackUserToken        => "SlackUserToken",
            Self::SsnPattern            => "SsnPattern",
        }
    }
}

/// A single detected credential finding.
///
/// `offset` is the byte offset in the original text where the pattern was found.
/// `matched` is the redacted label, e.g. `[REDACTED:AwsAccessKey]`. The raw
/// secret is never stored.
///
/// The `end` field is intentionally private; it is used by [`ScanResult::redact`]
/// to splice the original match without exposing raw length arithmetic to callers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CredentialFinding {
    pub kind:    CredentialKind,
    pub offset:  usize,
    pub matched: String,
    #[cfg_attr(feature = "serde", serde(skip))]
    end: usize,
}

impl CredentialFinding {
    fn new(kind: CredentialKind, offset: usize, end: usize) -> Self {
        let label = format!("[REDACTED:{}]", kind.as_str());
        Self { kind, offset, matched: label, end }
    }
}

/// The result of a [`CredentialScanner::scan`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScanResult {
    pub findings: Vec<CredentialFinding>,
}

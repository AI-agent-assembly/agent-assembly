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

impl ScanResult {
    /// Returns `true` if no credential findings were detected.
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// Returns a copy of `text` with every finding replaced by its redacted label.
    ///
    /// Replacements are applied in reverse offset order so earlier byte positions
    /// remain valid after each splice. The `end` field of each finding records the
    /// original match boundary and is used here without being exposed in the public API.
    pub fn redact(&self, text: &str) -> String {
        let mut sorted: Vec<&CredentialFinding> = self.findings.iter().collect();
        sorted.sort_by(|a, b| b.offset.cmp(&a.offset));
        let mut result = text.to_string();
        for finding in sorted {
            if finding.end <= result.len() && finding.offset <= finding.end {
                result.replace_range(finding.offset..finding.end, &finding.matched);
            }
        }
        result
    }
}

/// Pre-compiled multi-pattern credential scanner.
///
/// Construct once with [`CredentialScanner::new`] and call [`CredentialScanner::scan`]
/// repeatedly. Pattern compilation happens at construction time; each scan call is
/// O(n) in the length of the input text.
pub struct CredentialScanner {
    patterns: AhoCorasick,
}

impl Default for CredentialScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialScanner {
    /// Build the scanner, compiling all patterns into an Aho-Corasick automaton.
    ///
    /// # Panics
    ///
    /// Panics only if the hard-coded AC patterns are somehow invalid — this
    /// cannot happen in practice.
    pub fn new() -> Self {
        let ac = AhoCorasick::builder()
            .match_kind(aho_corasick::MatchKind::LeftmostFirst)
            .build(AC_PATTERNS)
            .expect("static AC patterns are always valid");
        Self { patterns: ac }
    }

    /// Scan `text` for credential patterns and return a [`ScanResult`].
    ///
    /// Four passes are performed:
    /// 1. Aho-Corasick literal prefix scan — O(n), 18 patterns covering API keys,
    ///    auth tokens, cloud credentials, database URLs, and PEM private key headers.
    /// 2. Credit card and SSN digit-sequence scan.
    /// 3. Email address scan.
    /// 4. High-entropy token scan (Shannon entropy > 4.5 bits/char, length 20–64).
    pub fn scan(&self, text: &str) -> ScanResult {
        let mut findings = Vec::new();

        // Phase 1: AC literal prefix scan (API keys, auth tokens, cloud creds,
        //          database URLs, PEM private key headers — 18 patterns)
        for mat in self.patterns.find_iter(text) {
            let kind = AC_KINDS[mat.pattern()].clone();
            let offset = mat.start();
            let end = token_end(text, mat.end());
            findings.push(CredentialFinding::new(kind, offset, end));
        }

        // Phase 2: PII — credit card numbers and SSN patterns
        scan_digit_sequences(text, &mut findings);

        // Phase 3: Email addresses
        scan_emails(text, &mut findings);

        // Phase 4: High-entropy tokens (Shannon entropy > 4.5 bits/char, length 20–64)
        scan_high_entropy(text, &mut findings);

        findings.sort_by_key(|f| f.offset);
        ScanResult { findings }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Returns the byte index of the first token-terminating character at or after
/// `from`. Token terminators are whitespace and common delimiters.
fn token_end(text: &str, from: usize) -> usize {
    text[from..]
        .find(|c: char| {
            c.is_whitespace() || matches!(c, '"' | '\'' | ',' | ';' | ')' | ']' | '}')
        })
        .map(|i| from + i)
        .unwrap_or(text.len())
}

/// Returns `true` if `s` matches the SSN format `DDD-DD-DDDD` exactly.
fn is_ssn(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 11
        && b[0..3].iter().all(u8::is_ascii_digit)
        && b[3] == b'-'
        && b[4..6].iter().all(u8::is_ascii_digit)
        && b[6] == b'-'
        && b[7..11].iter().all(u8::is_ascii_digit)
}

/// Returns `true` if `digits` (ASCII digit characters only, no separators) passes
/// the Luhn checksum algorithm used by credit card numbers.
fn luhn_valid(digits: &str) -> bool {
    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }
    let mut sum = 0u32;
    let mut double = false;
    for ch in digits.chars().rev() {
        let Some(d) = ch.to_digit(10) else {
            return false;
        };
        let val = if double {
            let v = d * 2;
            if v > 9 { v - 9 } else { v }
        } else {
            d
        };
        sum += val;
        double = !double;
    }
    sum % 10 == 0
}

/// Scans `text` for credit card numbers (Luhn-validated) and SSN patterns (`DDD-DD-DDDD`).
fn scan_digit_sequences(text: &str, findings: &mut Vec<CredentialFinding>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }

        let start = i;
        let mut digits = String::new();
        let mut j = i;
        let limit = (start + 24).min(bytes.len());

        while j < limit {
            match bytes[j] {
                b if b.is_ascii_digit() => {
                    digits.push(b as char);
                    j += 1;
                }
                b' ' | b'-' if !digits.is_empty() => {
                    j += 1;
                }
                _ => break,
            }
        }

        let end = j;
        let segment = &text[start..end];

        if is_ssn(segment) {
            findings.push(CredentialFinding::new(CredentialKind::SsnPattern, start, end));
        } else if digits.len() >= 13 && digits.len() <= 19 && luhn_valid(&digits) {
            findings.push(CredentialFinding::new(CredentialKind::CreditCardLuhn, start, end));
        }
        i = end.max(i + 1);
    }
}

/// Computes the Shannon entropy of `s` in bits per character.
fn shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let mut freq = [0u32; 256];
    for &b in s.as_bytes() {
        freq[b as usize] += 1;
    }
    let len = s.len() as f64;
    freq.iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / len;
            -p * p.log2()
        })
        .sum()
}

/// Scans `text` for high-entropy whitespace-delimited tokens (> 4.5 bits/char,
/// length 20–64 bytes) and reports them as [`CredentialKind::GenericHighEntropy`].
fn scan_high_entropy(text: &str, findings: &mut Vec<CredentialFinding>) {
    let mut offset = 0usize;
    for token in text.split_whitespace() {
        let token_offset = text[offset..].find(token).map(|i| offset + i).unwrap_or(offset);
        let token_end_pos = token_offset + token.len();
        let len = token.len();
        if len >= 20 && len <= 64 && shannon_entropy(token) > 4.5 {
            findings.push(CredentialFinding::new(
                CredentialKind::GenericHighEntropy,
                token_offset,
                token_end_pos,
            ));
        }
        offset = token_end_pos;
    }
}

/// Scans `text` for email addresses by locating `@` signs and expanding outward.
fn scan_emails(text: &str, findings: &mut Vec<CredentialFinding>) {
    let mut search = text;
    let mut base = 0usize;

    while let Some(at) = search.find('@') {
        let abs_at = base + at;

        let local_start = text[..abs_at]
            .rfind(|c: char| c.is_whitespace() || matches!(c, '<' | ',' | ';' | '"' | '\''))
            .map(|i| i + 1)
            .unwrap_or(0);

        let domain_end = token_end(text, abs_at + 1);
        let local = &text[local_start..abs_at];
        let domain = &text[abs_at + 1..domain_end];

        if !local.is_empty() && domain.contains('.') && domain.len() >= 3 {
            findings.push(CredentialFinding::new(
                CredentialKind::EmailAddress,
                local_start,
                domain_end,
            ));
        }

        let next = abs_at + 1;
        if next >= text.len() {
            break;
        }
        search = &text[next..];
        base = next;
    }
}

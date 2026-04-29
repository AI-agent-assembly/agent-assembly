# AAASM-82: API Key and JWT Authentication — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement API key + JWT authentication for `aa-api` endpoints as `FromRequestParts` extractors, with per-key rate limiting (1000 req/min token bucket), scope enforcement, `AA_AUTH=off` bypass mode, and `POST /api/v1/auth/token` JWT issuance endpoint.

**Architecture:** Auth is extractor-based (not Tower middleware). `AuthenticatedCaller` extractor parses `Authorization: Bearer <token>`, validates as API key or JWT, checks rate limit, and injects caller identity. `RequireScope` extractor enforces per-handler scope requirements. All auth modules live in `aa-api/src/auth/`.

**Tech Stack:** Rust, Axum 0.7, jsonwebtoken, argon2, rand, dashmap, tokio

**Spec:** `docs/superpowers/specs/2026-04-29-aaasm-82-api-key-jwt-auth-design.md`

---

## Branch

```
v0.0.1/AAASM-82/feat/api_key_jwt_auth
```

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Modify | `aa-api/Cargo.toml` | Add auth dependencies |
| Create | `aa-api/src/auth/config.rs` | AuthConfig (env vars, validation) |
| Create | `aa-api/src/auth/scope.rs` | Scope enum + RequireScope extractor |
| Create | `aa-api/src/auth/api_key.rs` | ApiKey newtype, ApiKeyEntry, ApiKeyStore |
| Create | `aa-api/src/auth/jwt.rs` | JwtSigner, JwtVerifier, Claims |
| Create | `aa-api/src/auth/rate_limit.rs` | TokenBucket, RateLimiter |
| Create | `aa-api/src/auth/mod.rs` | AuthenticatedCaller, AuthError, FromRequestParts |
| Create | `aa-api/src/routes/auth.rs` | POST /api/v1/auth/token handler |
| Modify | `aa-api/src/state.rs` | Add auth fields to AppState |
| Modify | `aa-api/src/config.rs` | Add AuthConfig to ApiConfig |
| Modify | `aa-api/src/routes/mod.rs` | Register auth route |
| Modify | `aa-api/src/server.rs` | Wire auth into build_app |
| Modify | `aa-api/src/lib.rs` | Export auth module |
| Modify | `aa-api/src/middleware/mod.rs` | Update doc comment |
| Create | `aa-api/tests/auth_api_key.rs` | API key integration tests |
| Create | `aa-api/tests/auth_jwt.rs` | JWT integration tests |
| Create | `aa-api/tests/auth_rate_limit.rs` | Rate limit integration tests |
| Create | `aa-api/tests/auth_scope.rs` | Scope enforcement integration tests |
| Create | `aa-api/tests/auth_bypass.rs` | AA_AUTH=off bypass integration tests |
| Modify | `aa-api/tests/common/mod.rs` | Add auth test helpers |

---

### Task 1: Add auth dependencies to Cargo.toml

**Files:**
- Modify: `aa-api/Cargo.toml`

- [ ] **Step 1: Add `jsonwebtoken`, `argon2`, `rand`, `dashmap` dependencies**

Add to `[dependencies]`:
```toml
jsonwebtoken = "9"
argon2 = "0.5"
rand = "0.8"
dashmap = "6"
```

- [ ] **Step 2: Commit**

```
✨ (aa-api): Add auth dependencies (jsonwebtoken, argon2, rand, dashmap)
```

---

### Task 2: Add Scope enum

**Files:**
- Create: `aa-api/src/auth/scope.rs`

- [ ] **Step 1: Create `Scope` enum with `Read`, `Write`, `Admin` variants**

```rust
/// Authorization scope level for API operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Read,
    Write,
    Admin,
}
```

Include `Scope::contains()` method that checks if a set of scopes satisfies a required scope.

- [ ] **Step 2: Commit**

```
✨ (auth): Add Scope enum with Read, Write, Admin variants
```

---

### Task 3: Add AuthConfig

**Files:**
- Create: `aa-api/src/auth/config.rs`

- [ ] **Step 1: Create `AuthMode` enum (`On`, `Off`) and `AuthConfig` struct**

```rust
pub enum AuthMode { On, Off }

pub struct AuthConfig {
    pub mode: AuthMode,
    pub jwt_secret: Option<Vec<u8>>,  // None when mode=Off
    pub api_keys_path: PathBuf,
    pub rate_limit_rpm: u32,
}
```

- [ ] **Step 2: Implement `AuthConfig::from_env()`**

Reads `AA_AUTH`, `AA_JWT_SECRET`, `AA_API_KEYS_PATH`, `AA_RATE_LIMIT_RPM`. Validates:
- `AA_JWT_SECRET` required when mode=On, min 32 bytes
- Logs warning when mode=Off
- Returns `Result<AuthConfig, AuthConfigError>`

- [ ] **Step 3: Commit**

```
✨ (auth): Add AuthConfig with env var parsing and fail-closed validation
```

---

### Task 4: Add ApiKey newtype and format validation

**Files:**
- Create: `aa-api/src/auth/api_key.rs`

- [ ] **Step 1: Create `ApiKey` newtype with `aa_<32-hex>` format**

```rust
pub struct ApiKey(String);
```

Implement `ApiKey::parse(raw: &str) -> Result<Self, ApiKeyError>` — validates `aa_` prefix + 32 hex chars. Implement `ApiKey::generate() -> (Self, String)` — returns key + plaintext for display.

- [ ] **Step 2: Commit**

```
✨ (auth): Add ApiKey newtype with format validation and generation
```

---

### Task 5: Add ApiKeyEntry and ApiKeyStore

**Files:**
- Modify: `aa-api/src/auth/api_key.rs`

- [ ] **Step 1: Add `ApiKeyEntry` struct**

```rust
pub struct ApiKeyEntry {
    pub id: String,
    pub key_hash: String,      // argon2 hash
    pub scopes: Vec<Scope>,
    pub created_at: u64,       // unix timestamp
    pub label: Option<String>,
}
```

- [ ] **Step 2: Commit**

```
✨ (auth): Add ApiKeyEntry struct with hashed key storage
```

---

### Task 6: Add ApiKeyStore (load + validate)

**Files:**
- Modify: `aa-api/src/auth/api_key.rs`

- [ ] **Step 1: Implement `ApiKeyStore`**

```rust
pub struct ApiKeyStore {
    entries: Vec<ApiKeyEntry>,
}
```

`ApiKeyStore::load(path: &Path) -> Result<Self, ApiKeyError>` — loads from JSON file, creates empty store if file doesn't exist.
`ApiKeyStore::validate(&self, raw_key: &str) -> Option<&ApiKeyEntry>` — parses key, compares argon2 hash against all entries.

- [ ] **Step 2: Commit**

```
✨ (auth): Add ApiKeyStore with JSON file loading and hash validation
```

---

### Task 7: Add unit tests for API key module

**Files:**
- Modify: `aa-api/src/auth/api_key.rs`

- [ ] **Step 1: Add `#[cfg(test)]` module with tests**

Tests:
- `test_api_key_generate_format` — generated key matches `aa_<32-hex>` pattern
- `test_api_key_parse_valid` — valid format parses successfully
- `test_api_key_parse_invalid_prefix` — missing `aa_` prefix rejected
- `test_api_key_parse_invalid_length` — wrong hex length rejected
- `test_api_key_store_validate_roundtrip` — generate + store + validate succeeds
- `test_api_key_store_validate_wrong_key` — wrong key returns None
- `test_api_key_store_load_missing_file` — returns empty store

- [ ] **Step 2: Commit**

```
✅ (auth): Add unit tests for API key format validation and store
```

---

### Task 8: Add JWT Claims struct

**Files:**
- Create: `aa-api/src/auth/jwt.rs`

- [ ] **Step 1: Create `Claims` struct**

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,           // api_key_id
    pub iat: u64,              // issued at (unix timestamp)
    pub exp: u64,              // expiry (unix timestamp)
    pub scope: Vec<Scope>,     // authorized scopes
}
```

- [ ] **Step 2: Commit**

```
✨ (auth): Add JWT Claims struct with sub, iat, exp, scope fields
```

---

### Task 9: Add JwtSigner

**Files:**
- Modify: `aa-api/src/auth/jwt.rs`

- [ ] **Step 1: Implement `JwtSigner`**

```rust
pub struct JwtSigner {
    encoding_key: EncodingKey,
}
```

`JwtSigner::new(secret: &[u8]) -> Self`
`JwtSigner::sign(&self, key_id: &str, scopes: &[Scope]) -> Result<String, JwtError>` — creates token with 24h expiry

- [ ] **Step 2: Commit**

```
✨ (auth): Add JwtSigner with HMAC-SHA256 signing and 24h expiry
```

---

### Task 10: Add JwtVerifier

**Files:**
- Modify: `aa-api/src/auth/jwt.rs`

- [ ] **Step 1: Implement `JwtVerifier`**

```rust
pub struct JwtVerifier {
    decoding_key: DecodingKey,
    validation: Validation,
}
```

`JwtVerifier::new(secret: &[u8]) -> Self` — configures HS256, validates exp
`JwtVerifier::verify(&self, token: &str) -> Result<Claims, JwtError>`

- [ ] **Step 2: Commit**

```
✨ (auth): Add JwtVerifier with signature and expiry validation
```

---

### Task 11: Add unit tests for JWT module

**Files:**
- Modify: `aa-api/src/auth/jwt.rs`

- [ ] **Step 1: Add `#[cfg(test)]` module with tests**

Tests:
- `test_jwt_sign_verify_roundtrip` — sign then verify returns same claims
- `test_jwt_expired_token_rejected` — token with past expiry fails verification
- `test_jwt_wrong_secret_rejected` — token signed with different secret fails
- `test_jwt_scopes_preserved` — scopes in claims survive roundtrip

- [ ] **Step 2: Commit**

```
✅ (auth): Add unit tests for JWT sign/verify roundtrip
```

---

### Task 12: Add TokenBucket struct

**Files:**
- Create: `aa-api/src/auth/rate_limit.rs`

- [ ] **Step 1: Create `TokenBucket` with consume/refill logic**

```rust
pub struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64,    // tokens per second
    last_refill: Instant,
}
```

`TokenBucket::new(capacity: u32) -> Self`
`TokenBucket::try_consume(&mut self) -> Result<(), RetryAfter>` — refills based on elapsed time, then consumes one token. Returns seconds until next token if empty.

- [ ] **Step 2: Commit**

```
✨ (auth): Add TokenBucket with consume and time-based refill
```

---

### Task 13: Add RateLimiter (concurrent per-key map)

**Files:**
- Modify: `aa-api/src/auth/rate_limit.rs`

- [ ] **Step 1: Implement `RateLimiter`**

```rust
pub struct RateLimiter {
    buckets: DashMap<String, TokenBucket>,
    capacity: u32,
}
```

`RateLimiter::new(rpm: u32) -> Self`
`RateLimiter::check(&self, key_id: &str) -> Result<(), RetryAfter>` — gets or creates bucket, calls try_consume

- [ ] **Step 2: Commit**

```
✨ (auth): Add RateLimiter with per-key DashMap token bucket lookup
```

---

### Task 14: Add unit tests for rate limiter

**Files:**
- Modify: `aa-api/src/auth/rate_limit.rs`

- [ ] **Step 1: Add `#[cfg(test)]` module with tests**

Tests:
- `test_token_bucket_allows_within_limit` — consuming tokens under capacity succeeds
- `test_token_bucket_rejects_over_limit` — consuming past capacity returns RetryAfter
- `test_token_bucket_refills_over_time` — after waiting, tokens become available
- `test_rate_limiter_per_key_isolation` — different keys have independent buckets

- [ ] **Step 2: Commit**

```
✅ (auth): Add unit tests for token bucket and rate limiter
```

---

### Task 15: Add AuthError enum

**Files:**
- Create: `aa-api/src/auth/mod.rs`

- [ ] **Step 1: Create `AuthError` enum with `IntoResponse` impl**

```rust
pub enum AuthError {
    MissingHeader,
    InvalidToken(String),
    ExpiredToken,
    RateLimited { retry_after_secs: u64 },
    InsufficientScope { required: Scope },
}
```

Implement `IntoResponse` — maps to `ProblemDetail` with appropriate status code (401/403/429). `RateLimited` variant adds `Retry-After` header.

- [ ] **Step 2: Commit**

```
✨ (auth): Add AuthError enum with ProblemDetail response mapping
```

---

### Task 16: Add AuthenticatedCaller struct

**Files:**
- Modify: `aa-api/src/auth/mod.rs`

- [ ] **Step 1: Create `AuthenticatedCaller` struct**

```rust
pub struct AuthenticatedCaller {
    pub key_id: String,
    pub scopes: Vec<Scope>,
}
```

- [ ] **Step 2: Commit**

```
✨ (auth): Add AuthenticatedCaller identity struct
```

---

### Task 17: Implement FromRequestParts for AuthenticatedCaller

**Files:**
- Modify: `aa-api/src/auth/mod.rs`

- [ ] **Step 1: Implement `FromRequestParts<()>` for `AuthenticatedCaller`**

Pipeline:
1. Get `AuthConfig` from request extensions — if `Off`, return synthetic admin caller
2. Parse `Authorization: Bearer <token>` header
3. If token starts with `aa_` → validate via `ApiKeyStore`
4. Otherwise → validate via `JwtVerifier`
5. Check rate limit via `RateLimiter`
6. Return `AuthenticatedCaller { key_id, scopes }`

- [ ] **Step 2: Commit**

```
✨ (auth): Implement FromRequestParts for AuthenticatedCaller
```

---

### Task 18: Add RequireScope extractor

**Files:**
- Modify: `aa-api/src/auth/scope.rs`

- [ ] **Step 1: Implement `RequireScope` extractor**

```rust
pub struct RequireScope(pub Scope);
```

`FromRequestParts` impl: reads `AuthenticatedCaller` from request extensions, checks if caller's scopes contain the required scope, returns `AuthError::InsufficientScope` if not.

- [ ] **Step 2: Commit**

```
✨ (auth): Add RequireScope FromRequestParts extractor
```

---

### Task 19: Add unit tests for scope enforcement

**Files:**
- Modify: `aa-api/src/auth/scope.rs`

- [ ] **Step 1: Add `#[cfg(test)]` module with tests**

Tests:
- `test_scope_ordering` — Admin > Write > Read
- `test_scope_contains_same_level` — Write satisfies Write
- `test_scope_contains_higher_level` — Admin satisfies Write
- `test_scope_rejects_lower_level` — Read does not satisfy Write

- [ ] **Step 2: Commit**

```
✅ (auth): Add unit tests for scope ordering and enforcement
```

---

### Task 20: Export auth module from lib.rs

**Files:**
- Modify: `aa-api/src/lib.rs`

- [ ] **Step 1: Add `pub mod auth;` to lib.rs**

- [ ] **Step 2: Commit**

```
✨ (aa-api): Export auth module from crate root
```

---

### Task 21: Add auth fields to AppState

**Files:**
- Modify: `aa-api/src/state.rs`

- [ ] **Step 1: Add auth-related fields to `AppState`**

```rust
pub auth_config: Arc<auth::config::AuthConfig>,
pub key_store: Arc<auth::api_key::ApiKeyStore>,
pub rate_limiter: Arc<auth::rate_limit::RateLimiter>,
pub jwt_signer: Arc<auth::jwt::JwtSigner>,
pub jwt_verifier: Arc<auth::jwt::JwtVerifier>,
```

- [ ] **Step 2: Commit**

```
✨ (state): Add auth config, key store, rate limiter, JWT fields to AppState
```

---

### Task 22: Add AuthConfig to ApiConfig

**Files:**
- Modify: `aa-api/src/config.rs`

- [ ] **Step 1: Extend `ApiConfig` with auth config, update `from_env()`**

`ApiConfig::from_env()` calls `AuthConfig::from_env()` — if auth is enabled and config is invalid, return error (fail-closed).

- [ ] **Step 2: Commit**

```
✨ (config): Integrate AuthConfig into ApiConfig with fail-closed startup
```

---

### Task 23: Add unit tests for AuthConfig

**Files:**
- Modify: `aa-api/src/auth/config.rs`

- [ ] **Step 1: Add `#[cfg(test)]` module with tests**

Tests:
- `test_config_auth_off_no_secret_required` — `AA_AUTH=off` succeeds without `AA_JWT_SECRET`
- `test_config_auth_on_missing_secret_fails` — auth enabled without secret returns error
- `test_config_auth_on_short_secret_fails` — secret shorter than 32 bytes returns error
- `test_config_auth_on_valid_secret_succeeds` — 32+ byte secret succeeds
- `test_config_default_rate_limit` — default is 1000 rpm
- `test_config_custom_rate_limit` — `AA_RATE_LIMIT_RPM=500` sets 500

- [ ] **Step 2: Commit**

```
✅ (auth): Add unit tests for AuthConfig env var parsing
```

---

### Task 24: Add POST /api/v1/auth/token route handler

**Files:**
- Create: `aa-api/src/routes/auth.rs`

- [ ] **Step 1: Implement `issue_token` handler**

```rust
pub async fn issue_token(
    caller: AuthenticatedCaller,
    Extension(state): Extension<AppState>,
    Json(body): Json<TokenRequest>,
) -> Result<Json<TokenResponse>, ProblemDetail>
```

`TokenRequest`: optional `scope: Vec<Scope>` — requests subset of caller's scopes.
`TokenResponse`: `{ token: String, expires_at: u64, scopes: Vec<Scope> }`.

Handler validates requested scopes are subset of caller's, signs JWT, returns response.

- [ ] **Step 2: Commit**

```
✨ (routes): Add POST /api/v1/auth/token JWT issuance endpoint
```

---

### Task 25: Wire auth route into v1 router

**Files:**
- Modify: `aa-api/src/routes/mod.rs`

- [ ] **Step 1: Add `pub mod auth;` and register `POST /auth/token` in `v1_router()`**

```rust
.route("/auth/token", post(auth::issue_token))
```

- [ ] **Step 2: Commit**

```
🔧 (routes): Register auth/token route in v1 router
```

---

### Task 26: Wire auth into server startup

**Files:**
- Modify: `aa-api/src/server.rs`

- [ ] **Step 1: Update `build_app` to construct auth state and inject into extensions**

Build `AuthConfig`, `ApiKeyStore`, `RateLimiter`, `JwtSigner`, `JwtVerifier` from config. Store in `AppState`. The `AuthenticatedCaller` extractor reads these from extensions at request time.

- [ ] **Step 2: Commit**

```
🔧 (server): Wire auth state into build_app and server startup
```

---

### Task 27: Update middleware doc comment

**Files:**
- Modify: `aa-api/src/middleware/mod.rs`

- [ ] **Step 1: Update the module doc comment**

Replace `"Authentication middleware will be added by AAASM-82."` with `"Authentication is handled by FromRequestParts extractors (see auth module), not middleware layers."`.

- [ ] **Step 2: Commit**

```
📝 (middleware): Update doc comment to reflect extractor-based auth
```

---

### Task 28: Add auth test helpers to common module

**Files:**
- Modify: `aa-api/tests/common/mod.rs`

- [ ] **Step 1: Add helpers for creating test app with auth enabled/disabled**

Helpers:
- `test_app_with_auth(secret, api_keys) -> TestServer` — builds app with auth enabled
- `test_app_no_auth() -> TestServer` — builds app with `AA_AUTH=off`
- `generate_test_api_key(store, scopes) -> String` — creates key and returns plaintext
- `generate_test_jwt(signer, key_id, scopes) -> String` — creates JWT for testing

- [ ] **Step 2: Commit**

```
✅ (test): Add auth test helpers to common module
```

---

### Task 29: Add API key integration tests

**Files:**
- Create: `aa-api/tests/auth_api_key.rs`

- [ ] **Step 1: Add integration tests for API key auth flow**

Tests:
- `test_valid_api_key_grants_access` — request with valid key → 200
- `test_invalid_api_key_returns_401` — request with wrong key → 401 ProblemDetail
- `test_missing_auth_header_returns_401` — no header → 401 ProblemDetail
- `test_malformed_bearer_returns_401` — `Authorization: Basic ...` → 401

- [ ] **Step 2: Commit**

```
✅ (test): Add API key authentication integration tests
```

---

### Task 30: Add JWT integration tests

**Files:**
- Create: `aa-api/tests/auth_jwt.rs`

- [ ] **Step 1: Add integration tests for JWT auth flow**

Tests:
- `test_valid_jwt_grants_access` — request with valid JWT → 200
- `test_expired_jwt_returns_401` — expired JWT → 401
- `test_wrong_secret_jwt_returns_401` — JWT signed with different secret → 401
- `test_token_endpoint_issues_jwt` — POST /api/v1/auth/token with API key → JWT in response
- `test_token_endpoint_respects_scope_subset` — requested scopes must be subset of key's scopes

- [ ] **Step 2: Commit**

```
✅ (test): Add JWT authentication integration tests
```

---

### Task 31: Add rate limit integration tests

**Files:**
- Create: `aa-api/tests/auth_rate_limit.rs`

- [ ] **Step 1: Add integration tests for rate limiting**

Tests:
- `test_rate_limit_allows_under_threshold` — N requests under limit all succeed
- `test_rate_limit_returns_429_with_retry_after` — exceed limit → 429 + `Retry-After` header
- `test_rate_limit_per_key_isolation` — different keys have independent limits

- [ ] **Step 2: Commit**

```
✅ (test): Add rate limit integration tests with 429 and Retry-After
```

---

### Task 32: Add scope enforcement integration tests

**Files:**
- Create: `aa-api/tests/auth_scope.rs`

- [ ] **Step 1: Add integration tests for scope enforcement**

Tests:
- `test_read_scope_allows_get` — read-only key can GET endpoints
- `test_read_scope_blocks_post` — read-only key POST → 403
- `test_write_scope_allows_post` — write key can POST
- `test_admin_scope_allows_all` — admin key can access everything

- [ ] **Step 2: Commit**

```
✅ (test): Add scope enforcement integration tests
```

---

### Task 33: Add AA_AUTH=off bypass integration tests

**Files:**
- Create: `aa-api/tests/auth_bypass.rs`

- [ ] **Step 1: Add integration tests for bypass mode**

Tests:
- `test_bypass_mode_allows_unauthenticated` — no credentials → 200 when AA_AUTH=off
- `test_bypass_mode_grants_admin_scope` — bypass caller has admin scope

- [ ] **Step 2: Commit**

```
✅ (test): Add AA_AUTH=off bypass mode integration tests
```

---

## Summary

| # | Commit message | Files touched |
|---|---------------|--------------|
| 1 | `✨ (aa-api): Add auth dependencies` | `Cargo.toml` |
| 2 | `✨ (auth): Add Scope enum` | `auth/scope.rs` |
| 3 | `✨ (auth): Add AuthConfig` | `auth/config.rs` |
| 4 | `✨ (auth): Add ApiKey newtype` | `auth/api_key.rs` |
| 5 | `✨ (auth): Add ApiKeyEntry struct` | `auth/api_key.rs` |
| 6 | `✨ (auth): Add ApiKeyStore` | `auth/api_key.rs` |
| 7 | `✅ (auth): Add API key unit tests` | `auth/api_key.rs` |
| 8 | `✨ (auth): Add JWT Claims struct` | `auth/jwt.rs` |
| 9 | `✨ (auth): Add JwtSigner` | `auth/jwt.rs` |
| 10 | `✨ (auth): Add JwtVerifier` | `auth/jwt.rs` |
| 11 | `✅ (auth): Add JWT unit tests` | `auth/jwt.rs` |
| 12 | `✨ (auth): Add TokenBucket` | `auth/rate_limit.rs` |
| 13 | `✨ (auth): Add RateLimiter` | `auth/rate_limit.rs` |
| 14 | `✅ (auth): Add rate limiter unit tests` | `auth/rate_limit.rs` |
| 15 | `✨ (auth): Add AuthError enum` | `auth/mod.rs` |
| 16 | `✨ (auth): Add AuthenticatedCaller struct` | `auth/mod.rs` |
| 17 | `✨ (auth): Implement FromRequestParts` | `auth/mod.rs` |
| 18 | `✨ (auth): Add RequireScope extractor` | `auth/scope.rs` |
| 19 | `✅ (auth): Add scope unit tests` | `auth/scope.rs` |
| 20 | `✨ (aa-api): Export auth module` | `lib.rs` |
| 21 | `✨ (state): Add auth fields to AppState` | `state.rs` |
| 22 | `✨ (config): Integrate AuthConfig into ApiConfig` | `config.rs` |
| 23 | `✅ (auth): Add AuthConfig unit tests` | `auth/config.rs` |
| 24 | `✨ (routes): Add POST /api/v1/auth/token` | `routes/auth.rs` |
| 25 | `🔧 (routes): Register auth/token in v1 router` | `routes/mod.rs` |
| 26 | `🔧 (server): Wire auth state into build_app` | `server.rs` |
| 27 | `📝 (middleware): Update doc comment` | `middleware/mod.rs` |
| 28 | `✅ (test): Add auth test helpers` | `tests/common/mod.rs` |
| 29 | `✅ (test): Add API key integration tests` | `tests/auth_api_key.rs` |
| 30 | `✅ (test): Add JWT integration tests` | `tests/auth_jwt.rs` |
| 31 | `✅ (test): Add rate limit integration tests` | `tests/auth_rate_limit.rs` |
| 32 | `✅ (test): Add scope enforcement integration tests` | `tests/auth_scope.rs` |
| 33 | `✅ (test): Add bypass mode integration tests` | `tests/auth_bypass.rs` |

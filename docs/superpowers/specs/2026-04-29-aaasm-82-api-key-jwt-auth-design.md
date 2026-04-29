# AAASM-82: API Key and JWT Authentication for aa-api

**Ticket:** AAASM-82
**Epic:** AAASM-9 (REST API Layer & OpenAPI Contract)
**Date:** 2026-04-29
**Status:** Approved

## Problem

The `aa-api` crate (AAASM-78) provides an Axum HTTP server with router, middleware stack, and health endpoint. All endpoints are currently unauthenticated. The API needs two authentication modes — API key (for CLI/integrations) and JWT (for dashboard sessions) — with configurable bypass for local development.

## Design Decisions

### 1. Auth Middleware Architecture: FromRequestParts Extractor

**Decision:** Pure Axum `FromRequestParts` extractor (`AuthenticatedCaller`), not a Tower `Layer`/`Service`.

**Rationale:**
- Matches the ticket spec ("Axum `FromRequestParts` extractor")
- Per-route granularity: public endpoints (`/health`) simply omit the extractor — no brittle path allowlist
- Rate limiting (per-key) naturally belongs in the auth path where the key is already identified
- Scope enforcement composes as a separate `RequireScope` extractor
- Testable without a running server: `AuthenticatedCaller` can be constructed directly in tests

### 2. JWT Secret Source: Environment Variable

**Decision:** `AA_JWT_SECRET` environment variable. Server refuses to start if unset (unless `AA_AUTH=off`).

**Rationale:**
- 12-factor compliance: secrets from environment, not filesystem
- `AA_AUTH=off` already covers the dev convenience case
- Fail-closed: missing secret + auth enabled = startup error with clear message
- Minimum 32 bytes (256 bits), validated at startup
- Read once at startup, held in memory, no hot-reload (restart invalidates all JWTs)

### 3. API Key Format and Storage

**Decision:** `aa_<32-hex-chars>` format, stored hashed in `~/.aa/api-keys.json`.

**Rationale:**
- `aa_` prefix helps detect accidental credential exposure in logs/repos
- Keys are hashed (argon2) at rest — plaintext only shown once at generation time
- JSON file format consistent with existing project patterns (`budget/persistence.rs`)

## Architecture

### Module Structure

```
aa-api/src/auth/
├── mod.rs           // AuthenticatedCaller extractor (FromRequestParts impl)
├── api_key.rs       // API key generation, validation, storage (api-keys.json)
├── jwt.rs           // JWT sign/verify (HMAC-SHA256, 24h expiry, scopes)
├── rate_limit.rs    // In-memory token bucket (per key, 1000 req/min)
├── scope.rs         // RequireScope extractor, Scope enum (read/write/admin)
└── config.rs        // AuthConfig (AA_AUTH, AA_JWT_SECRET, AA_API_KEYS_PATH)
```

### Data Flow

```
Request with Authorization: Bearer <token>
    │
    ▼
AuthenticatedCaller::from_request_parts()
    │
    ├── AA_AUTH=off? → return synthetic admin caller
    │
    ├── Parse "Bearer <token>" from Authorization header
    │   └── Missing/malformed → 401 ProblemDetail
    │
    ├── Token starts with "aa_"?
    │   ├── Yes → validate against ApiKeyStore (hash comparison)
    │   └── No  → validate as JWT (verify signature, check expiry)
    │       └── Invalid → 401 ProblemDetail
    │
    ├── Check rate limit (token bucket for resolved key ID)
    │   └── Exceeded → 429 ProblemDetail + Retry-After header
    │
    └── Store CallerIdentity + scopes in request extensions
        └── Return AuthenticatedCaller { key_id, scopes }
```

### Components

**`auth/config.rs` — AuthConfig**
- `AA_AUTH`: `"on"` (default) or `"off"` (bypass mode, logs warning)
- `AA_JWT_SECRET`: HMAC key, min 32 bytes, required when auth enabled
- `AA_API_KEYS_PATH`: path to keys file (default `~/.aa/api-keys.json`)
- `AA_RATE_LIMIT_RPM`: requests per minute per key (default 1000)

**`auth/api_key.rs` — API Key Management**
- `ApiKey`: newtype wrapping `aa_<32-hex-chars>` string
- `ApiKeyEntry`: `{ id, key_hash, scopes, created_at, label }`
- `ApiKeyStore`: loads from JSON file, `validate(token) -> Option<ApiKeyEntry>`
- `ApiKey::generate()`: cryptographically random via `rand`

**`auth/jwt.rs` — JWT Operations**
- Claims: `{ sub: api_key_id, iat, exp, scope: ["read","write","admin"] }`
- `JwtSigner`: holds HMAC key, `sign(key_id, scopes) -> String`
- `JwtVerifier`: `verify(token) -> Result<Claims, AuthError>`
- 24-hour expiry, HMAC-SHA256
- Uses `jsonwebtoken` crate

**`auth/rate_limit.rs` — Token Bucket**
- `RateLimiter`: `DashMap<String, TokenBucket>` keyed by API key ID
- `TokenBucket`: capacity=configurable (default 1000), refill=capacity/minute
- `check(key_id) -> Result<(), RetryAfter>`: returns seconds until next available token
- Stale entry cleanup: keys not seen for >1h removed on next check

**`auth/scope.rs` — Scope Enforcement**
- `Scope` enum: `Read`, `Write`, `Admin` (ordered by privilege)
- `RequireScope(Scope)`: `FromRequestParts` extractor, reads `AuthenticatedCaller` from extensions
- Returns 403 `ProblemDetail` if caller's scopes are insufficient
- DELETE/POST endpoints require `Write`; kill-agent requires `Admin`

**`auth/mod.rs` — AuthenticatedCaller**
- `AuthenticatedCaller { key_id: String, scopes: Vec<Scope> }`
- `FromRequestParts` impl with full validation pipeline
- `AuthError` enum: `MissingHeader`, `InvalidToken`, `ExpiredToken`, `RateLimited(u64)`, `InsufficientScope`
- `AuthError` implements `IntoResponse` via `ProblemDetail`

**`routes/auth.rs` — Token Endpoint**
- `POST /api/v1/auth/token`: accepts API key in Authorization header, issues JWT
- Optional request body: `{ scope: ["read"] }` to request subset of key's scopes
- Response: `{ token, expires_at, scopes }`

### Integration with Existing Scaffold

- `AppState` gains: `auth_config`, `key_store`, `rate_limiter`, `jwt_signer`, `jwt_verifier` (all `Arc`-wrapped)
- `config.rs`: `ApiConfig` gains `auth: AuthConfig` field
- `routes/mod.rs`: adds `auth` module, nests `POST /auth/token` under v1
- `server.rs`: `build_app` validates `AuthConfig` at startup (fail-closed)
- `middleware/mod.rs`: comment updated to reflect auth is extractor-based, not a layer

### Error Responses

All auth errors use existing `ProblemDetail` (RFC 7807):

| Status | Title | When |
|--------|-------|------|
| 401 | Unauthorized | Missing/invalid/expired token |
| 403 | Forbidden | Valid token but insufficient scope |
| 429 | Too Many Requests | Rate limit exceeded (includes `Retry-After` header) |

### Testing Strategy

**Unit tests** (per module):
- `api_key`: format validation, generate roundtrip, store load/validate
- `jwt`: sign/verify roundtrip, expired token rejection, scope preservation
- `rate_limit`: token consumption, refill timing, stale cleanup
- `scope`: ordering, comparison, `RequireScope` logic
- `config`: env var parsing, validation rules, fail-closed behavior

**Integration tests** (with `axum-test`):
- Valid API key → 200
- Invalid API key → 401
- Valid JWT → 200
- Expired JWT → 401
- Rate limit exceeded → 429 with `Retry-After`
- Scope enforcement: read-only key POST → 403
- `AA_AUTH=off` → 200 without credentials
- Token endpoint: API key → JWT issuance

### New Dependencies

- `jsonwebtoken`: JWT signing/verification (HMAC-SHA256)
- `argon2`: password hashing for API key storage
- `rand`: cryptographic random key generation
- `dashmap`: already in workspace (used by aa-gateway)

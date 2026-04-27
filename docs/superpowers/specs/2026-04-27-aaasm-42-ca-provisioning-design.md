# AAASM-42: CA Certificate Provisioning Design

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the CA certificate provisioning flow in `aa-proxy` — generating and persisting a local EC P-256 CA key pair on first run, signing per-domain leaf certs, and managing the CA's trust status in the macOS System Keychain.

**Architecture:** `ca.rs` is platform-agnostic and owns key generation, disk persistence, and cert signing. A new `keychain.rs` is macOS-only (`#[cfg(target_os = "macos")]`) and owns all `security` CLI invocations. `CaStore` delegates trust store operations to `keychain.rs` through three methods: `install`, `uninstall`, `is_installed`. `CertCache::get_or_insert` ties the cache to `CaStore::sign_cert` for on-demand per-domain cert generation.

**Tech Stack:** `rcgen 0.13` (key + cert generation), `std::process::Command` (macOS `security` CLI), `std::fs` (file I/O + permissions), `lru 0.16` (cert cache).

---

## Component Responsibilities

### `aa-proxy/src/error.rs`

Add a `Keychain(String)` variant to `ProxyError` for failures from the `security` CLI (non-zero exit, unexpected output). Keeps keychain errors distinct from I/O and cert-gen errors.

### `aa-proxy/src/tls/keychain.rs` (new, macOS-only)

Three package-private functions, all gated with `#[cfg(target_os = "macos")]`:

| Function | CLI invoked | Purpose |
|---|---|---|
| `add_trusted_cert(cert_path: &Path)` | `security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain <cert>` | Install CA cert into System Keychain with full trust |
| `remove_trusted_cert(cert_path: &Path)` | `security remove-trusted-cert -d <cert>` | Remove CA cert from System Keychain |
| `is_cert_trusted(subject: &str)` | `security find-certificate -c <subject> -a /Library/Keychains/System.keychain` | Return `true` if a cert with the given CN is found |

All three check the exit code and map failures to `ProxyError::Keychain`. The `security` CLI triggers macOS's native privilege prompt automatically when the System Keychain requires elevation — no explicit `osascript` needed.

### `aa-proxy/src/tls/ca.rs`

`CaStore` gains a `ca_dir: PathBuf` field and the following concrete implementations:

**`load_or_create(ca_dir: &Path) -> Result<Self, ProxyError>`**

1. If `ca_dir/ca-cert.pem` and `ca_dir/ca-key.pem` both exist → read and return.
2. Otherwise: generate EC P-256 (`PKCS_ECDSA_P256_SHA256`) CA key pair with `rcgen`, set validity to 10 years, set `is_ca = IsCa::Ca(BasicConstraints::Unconstrained)`, set `key_usages = [KeyCertSign, CrlSign]`, self-sign.
3. Create `ca_dir` with `fs::create_dir_all`.
4. Write `ca-cert.pem` (world-readable) and `ca-key.pem` (`chmod 600`, owner-only).
5. Return the loaded `CaStore`.

After `load_or_create` returns, `aa_proxy::run()` calls `is_installed()` and, if `false`, calls `install()`. This keeps `load_or_create` focused on disk state and `install` focused on OS trust state — two distinct concerns with a clear call sequence in `run()`.

**`sign_cert(&self, domain: &str) -> Result<CertifiedKey, ProxyError>`**

Generate a fresh EC P-256 leaf cert for `domain`: SAN = `[domain]`, validity = 1 year, signed by the CA key. Return DER-encoded cert + key as `CertifiedKey`.

**`install(&self) -> Result<(), ProxyError>`**

Call `keychain::add_trusted_cert(&self.ca_dir.join("ca-cert.pem"))`.

**`uninstall(&self) -> Result<(), ProxyError>`**

Call `keychain::remove_trusted_cert(&self.ca_dir.join("ca-cert.pem"))`. Then `fs::remove_dir_all(&self.ca_dir)`.

**`is_installed(&self) -> Result<bool, ProxyError>`**

Call `keychain::is_cert_trusted("Agent Assembly CA")`.

### `aa-proxy/src/tls/cert.rs`

**`get_or_insert(&self, domain: &str, ca: &CaStore) -> Result<Arc<CertifiedKey>, ProxyError>`**

Lock the `Mutex`, look up `domain` in the `LruCache`. On hit: clone and return the `Arc`. On miss: call `ca.sign_cert(domain)`, wrap in `Arc`, insert into the cache, return.

---

## Data Flow

### First proxy start (cold)

```
aa_proxy::run(config)
  → CaStore::load_or_create(&config.ca_dir)
      → ca-cert.pem missing → generate EC P-256 CA
      → write ca-cert.pem + ca-key.pem (chmod 600)
      → return CaStore
  → CaStore::is_installed()
      → security find-certificate → not found
  → CaStore::install()
      → security add-trusted-cert → macOS prompts for auth
  → ProxyServer::new(config, ca) → ready
```

### Per-domain TLS interception (cache miss)

```
client CONNECT api.openai.com:443
  → CertCache::get_or_insert("api.openai.com", &ca)
      → LRU miss
      → ca.sign_cert("api.openai.com")
          → rcgen generates EC P-256 leaf cert, 1-year, signed by CA
      → Arc<CertifiedKey> inserted into LRU, returned
  → TLS handshake with dynamic cert proceeds
```

### Subsequent connections (cache hit)

```
client CONNECT api.openai.com:443
  → CertCache::get_or_insert("api.openai.com", &ca)
      → LRU hit → return Arc clone (zero signing cost)
```

---

## Error Handling

| Scenario | Error |
|---|---|
| `ca_dir` not writable | `ProxyError::Io` (from `fs::create_dir_all`) |
| `rcgen` key/cert generation fails | `ProxyError::CertGen(e.to_string())` |
| `chmod 600` fails | `ProxyError::Io` |
| `security` CLI not found (non-macOS / CI) | `ProxyError::Keychain("security CLI not found")` |
| `security` exits non-zero | `ProxyError::Keychain(stderr)` |
| CA already installed (is_installed = true) | `install()` is a no-op — skip CLI call |

`load_or_create` is called at startup; any error is fatal and propagated via `anyhow::Result` in `main.rs`. Keychain failures are non-fatal if CA was previously installed and `is_installed()` returns true.

---

## Testing Strategy

### Unit tests (no macOS required, no sudo)

- **`ca.rs`**: `load_or_create` with a `tempdir` — verify files are created, key file has mode `0o600`, reloading same dir returns without regenerating, cert CN is "Agent Assembly CA".
- **`cert.rs`**: `get_or_insert` with a real `CaStore` from `tempdir` — verify first call for a domain signs a cert, second call for the same domain returns the same `Arc` pointer (cache hit).

### Integration tests (`#[cfg(target_os = "macos")]` + `#[ignore]`)

- `install()` → `is_installed()` returns `true`.
- `uninstall()` → `is_installed()` returns `false` → `ca_dir` no longer exists.
- Python `requests` GET to a test HTTPS server using the dynamic cert succeeds after installation.

Integration tests are marked `#[ignore]` (require sudo and a real macOS keychain) and run only in the CI job that has keychain access, or manually with `cargo test -- --ignored`.

---

## Files Changed

| File | Action |
|---|---|
| `aa-proxy/src/error.rs` | Modify — add `Keychain(String)` variant |
| `aa-proxy/src/tls/keychain.rs` | Create — macOS `security` CLI wrappers |
| `aa-proxy/src/tls/ca.rs` | Modify — implement all methods, add `ca_dir` field |
| `aa-proxy/src/tls/cert.rs` | Modify — implement `get_or_insert` |
| `aa-proxy/src/tls/mod.rs` | Modify — add `mod keychain` |
| `aa-proxy/Cargo.toml` | No change — `rcgen 0.13` already present |

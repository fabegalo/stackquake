# Security

## Reporting a vulnerability

**Do not open a public issue.** Use this repository's [private security advisory](https://github.com/fabegalo/stackquake/security/advisories/new). We reply within 7 days and publish the fix with credit, if you want it.

## Supported versions

Only the latest release receives security fixes.

## Threat model

Stackquake handles Google credentials with access to Play Console and Firebase, and crash reports that may contain end-user data. The goal is that **neither a UI bug nor a malicious crash report** can leak credentials or data, and that the app can never modify anything in Firebase.

### Controls

| Area | Control | Where |
|---|---|---|
| Sign-in | OAuth 2.0 for native apps (RFC 8252): system browser, loopback on `127.0.0.1` with a random port, **PKCE S256**, `state` compared in constant time, 5-minute timeout | `src-tauri/src/auth.rs` |
| Scopes | `playdeveloperreporting` (read) + `firebase` (the Crashlytics API has no read-only scope) + `openid`/`email` to separate data per account. Unchecked scopes on the consent screen are detected | `auth.rs` |
| Read-only Firebase | Despite the `firebase` scope, the core **only allows GET** to `firebase.googleapis.com` and `firebasecrashlytics.googleapis.com`: no change in Firebase is possible | `http.rs` |
| Secrets | Refresh token and database keys **only in the OS keychain**; the access token only lives in memory and is zeroed on drop (`zeroize`) | `secrets.rs` |
| OAuth client | Embedded at build time (git-ignored local file or CI secret). For installed apps the `client_secret` is not confidential (RFC 8252 §8.5); protection comes from PKCE + loopback + `state` | `build.rs`, `docs/DEVELOPMENT.md` |
| Network | A single HTTP client: **HTTPS only**, fixed host allowlist with **allowed methods per host**, **no redirects**, timeouts, response size limit, TLS via rustls | `http.rs` |
| Data at rest | SQLite + **SQLCipher (AES-256)**, **one database and one key per Google account**, 256-bit keys from the OS RNG; data folder with `0700` permissions (Unix); `secure_delete` | `db.rs` |
| Isolated UI | CSP `default-src 'self'`, **`connect-src` IPC only** (the UI cannot reach the internet), no `unsafe-inline`/`eval`, `freezePrototype` | `tauri.conf.json` |
| IPC | **Capabilities** with one permission per command; the **Isolation** pattern validates every call in an isolated iframe; no plugin exposed to the UI | `capabilities/`, `isolation/` |
| Input | Package names, project IDs, Firebase App IDs, issue/event IDs and free text are validated in Rust before becoming part of a URL | `validate.rs` |
| Untrusted content | Stack traces and logs are rendered **as text only** (never HTML). The OAuth callback page is static | Svelte, `auth.rs` |
| User KPIs | **Read-only** connection + SQLite authorizer that only allows SELECT/reads/functions (no ATTACH, PRAGMA or writes) + 5 s timeout + row limit | `kpi.rs`, `db.rs` |
| Supply chain | Exact versions and lockfiles, `cargo-deny` (RustSec, licenses, sources), `npm audit`, Actions pinned by SHA, Dependabot, SLSA provenance attestations on releases | `.github/`, `deny.toml` |
| Binary | `panic = "abort"`, LTO, stripped symbols; no sourcemaps in the bundle | `Cargo.toml`, `vite.config.ts` |

### Out of scope / known risks

- **Compromised machine**: anyone running code as your user can read the keychain.
- **Linux**: the webview (WebKitGTK) depends on distribution updates. The UI never loads remote content, which greatly reduces exposure.
- **Unsigned releases** until code-signing certificates are configured in CI. Verify provenance with `gh attestation verify`.

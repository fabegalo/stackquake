# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-10-27

First public release.

### Added

- Unified crash and ANR problems from Google Play Console (Play Developer Reporting API) and Firebase Crashlytics (Crashlytics API), with no BigQuery and no paid plan required.
- "Sign in with Google" with an embedded OAuth client; apps and Firebase projects are discovered automatically.
- Problem triage: status, assignee, notes and automatic regression detection by versionCode.
- Dashboard with user-perceived crash and ANR rates against Google Play's bad-behavior thresholds, crash-free users, daily trend and version health.
- KPIs as editable `.sql` files (summary, version health, daily trend, top problems, new issues and regressions, source coverage), with translatable names.
- On-demand problem details: sample stack traces from both sources and per-version distribution.
- Separate encrypted data per Google account.
- English and Portuguese (Brazil) UI, following the system language.
- Installers for Windows, macOS (Apple Silicon and Intel) and Linux.

### Security

- OAuth 2.0 for native apps with PKCE (S256), loopback redirect and `state` validation.
- Firebase is read-only by design: the core only allows GET requests to Firebase hosts.
- Tokens and database keys stored only in the OS keychain; local data encrypted with SQLCipher (AES-256).
- UI isolated from the network (CSP), Tauri capabilities and the Isolation pattern for IPC.
- User KPIs run on a read-only connection with a SQLite authorizer.
- Supply chain: pinned dependencies and Actions, `cargo-deny`, `npm audit` and build provenance attestations.

[Unreleased]: https://github.com/fabegalo/stackquake/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/fabegalo/stackquake/releases/tag/v0.1.0

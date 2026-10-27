# Stackquake

**Crashes and ANRs from Google Play Console and Firebase Crashlytics, unified in one desktop app** for Windows, macOS and Linux, with problem management and KPIs per app and version.

[![CI](https://github.com/fabegalo/stackquake/actions/workflows/ci.yml/badge.svg)](https://github.com/fabegalo/stackquake/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

[Português (Brasil)](README.pt-BR.md)

- **Just "Sign in with Google"**: no keys, JSON files or IDs to configure. Apps and Firebase projects are discovered automatically.
- **Works on the free Firebase Spark plan, no BigQuery**: uses the Play Developer Reporting API and the Crashlytics API, both free.
- **Read-only by design**: the core only allows read requests to Firebase, tokens never leave the OS keychain, and the local database is encrypted. See [SECURITY.md](SECURITY.md).
- **Local-first and private**: no server, no telemetry. Each Google account has its own encrypted database.
- **Hackable**: every KPI is a `.sql` file; every data source is an isolated module.
- **Multilingual**: English and Portuguese (Brazil), easy to extend.

## Features

| | |
|---|---|
| **Unified problems** | The same failure reported by Play and Crashlytics becomes a single problem (signature `kind\|Exception\|Class.method`). Impact is estimated **without double counting**. |
| **Triage** | Status (open, investigating, resolved, ignored), assignee and notes. **Automatic regression detection**: a problem fixed in versionCode N that shows up in a newer version is flagged as "Regressed". |
| **KPIs** | User-perceived crash and ANR rates against Google Play's bad-behavior thresholds (1.09% / 0.47%), crash-free users, version health, daily trend, top problems, new issues in the latest version, source coverage. |
| **History** | The APIs only expose the last 90 days; Stackquake stores data locally, so your history keeps growing. |
| **Multiple accounts** | Each Google account has its own data, separated and encrypted with its own key. |

## Architecture

```
UI (Svelte/TS)   ── renders data only; CSP with no network; filtered IPC (Isolation)
        │  typed commands, one permission each
Core (Rust)
  ├─ auth.rs     desktop OAuth: system browser + loopback 127.0.0.1 + PKCE S256 + state
  ├─ secrets.rs  OS keychain (Keychain / Credential Manager / Secret Service)
  ├─ http.rs     HTTPS to allowlisted hosts only, methods per host (Firebase: GET only)
  ├─ google/     play.rs (Reporting API) · firebase.rs (discovery) · crashlytics.rs
  ├─ sync.rs     incremental sync, failures isolated per source
  ├─ unify.rs    signature that links both sources (tested heuristic)
  ├─ db.rs       SQLite + SQLCipher (AES-256), one database and key per account
  └─ kpi.rs      .sql KPIs on a read-only connection with an authorizer
```

## Getting started

**Using it:** open the app and click **Sign in with Google**. See [docs/SETUP.md](docs/SETUP.md).

**Developing:**

```bash
npm install
npm run tauri dev
```

For sign-in to work in your own build, register the app with Google once: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Custom KPIs

Go to **KPIs → Open KPI folder** and create a `.sql` file:

```sql
-- name: ANRs per version
-- name.pt-BR: ANRs por versão
-- description: Crashlytics ANR events grouped by versionCode
SELECT build_version AS version_code, SUM(events) AS anrs
FROM crashlytics_version_daily
WHERE package_name = :package AND kind = 'anr'
  AND day >= date('now', '-' || :days || ' days')
GROUP BY build_version
ORDER BY CAST(build_version AS INTEGER) DESC
```

Click **reload**. A file with the same name as a built-in KPI (e.g. `02_version_health.sql`) replaces it. Tables are described in [`src-tauri/sql/schema.sql`](src-tauri/sql/schema.sql).

## Known limitations

- Play aggregates daily data in **America/Los_Angeles**; Crashlytics in **UTC**.
- Linking both sources is a heuristic. If an R8 mapping file was not uploaded to one source, the failure is obfuscated there and will not link. The "Source coverage" KPI helps spot this.
- The Crashlytics API (`v1alpha`) and the Play Reporting API expose up to 90 days.
- No auto-update yet (Tauri's signed updater is on the roadmap).

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md); commits follow [Conventional Commits](https://www.conventionalcommits.org/).

## License

[Apache-2.0](LICENSE).

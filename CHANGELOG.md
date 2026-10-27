# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Unified crash/ANR problems from Google Play Console (Play Developer Reporting API) and Firebase Crashlytics (Crashlytics API, no BigQuery).
- "Sign in with Google" with an embedded OAuth client, automatic discovery of apps and Firebase projects.
- Problem triage: status, assignee, notes and automatic regression detection.
- KPIs as editable `.sql` files: summary, version health, daily trend, top problems, new issues and regressions, source coverage.
- Encrypted local storage (SQLCipher), one database and key per Google account.
- English and Portuguese (Brazil) UI.

# Development and releases

## Running locally

Requirements: stable [Rust](https://rustup.rs), Node.js 20+ and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform. On Linux also `libdbus-1-dev` and a keychain service (GNOME Keyring or KWallet).

```bash
npm install
npm run tauri dev
```

## Google sign-in (one-time, by whoever publishes the build)

Google requires every app with "Sign in with Google" to be registered by its developer. This is done **once**; the registration is embedded in the app and users just click the button.

1. In the [Google Cloud Console](https://console.cloud.google.com), create a project for Stackquake (e.g. `stackquake-app`). No billing account needed.
2. In **APIs & Services → Library**, enable:
   - Google Play Developer Reporting API
   - Firebase Management API
   - Firebase Crashlytics API
3. In **Google Auth Platform**:
   - **Branding**: app name "Stackquake" and a support email. Don't upload a logo unless you plan to verify the app (a logo requires brand verification).
   - **Audience**: *External*, then **Publish app** ("In production"). In "Testing" mode Google expires sign-ins every 7 days.
   - **Data access**: add the scopes `.../auth/playdeveloperreporting`, `.../auth/firebase`, `openid` and `.../auth/userinfo.email`. All of them are classified as non-sensitive, so no Google verification is needed.
4. In **Clients → Create client → Desktop app**, download the JSON and save it as:

   ```
   src-tauri/google-oauth-client.json
   ```

   The file is git-ignored and never committed. `build.rs` reads it and embeds the client in the binary.

In CI, use the `STACKQUAKE_GOOGLE_CLIENT_ID` and `STACKQUAKE_GOOGLE_CLIENT_SECRET` secrets; the release workflow passes them through.

### About the embedded "client secret"

For installed apps, Google and [RFC 8252 §8.5](https://datatracker.ietf.org/doc/html/rfc8252#section-8.5) **do not consider the `client_secret` confidential**, since any binary can be inspected. Sign-in security comes from PKCE (S256), the `127.0.0.1`-only redirect and the `state` parameter, the same model used by tools such as `gcloud` and `rclone`. The value still stays out of the repository so forks use their own client.

### Why the `firebase` scope

The Crashlytics API only accepts `firebase` or `cloud-platform`; there is no read-only variant. Stackquake uses the narrower one and, in the core (`src-tauri/src/http.rs`), **blocks every non-GET request** to Firebase hosts. The app cannot change anything in Firebase, even if a bug tries to. For a server-side guarantee, sign in with an account that only has a *Viewer* role on the Firebase project.

## Translations

UI strings live in `src/lib/i18n/<locale>.ts`. To add a language, copy `en.ts`, translate it, type it as `Dictionary` and register it in `src/lib/i18n/index.svelte.ts`. The Rust core never produces user-facing text: errors are sent as codes (`errors.<code>` in the dictionaries). KPI names are translated in the `.sql` header with `-- name.<locale>:` and `-- description.<locale>:`.

## Releasing

Releases follow [Semantic Versioning](https://semver.org/) and are driven by [Conventional Commits](https://www.conventionalcommits.org/). Update `CHANGELOG.md`, bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`, then:

```bash
git tag v0.1.0 && git push --tags
```

The `release.yml` workflow builds Windows, macOS and Linux installers as a draft release, with a provenance attestation for each artifact.

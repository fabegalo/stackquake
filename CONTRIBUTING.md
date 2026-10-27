# Contributing

Thanks for helping! Stackquake is Apache-2.0 and every contribution is accepted under that license. Code, comments, docs and commit messages are in English; the UI is translated (see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md#translations)).

## Where to change things

| I want to… | File |
|---|---|
| Add or tune a KPI | `src-tauri/kpis/*.sql` (or the user KPI folder, no rebuild needed) |
| Change how Play and Crashlytics are linked | `src-tauri/src/unify.rs` (has tests) |
| Add a data source | a new module in `src-tauri/src/google/` + a call in `sync.rs` |
| Change a screen | `src/views/*.svelte` |
| Add or fix a translation | `src/lib/i18n/<locale>.ts` |
| Add an IPC command | `commands.rs` + `lib.rs` + **all three lists**: `build.rs`, `capabilities/default.json`, `isolation/index.js` |

## Security rules (required in every PR)

1. **No network access from the UI.** Every external call goes through `http.rs`. A new host goes into `ALLOWED` with its allowed methods and a justification in the PR.
2. **Firebase stays read-only.** Firebase hosts only accept GET; never add another method for them.
3. **No token or secret** leaves the Rust core, whether in a command result, a log or an error.
4. **Validate all command input** in `validate.rs`. Never interpolate user text into SQL or URLs.
5. **No HTML from data**: no `{@html}` in Svelte.
6. **No inline `style` attributes** (blocked by the CSP); use classes.
7. **No user-facing text in Rust**: return an error code and add it to every dictionary.
8. New dependencies need a justification, an exact version and a license allowed by `deny.toml`.

## Commit messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<optional scope>): <description>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Add `!` (or a `BREAKING CHANGE:` footer) for breaking changes. Examples:

```
feat(kpi): add crash-free sessions per version
fix(sync): keep Play data when Crashlytics fails
docs: explain the read-only Firebase guarantee
```

Pull request titles must follow the same format (checked by CI), since they become the squash-merge commit.

## Local checks

```bash
npm run check && npm run build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

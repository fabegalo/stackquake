# Privacy Policy

_Last updated: 2026-10-27_

Stackquake is an open-source desktop application. This policy explains what data it accesses, why, and where that data goes.

## Summary

- Stackquake has **no servers** and **no telemetry**. The maintainers never receive your data.
- Your Google data is accessed **directly from your computer** to Google's APIs and stored **only on your computer**, encrypted.
- Access is **read-only**: the app never modifies anything in your Google Play Console or Firebase projects.

## Data accessed and why

When you click "Sign in with Google", you grant Stackquake these permissions:

| Permission | Used for |
|---|---|
| `playdeveloperreporting` | Reading Android Vitals (crash and ANR rates, error issues and reports) of the apps your account can view in Google Play Console. |
| `firebase` | Finding the Firebase project of each app and reading Firebase Crashlytics issues, metrics and sample stack traces. Stackquake only sends read (GET) requests to Firebase; any other request is blocked by the app itself. |
| `openid`, `email` | Identifying which Google account is signed in, so each account's data is kept separate. Your email is shown in the app. |

Stackquake does not access any other Google data (such as Gmail, Drive, Contacts or Calendar).

## Where data is stored

- **On your computer only**: app lists, metrics, crash summaries, sample stack traces, and the statuses, assignees and notes you enter. They are stored in a local database encrypted with AES-256 (SQLCipher), one database per Google account.
- **Credentials**: the Google refresh token and the database encryption keys are stored only in your operating system's keychain (macOS Keychain, Windows Credential Manager or Linux Secret Service). They never leave your computer except to talk to Google.

## Sharing

Stackquake does **not** sell, share, transfer or disclose your data to anyone. Data only travels between your computer and Google's APIs, over HTTPS.

## Google API Services User Data Policy

Stackquake's use and transfer of information received from Google APIs adheres to the [Google API Services User Data Policy](https://developers.google.com/terms/api-services-user-data-policy), including the Limited Use requirements. Google user data is used only to provide the features described above, is not used for advertising, is not used to train AI or machine learning models, and is not read by humans.

## Retention and deletion

- **Sign out** (Settings → Sign out of Google) revokes Stackquake's access at Google and removes the token from your keychain. Local data is kept so it is available when you sign in again.
- **Delete this account's data** (Settings) permanently deletes the local database and its encryption key, then signs out.
- You can also revoke access at any time at [myaccount.google.com/permissions](https://myaccount.google.com/permissions), and uninstalling the app and deleting its data folder removes everything it stored.

## Children

Stackquake is a developer tool and is not directed to children.

## Changes

Changes to this policy are published in this file, with its history available in the [repository](https://github.com/fabegalo/stackquake/commits/main/PRIVACY.md).

## Contact

Questions about privacy: open an issue at [github.com/fabegalo/stackquake/issues](https://github.com/fabegalo/stackquake/issues) or email fabegalo@gmail.com. Security issues: see [SECURITY.md](SECURITY.md).

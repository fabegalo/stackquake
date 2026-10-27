# Using Stackquake

1. Open Stackquake and click **Sign in with Google**. Use the account that has access to your Play Console.
2. That's it. Your apps show up automatically, and each app's Firebase project is discovered automatically too.

No keys, JSON files, IDs or integrations to configure. It works on the free Firebase **Spark** plan and **does not use BigQuery**.

## Where the data comes from

| Source | What it provides | Period |
|---|---|---|
| **Play Console** (Android Vitals) | User-perceived crash and ANR rates, daily users, issues and reports | 90 days on the first sync |
| **Firebase Crashlytics** (Crashlytics API) | Issues, events and affected users per day, crash-free users per version, stack traces | Last 90 days (API limit) |

Stackquake stores everything locally, so your history keeps growing beyond 90 days.

## Multiple Google accounts

Each account has its own data, separated and encrypted with its own key. When you switch accounts you only see the signed-in account's apps. When you come back, that account's data (statuses, assignees, notes) is still there.

## Language

The UI follows the system language (English or Portuguese) and can be changed in **Settings → Language**.

## Account permissions

- **Play Console**: *View app information* (read-only) on the apps you want.
- **Firebase**: access to the app's Firebase project (any role that can view Crashlytics, such as *Viewer*).

## Troubleshooting

| Message | What to do |
|---|---|
| `No Firebase project with this app was found` | The account has no access to the app's Firebase project. Ask the project admin for access. |
| `No Crashlytics data for this app` | The Crashlytics SDK is not in the app, or there were no events yet. |
| `No permission in Firebase` | The account has no role in the Firebase project that can view Crashlytics. |
| `Permissions not granted` | A box was left unchecked on Google's consent screen. Sign in again and check all boxes. |
| `The Firebase Crashlytics API is not enabled` | Whoever publishes the build must enable it in their Google Cloud project (see [DEVELOPMENT.md](DEVELOPMENT.md)). |

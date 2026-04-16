# Vonk — Capacitor wrapper

Wraps `apps/web` in a native iOS + Android shell so Vonk can ship on
the App Store and Google Play without maintaining a second codebase.

## First-time setup

```bash
# from repo root
cd apps/mobile
npm install
npm run build          # builds web → copies to www/ → cap sync

# iOS (Xcode required)
npx cap add ios
npm run ios

# Android (Android Studio required)
npx cap add android
npm run android
```

## Ongoing dev

```bash
# Live-reload against the dev SvelteKit server on your LAN
CAPACITOR_LIVE_URL=http://192.168.1.x:5173 npm run build
npm run ios   # or android
```

## Privacy & permissions

- **Camera** — only opened when the user hits the camera FAB. Never
  background / silent.
- **Contacts** — gated behind the friends-import wizard. Reads the
  address book once, hashes (salted SHA-256) email + phone client-
  side, uploads *only the hashes* via `/api/invites/match-handles`.
  Plaintext never leaves the device.
- **Push notifications** — uses FCM (Android) / APNs (iOS) via
  `@capacitor/push-notifications`. Tokens are stored in
  `push_subscriptions` with `kind='apns'` or `kind='fcm'`.

## Build targets

| Platform | Min version  | Status       |
| -------- | ------------ | ------------ |
| iOS      | 15.0         | Scaffold only |
| Android  | 24 (7.0)     | Scaffold only |

Submitting to the stores is the user's responsibility — we ship the
open-source project, not the store listings.

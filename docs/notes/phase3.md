# Phase 3 — E2EE, mobile, friend import

This note catalogs what shipped in Phase 3, what's gated behind
credentials you still need to provision, and what's intentionally
deferred to Phase 4.

## What shipped

### Sessions & auth hardening

- **Refresh-token rotation** with OWASP-style reuse detection.
  `POST /api/auth/refresh` mints a brand-new session row, sets
  `rotated_to` on the old one, and invalidates the whole chain
  (recursive CTE) if a rotated token is ever presented a second
  time. See `packages/api/src/routes/auth.rs::refresh`.
- **48-hour IP-hash sweep**. `jobs::ip_sweep::spawn` kicks off a
  tokio background task on boot that nulls `sessions.ip_hash` rows
  older than 48 h every 15 min. CLAUDE.md §9 satisfied.
- **GitHub OAuth 2.0 + PKCE** — `GET /api/auth/login/github` +
  `/callback/github`. Auto-pulls primary+verified email from
  `/user/emails` when the user hides it on their profile. Gated on
  `GITHUB_CLIENT_ID` + `GITHUB_CLIENT_SECRET` env vars.
- **Apple Sign-in scaffold** — authorize redirect lives; token
  exchange deliberately left as TODO until a real `.p8` key is on
  hand (see *deferred* below).

### Friend import (ethical path)

- `POST /api/invites` — Postal SMTP via `lettre`. Dedupes per
  `(sender, recipient)`; records `sent_at`, `failed_at`,
  `failure_reason`.
- `GET /api/invites/sent` — audit log of outgoing invites.
- `POST /api/invites/match-handles` — case-insensitive ANY-array
  match across 6 platform columns + website. Returns Vonk users
  who *opted in to publish* that handle on their profile. We never
  scrape external sites.
- `PATCH /api/users/me` grew 7 handle fields + `public_key`. Empty
  string clears a handle.
- Frontend: `/invite` wizard (email form + handle-match form +
  outgoing-invite status list). `/settings` → "Externe profielen"
  section persists the handles.

### E2EE v1 (snaps)

- Migration 005 already added `messages.encryption_version`,
  `ephemeral_pubkey`, `nonce`. `users.public_key` (TEXT, base64url)
  ready for the X25519 long-term key.
- Server: `POST /api/snaps` accepts all-three-or-none (ephemeral
  pubkey + nonce + ciphertext). On accept: stores verbatim, flips
  `encryption_version=1`, `protocol_version='aes-gcm-x25519-v1'`.
  Legacy v0 plaintext snaps still work. `GET /api/snaps/:uuid/view`
  projects the envelope back to the viewer.
- Client: `$lib/e2ee.ts` generates an X25519 long-term keypair on
  first use, stores it in IndexedDB, PATCHes the pubkey to
  `/api/users/me`. `encryptFor(recipientPub, bytes)` returns the
  envelope; `decryptFrom(env)` unwraps it. Uses `@noble/curves` +
  `@noble/ciphers` (audited, tree-shakable).
- *Not yet wired*: the snap-compose route still sends plaintext
  (v0). Calling `encryptFor()` on the recipient's `public_key` +
  a 32-byte random key — then AES-decrypting the storage_key blob
  on view — is the next integration step. The backend is ready.

### Web Push

- `GET /api/push/vapid-public-key`, `POST /api/push/subscriptions`,
  `DELETE /api/push/subscriptions/:uuid`, `PATCH /api/push/preferences`.
- `push_subscriptions` table stores `(user, endpoint, p256dh, auth,
  notify_dm, notify_mention, notify_follow, notify_reply)`. Partial
  unique index on `(user, md5(endpoint))` prevents dupes.
- Frontend: `$lib/push.ts` + `/static/sw.js`. `/settings` has a push
  toggle that calls `subscribe()` / `unsubscribe()`.
- Dispatch (`crate::push::send`) is a stub that returns
  `push_dispatch_not_implemented` — it lights up when the first
  notification source is built (Phase 3.1).

### Mobile scaffold

- `apps/mobile/` — Capacitor 6 wrapper. `package.json` pins
  `@capacitor/core`, `/ios`, `/android`, `/camera`, `/contacts`,
  `/push-notifications`. `capacitor.config.ts` sets appId
  `social.vonk.app`. README explains the `build → sync → open ios/
  open android` flow.
- Platforms (ios/, android/) are added on first build on each dev
  machine via `npx cap add ios` / `cap add android` — those
  directories are gitignored by Capacitor defaults so we don't
  commit generated Xcode / Gradle scaffolding.

### Login UX

- `/login` fetches `/api/health` server-side and conditionally
  renders Google / GitHub / Apple buttons — only providers whose
  creds are loaded show up. `ProviderButton.svelte` styles GitHub
  and Apple to match `GoogleButton`.

## What you still need to provision

| What | How |
|---|---|
| GitHub OAuth app | github.com/settings/developers → New OAuth App. Homepage `https://vonk.social`, callback `https://vonk.social/api/auth/callback/github`. Paste client id/secret into `GITHUB_CLIENT_ID` / `GITHUB_CLIENT_SECRET`. |
| Apple developer account + .p8 | Developer → Certificates → Keys → Create a Sign in with Apple key. Download `.p8`. Stash contents in `APPLE_PRIVATE_KEY`, set `APPLE_CLIENT_ID` (service id), `APPLE_TEAM_ID`, `APPLE_KEY_ID`. Requires finishing the ES256-JWT client_secret signing — see *deferred* below. |
| SMTP creds | Already supplied: `post.wattify.be` / `info` / password. Set `SMTP_HOST=post.wattify.be`, `SMTP_PORT=587`, `SMTP_USER=info`, `SMTP_PASS=…`, `SMTP_FROM=noreply@vonk.social`. |
| VAPID keys | `npx web-push generate-vapid-keys` (one-off). Paste the pair into `VAPID_PUBLIC_KEY` / `VAPID_PRIVATE_KEY`. Keep `VAPID_SUBJECT=mailto:noreply@vonk.social`. |

## Deferred to Phase 4+

- **Apple Sign-in token exchange.** The authorize redirect is
  live; the callback needs an ES256-signed JWT as `client_secret`
  (re-mint every ≤6 months) and `id_token` verification against
  Apple's JWKS. `jsonwebtoken` supports ES256 — drop it in when
  the `.p8` key is on the server.
- **Push dispatch.** `crate::push::send` returns a placeholder; wire
  it from the first notification source (new-DM event) and it's
  done.
- **Contact hash import (Capacitor).** Contacts plugin reads the
  address book on-device; we SHA-256(email+salt) and SHA-256(phone
  in E.164 + salt) client-side, POST the digests to an endpoint
  that returns matched Vonk users. Staged: `user_contact_hashes`
  table + the plugin — the endpoint + wizard step are a
  follow-up.
- **Snap compose encryption.** Client library is ready
  (`$lib/e2ee.ts`). Integration into `/camera` + snap composer is
  the last 30 lines: get recipient `public_key` from the profile
  endpoint, `encryptFor` the storage_key, pass the envelope as
  form fields when posting `/api/snaps`.
- **MLS protocol for group DMs.** Phase 4 item; v1 AES-GCM+X25519
  covers 1:1 snap flows comprehensively.

## Verification

```bash
# Backend
cd packages/api
cargo clippy --all-targets -- -D warnings  # 0 warnings

# Frontend
cd apps/web
npm run check                              # svelte-check: 0 errors

# Smoke
curl -s https://vonk.social/api/health | jq
# → includes google_oauth_configured, github_oauth_configured,
#   apple_oauth_configured, smtp_configured, vapid_configured.
```

On vonk.social, only `google_oauth_configured: true` is expected
until you paste the rest of the credentials from the table above
into `/opt/vonk/.env`.

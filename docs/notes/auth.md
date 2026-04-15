# Auth — Phase 1 implementation notes

This document captures the decisions, wire protocol and setup steps for the
Google OAuth / JWT session flow shipped in Phase 1. Read after `CLAUDE.md` §
"Phase 1: Auth & Users".

---

## Token model

- **Access token** — JWT (HS256), 15-minute TTL. Claims:
  `sub` (user id, string-encoded i64), `sid` (session UUID), `username`,
  `iat`, `exp`, `iss=vonk`. Minted on callback and on `/api/auth/refresh`.
- **Refresh token** — opaque `sessions.id` UUID, 30-day TTL. Stored as an
  httpOnly cookie scoped to `/api/auth`. The server's `sessions` table is
  the source of truth; logging out deletes the row.
- Both tokens live in **httpOnly `SameSite=Lax`** cookies. `Secure` is on
  only in production (controlled by `ENVIRONMENT`). `SameSite=Lax` (not
  `Strict`) is required: the OAuth redirect from Google is a top-level
  cross-site navigation, and `Strict` would drop the cookie.

## Cookie names

| name           | content          | path        | max-age           |
|----------------|------------------|-------------|-------------------|
| `vonk_access`  | access JWT       | `/`         | `JWT_ACCESS_TTL_SECS` (900) |
| `vonk_refresh` | session UUID     | `/api/auth` | `REFRESH_TTL_SECS` (2592000) |

Scoping the refresh cookie to `/api/auth` means it is not sent on every
request — only on `POST /api/auth/refresh` and `POST /api/auth/logout`.

## Privacy

- IPs are never stored raw. `sessions.ip_hash` is
  `sha256(ip || salt || today_date)` truncated to 16 bytes (`IP_HASH_SALT`
  from env). The day rotation lets a future sweeper delete rows older than
  48h per `CLAUDE.md` §9 without re-identifying anyone from kept rows.
- EXIF is stripped from avatar uploads: decode to RGBA, centre-crop, resize,
  re-encode as WebP via the `webp` crate. The raw bytes never round-trip.
- We do not persist Google's access/refresh tokens in Phase 1. The
  `user_auth_providers` table has columns for them, kept null for now.

## OAuth flow (Google)

```
 browser                   SvelteKit (:5173)       API (:8080)        Google
  │                               │                  │                 │
  │ click "Sign in with Google"   │                  │                 │
  ├──────────────────────────────►│                  │                 │
  │                               │  GET /api/...google (proxied)      │
  │                               ├─────────────────►│                 │
  │                               │                  │  store PKCE     │
  │                               │                  │  verifier in    │
  │                               │                  │  Valkey (TTL    │
  │                               │                  │  10 min)        │
  │                               │  303 → Google    │                 │
  │                               │◄─────────────────┤                 │
  │  303 → accounts.google.com… ◄─┤                  │                 │
  │◄───────────────────────────── │                  │                 │
  │                                                                    │
  │  consent, then GET /api/auth/callback/google?code=…&state=…        │
  ├───────────────────────────────────────────────────────────────────►│
  │                               ├─────────────────►│                 │
  │                               │                  │  GETDEL state   │
  │                               │                  │  POST /token    │
  │                               │                  ├────────────────►│
  │                               │                  │ ◄───────────────┤
  │                               │                  │  GET /userinfo  │
  │                               │                  ├────────────────►│
  │                               │                  │ ◄───────────────┤
  │                               │                  │  upsert user,   │
  │                               │                  │  insert session,│
  │                               │                  │  mint JWT,      │
  │                               │                  │  set cookies    │
  │                               │  303 → /onboarding/username        │
  │                               │◄─────────────────┤                 │
  │◄───────────────────────────── │                  │                 │
```

## Google Cloud Console setup

1. **OAuth consent screen** → External → project name "Vonk".
2. **Credentials** → Create credentials → OAuth 2.0 Client ID → Web
   application, name "Vonk Social".
3. **Authorised JavaScript origins:**
   - `http://localhost:5173`
   - `https://dev.vonk.social`
4. **Authorised redirect URIs:**
   - `http://localhost:5173/api/auth/callback/google`
   - `https://dev.vonk.social/api/auth/callback/google`
5. Download the JSON, copy `client_id` + `client_secret` into `.env`.
6. While the consent screen is in "Testing" mode, add your Google account
   under **Test users** or sign-in will fail with `access_denied`.

## Local development

```
docker compose -f docker-compose.dev.yml up -d
cp .env.example .env && edit .env            # generate JWT_SECRET + IP_HASH_SALT
( cd packages/api && cargo run )             # API on :8080
( cd apps/web    && npm run dev )            # SvelteKit on :5173 with /api proxy
```

Visit http://localhost:5173. The Vite dev server proxies `/api/*` to
`http://localhost:8080`, so the browser sees one origin and cookies
behave naturally.

## Why we dropped the `oauth2` crate

The `oauth2` crate (v4.4) defaults to bundling `reqwest 0.11` via its
`reqwest` feature. The rest of the workspace is on `reqwest 0.12`.
Rather than manage two copies of hyper / rustls / tokio-util in the
binary, we implement the five-line Google flow ourselves (see
`src/auth/oauth_google.rs`). If we later add Apple + GitHub, we may
revisit — or keep the pattern, since it's a small footprint.

## Out of scope for Phase 1 (intentional)

- Apple / GitHub OAuth (next up, see Phase 2 in `CLAUDE.md`).
- Refresh-token rotation + reuse detection.
- 2FA (`user_2fa` table exists, no routes yet).
- Cron sweep of `sessions` rows with stale `ip_hash` (48h rule).
- Device management UI ("sessions" list + "sign out everywhere").
- `tower_governor` HTTP-layer rate limits — we rely on nginx for now
  and re-add in Phase 2 once Valkey-backed per-user limits are needed.

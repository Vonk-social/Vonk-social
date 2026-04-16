<p align="center">
  <img src="apps/web/static/icons/vonk-icon.svg" alt="Vonk" width="96" />
</p>

<h1 align="center">Vonk</h1>

<p align="center">
  <strong>Social media, maar dan voor mensen ✌️</strong><br />
  An open-source social platform where you are not the product.
</p>

<p align="center">
  <a href="https://vonk.openview.be">Staging (alpha)</a> ·
  <a href="#roadmap">Roadmap</a> ·
  <a href="#contributing">Contribute</a> ·
  <a href="https://github.com/sponsors/vonk-social">Donate</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/status-alpha-orange" alt="Alpha" />
  <img src="https://img.shields.io/badge/license-AGPL--3.0-blue" alt="License" />
  <img src="https://img.shields.io/badge/made_in-Europe_🇪🇺-green" alt="Made in Europe" />
  <img src="https://img.shields.io/badge/ads-never-red" alt="No Ads" />
</p>

---

## What is Vonk?

Vonk (Dutch: *spark*) is a social platform that aims to combine the useful
parts of Facebook, Instagram, Twitter and Snapchat — without the
exploitation. It's **alpha software** right now; the core loop works
end-to-end but many features are still under construction. See
[Roadmap](#roadmap) for exactly where things stand.

**Three non-negotiable rules:**

1. **No advertisements.** Ever. Nowhere. No "promoted posts", no "sponsored content".
2. **No data sales.** Your data never leaves the platform. No third parties, no analytics partnerships.
3. **No algorithmic manipulation.** The feed is strictly reverse-chronological. The only suggestions are *people you may know* based on mutual connections — never content-based.

The full list of architectural guardrails is in [CLAUDE.md §Non-Negotiable Rules](CLAUDE.md).

## How is Vonk funded?

Vonk runs on donations. That's it.

- [GitHub Sponsors](https://github.com/sponsors/vonk-social)
- Optional in-app donation (€1/€3/€5) — not built yet

**Every euro will be publicly tracked** at `/open` (endpoint scaffolded,
live data coming with Phase 4). Hosting costs are covered first.
Everything left over is **donated annually to charities** focused on
world peace and health. The community votes on which organisations
receive funding.

## Live staging

Alpha-testable at any of:

- **https://vonk.social** (primary domain)
- **https://www.vonk.social**
- **https://vonk.openview.be** (original staging host)

All three serve the same backend. Google sign-in works on each; OAuth
redirect_uri is built dynamically from the `Host` header so a login
started on `vonk.social` returns to `vonk.social`, not to whichever
domain happens to be pinned in the server env.

Seeded with a handful of placeholder accounts so the feed and discover
pages have content. Expect breaking changes; the database may be wiped
between phases.

## Feature matrix

Legend: ✅ shipped · 🟡 backend ready, frontend pending · 🧪 alpha · 📋 planned · ⏸ later phase

### Phase 1 — Auth & Users ✅

| Feature | Status |
|---|---|
| Google OAuth 2.0 / OIDC sign-in | ✅ |
| JWT access (15 min) + opaque session refresh (30 d) in httpOnly cookies | ✅ |
| User profiles (display name, bio, location, avatar, locale) | ✅ |
| Onboarding wizard (username → avatar → friends) | ✅ |
| Avatar upload (EXIF-strip → 3 WebP variants → MinIO) | ✅ |
| Apple sign-in | 📋 |
| GitHub sign-in | 📋 |
| itsme / EU eID sign-in | ⏸ |

### Phase 2 — Posts, feed, social graph 🧪

| Feature | Status |
|---|---|
| Text + image posts (up to 4 images per post) | ✅ |
| Chronological feed with cursor pagination | ✅ |
| Post visibility: public / followers / mentioned | ✅ |
| @mention autocomplete in composer | ✅ |
| #hashtag autocomplete (90-day corpus scan) | ✅ |
| Likes (private count — only author sees it, per privacy rule #7) | ✅ |
| Inline replies (no page navigation), one-shot composer | ✅ |
| Public profiles with follow button + followers/following lists | ✅ |
| Follow system (public + pending approval for private accounts) | ✅ |
| Stories (24h, tray view, viewer with keyboard / tap-to-skip) | ✅ |
| Snaps (view-once / view-24h ephemeral 1-to-1 media) | ✅ |
| User search + "people you may know" (mutual-follows) | ✅ |
| Bookmarks (private, server-side + `/bookmarks` page) | ✅ |
| Reposts + quote-reposts (embedded original post card) | ✅ |
| Pinned posts on profile (max 3, sorted pinned-first) | ✅ |
| Author counters inline (likes/replies/bookmarks/reposts — no impressions) | ✅ |
| Story viewer list (author-only 👁 button) | ✅ |
| Share button (web Share API + clipboard fallback) | ✅ |
| Post menu (pin/unpin + delete on your own posts) | ✅ |

### Phase 3 — E2EE, mobile, friend import 🧪

| Feature | Status |
|---|---|
| End-to-end-encrypted snaps (AES-256-GCM + X25519 ECDH v1 envelope) | ✅ |
| Long-term X25519 keypair in IndexedDB + `public_key` on profile | ✅ |
| Refresh-token rotation + reuse detection (chain revocation) | ✅ |
| 48h `sessions.ip_hash` sweep cron (tokio background task) | ✅ |
| GitHub OAuth 2.0 + PKCE sign-in (gated on env) | ✅ |
| Apple Sign-in scaffold (authorize-redirect only; callback needs ES256 JWT) | 🟡 |
| Postal SMTP integration + email invites (`/api/invites`) | ✅ |
| Handle-based friend discovery (`/api/invites/match-handles`, 6 platforms + website) | ✅ |
| `/invite` page — e-mail invites + handle-match UI | ✅ |
| `PATCH /api/users/me` accepts 7 handle fields + `public_key` | ✅ |
| Web Push subscriptions (`/api/push/*`) + service worker + VAPID | ✅ |
| `/settings` push toggle + per-device subscribe | ✅ |
| Login page conditionally renders Google/GitHub/Apple from `/api/health` | ✅ |
| Capacitor 6 iOS + Android scaffold (`apps/mobile/`) | ✅ |
| Native camera + contact-hash import via Capacitor plugins | 📋 |
| Push dispatch from notification sources (DM/mention/follow) | 📋 |
| Apple Sign-in token exchange (ES256 client_secret, id_token verification) | 📋 |
| Snap frontend actually calling `encryptFor()` on compose | 📋 |

### Phase 4+ — Content & growth ⏸

| Feature | Status |
|---|---|
| Public financial dashboard (`/api/open/*`) | 📋 |
| Waitlist endpoint | 📋 |
| Short video posts (ffmpeg transcode) | ⏸ |
| Snap Map (opt-in location) | ⏸ |
| Streaks, Memories, Bitmoji-style avatars | ⏸ |
| Groups | ⏸ |
| Events | ⏸ |
| ActivityPub federation | ⏸ |
| Admin moderation dashboard (`apps/admin/`) | ⏸ |

### Cross-cutting ✅

| Feature | Status |
|---|---|
| Dark + light mode (warm-dark palette, OS-aware) | ✅ |
| Instagram-style bottom nav (Home / Zoek / 📷 / Snaps / Profiel) | ✅ |
| 15 European languages on the landing (NL + EN native, others translated-for-review) | ✅ |
| Accept-Language auto-detect with cookie override | ✅ |
| Privacy-preserving IP hashing (rotating day salt) | ✅ |
| EXIF stripping on every upload | ✅ |
| Rate limiting (nginx-based currently; Valkey tower layer planned) | 🧪 |

## Privacy by design

- **EXIF stripping** on every upload — decode → re-encode path, never pass through
- **IP retention** — stored as `sha256(ip || salt || day)`; 48-hour sweep cron runs every 15 min
- **No tracking** — no cookies beyond auth, no fingerprinting, no analytics (Plausible, GA or otherwise)
- **Private like counts** — the JSON response literally does not contain `like_count` for non-authors (`#[serde(skip_serializing_if)]`), so the UI *cannot* leak it
- **E2EE DMs** — AES-256-GCM + X25519 ECDH v1 envelope shipped for snaps. Long-term keypair lives in IndexedDB; the server stores only ciphertext + ephemeral pubkey + nonce. Migration to MLS is a Phase 4 item; v1 is sufficient for 1:1 messages
- **Contact import is on-device** — handle-based friend discovery matches against opted-in public handles only. When the Capacitor contacts plugin lands, we hash (SHA-256 + shared salt) email + phone numbers *on the device* and upload only the hashes
- **Refresh-token reuse detection** — using a rotated refresh cookie twice invalidates the whole lineage (OWASP pattern) and forces re-login
- **Data export + real delete** — GDPR-compliant, planned for Phase 4
- **Open source** — AGPL-3.0, every commit is public on GitHub

## Tech stack

| Layer | Technology |
|---|---|
| Frontend | SvelteKit 5 + Svelte 5 runes, Tailwind 4, adapter-node |
| Backend | Rust 1.94 + Axum 0.8 + Tokio |
| Database | PostgreSQL 16 + SQLx (runtime-checked queries) |
| Cache / state | Valkey 8 (Redis-compatible, open-source fork) |
| Object storage | MinIO (S3-compatible) |
| Image pipeline | `image` crate + `webp` crate (EXIF-strip → Lanczos3 resize → WebP Q80) |
| Auth | Hand-rolled Google + GitHub + Apple OAuth / OIDC + PKCE, HS256 JWTs, refresh-token rotation |
| Email | Postal via `lettre` (SMTP STARTTLS) |
| Push | VAPID Web Push + service worker; APNs/FCM via Capacitor on mobile |
| Client crypto | `@noble/curves` (X25519) + `@noble/ciphers` (AES-256-GCM), keys in IndexedDB |
| Mobile shell | Capacitor 6 (iOS + Android) in `apps/mobile/` |
| Dev infra | Docker Compose (db + cache + storage + mailpit) |
| Prod infra (staging) | nginx + Let's Encrypt + systemd units on a Linux VM |

## Quick start (development)

```bash
git clone git@github.com:Vonk-social/Vonk-social.git
cd Vonk-social

# Start backing services (postgres, valkey, minio, mailpit)
docker compose -f docker-compose.dev.yml up -d

# Seed environment
cp .env.example .env
# Edit JWT_SECRET + IP_HASH_SALT (see comments in .env.example for
# openssl commands). Google OAuth creds are optional but required for
# the sign-in flow to work — see docs/notes/auth.md.

# Start the Rust API (migrations run automatically at boot)
( cd packages/api && cargo run )

# Start the SvelteKit frontend in a second terminal
( cd apps/web && npm install && npm run dev )
```

Open **http://localhost:5173**.

Optional: seed a plausible feed with 8 dummy users so `/discover` and
`/home` have content:

```bash
docker exec -i vonk-social-vonk-db-1 psql -U vonk vonk \
  < packages/db/seed/dev-users.sql
```

## Project structure

```
Vonk-social/
├── apps/
│   ├── web/                # SvelteKit 5 frontend (main UI)
│   ├── mobile/             # .keep — Capacitor, Phase 3
│   └── admin/              # .keep — moderation dashboard, Phase 4+
├── packages/
│   ├── api/                # Rust backend (Axum 0.8, SQLx 0.8)
│   │   └── src/
│   │       ├── auth/       # JWT, cookies, Google OIDC, IP hashing
│   │       ├── feed/       # cursor-paginated feed query
│   │       ├── media/      # shared image pipeline (avatar + posts + snaps)
│   │       ├── models/     # row + response types
│   │       └── routes/     # auth / users / posts / feed / follows / snaps / media
│   ├── db/
│   │   ├── migrations/     # 001_initial.sql → 004_repost_pin_bookmarks.sql
│   │   └── seed/           # dev-only fixtures
│   ├── crypto/             # .keep — MLS E2EE library, Phase 3
│   ├── media/              # .keep — video pipeline, Phase 4
│   └── vonk-ui/            # .keep — shared components, later
├── infra/
│   ├── nginx/              # vonk-openview site + TLS template
│   └── scripts/            # deploy-dev.sh
├── docs/
│   ├── notes/auth.md       # wire diagram, Google Console walkthrough
│   ├── HOSTING.md
│   ├── FINANCES.md
│   └── PROJECT_PLAN.md
├── docker-compose.dev.yml
├── CLAUDE.md               # architectural guardrails + phase plan
└── README.md
```

## Self-hosting

The code ships AGPL-3.0 so you can run your own Vonk instance. The
staging box on `vonk.openview.be` is a good reference for the shape of
a production deployment:

- One Linux host, 8 GB RAM, 50 GB disk comfortably hosts it for a small community
- `docker compose -f docker-compose.dev.yml up -d` provides Postgres / Valkey / MinIO
- Rust API built with `cargo build --release` and run under systemd
- SvelteKit built with `npm run build` and served via `node build` under systemd
- nginx handles TLS + routes `/api/*` → API, `/media/*` → MinIO, everything else → SvelteKit
- Let's Encrypt for certs (`certbot --nginx -d <your-host>`)

See `infra/scripts/deploy-dev.sh` for the current staging deploy.
Self-hosted instances are standalone today; ActivityPub federation
would come later if there's demand.

## Contributing

Everyone is welcome to contribute. Read [CONTRIBUTING.md](CONTRIBUTING.md) first.

### Workflow

1. **Fork** the repository (or create a feature branch if you have write access)
2. **Branch** off `main` — one feature per branch, use descriptive names (`feat/story-replies`, `fix/avatar-upload-crash`)
3. **Commit** using [conventional commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `chore:`, `docs:`)
4. **Open a Pull Request** against `main` — describe what changed and why
5. **Get at least 1 approving review** from a maintainer
6. **Squash-merge** into `main` (this is the standard merge strategy)

### Branch protection

The `main` branch is protected:

- **Direct pushes to `main` are blocked** — all changes go through pull requests
- **At least 1 approving review** is required before merging
- **Branches must be up to date** with `main` before merging
- **Force pushes and branch deletion** are not allowed on `main`

Only maintainers can merge PRs. Repo admins retain emergency push access.

### Highest-value contributions right now

- Frontend for the backend-ready items (bookmark / repost / pin / author stats UI)
- Native-speaker review of the 13 translated landing locales (fr, de, es, it, pt, pl, sv, da, fi, el, ro, cs, uk) — NL + EN are native-quality, the rest need a look from a native eye
- Phase 3 E2EE (MLS protocol implementation in `packages/crypto/`)
- Bug reports + feature suggestions via GitHub Discussions
- Security issues — see [SECURITY.md](SECURITY.md)
- [Donate](https://github.com/sponsors/vonk-social) — covers hosting, anything over pays for native reviewers and designers

## Governance

Vonk is operated by **VZW Vonk**, a European non-profit association
(*vereniging zonder winstoogmerk* — a Belgian legal form). No
shareholders, no investors, no profit distribution.

Major features are proposed as RFCs (`docs/rfcs/`) and discussed
publicly. The community votes annually on where charity fund surplus
goes.

## Roadmap

We're shipping in numbered phases, each merged as its own set of PRs on
`main`. Current status (April 2026):

- **Phase 1 — Auth & Users** — ✅ merged (PR #1)
- **Phase 2 — Posts, feed, stories, snaps, follows** — ✅ merged (PRs #2–#8)
- **Phase 2.5 — Bookmarks / reposts / pins / author counters / story viewers** — ✅ merged (backend PR #9, frontend PR #16)
- **Landing & multilingual (15 EU languages)** — ✅ merged (PR #11)
- **Instagram-style bottom nav + dark mode + `/discover` + one-shot reply** — ✅ merged (PRs #8, #13–#14)
- **Staging on vonk.openview.be** — ✅ live
- **Primary domain vonk.social + www.vonk.social + host-aware OAuth** — ✅ merged (PR #18)
- **Phase 3 — MLS E2EE + Capacitor mobile + friends import wizard** — 📋 planned
- **Phase 4 — Public finances dashboard, short video, Snap Map, streaks** — ⏸ later

## License

[AGPL-3.0](LICENSE) — You may use, modify, and host Vonk freely. All
modifications you distribute or host must remain open source under the
same licence.

---

<p align="center">
  <sub>Made with ♥ in Europe 🇪🇺 — for everyone, everywhere.</sub>
</p>

# CLAUDE.md — Vonk Social Platform

> **Vonk** is an open-source, privacy-first social platform. No ads, no data sales, no algorithmic manipulation. Donation-funded, with surplus going to charities for world peace and health.

## Project Overview

**Repo:** `github.com/Vonk-social/Vonk-social`
**Stack:** Rust (Axum) backend + SvelteKit frontend + PostgreSQL + Valkey + MinIO
**License:** AGPL-3.0
**Status:** Pre-launch MVP phase

## Architecture

```
apps/web/              → SvelteKit 5 frontend (SSR + SPA hybrid)
apps/web/static/       → Static assets (coming-soon.html is the current live page)
apps/mobile/           → Capacitor wrapper (later)
apps/admin/            → Moderation dashboard (later)
packages/api/          → Rust backend (Axum 0.8, Tokio, SQLx 0.8)
packages/api/src/      → main.rs, config.rs, db.rs, routes/
packages/db/           → SQL migrations (SQLx migrate)
packages/db/migrations/→ 001_initial.sql = full schema
packages/crypto/       → E2EE library (MLS protocol, later)
packages/media/        → Image/video processing pipeline (later)
packages/vonk-ui/      → Shared UI components (later)
infra/docker/          → Production Docker Compose
infra/nginx/           → Reverse proxy config
```

## Non-Negotiable Rules

These are absolute. Never violate them, even if asked.

1. **NEVER add any form of tracking, analytics, telemetry, or fingerprinting.** No Google Analytics, no Plausible, no "privacy-friendly" alternatives. Nothing.
2. **NEVER add advertising code, sponsored content, or any revenue stream other than donations.**
3. **NEVER weaken E2EE.** The server must never be able to read message content.
4. **NEVER store or log user content for purposes other than serving it back to the user.**
5. **NEVER add algorithmic feed sorting.** Feed is strictly reverse-chronological.
6. **The only "suggestions" allowed are "people you may know" based on mutual follows.** No content-based recommendations, no behavioral profiling.
7. **NEVER expose like counts publicly.** Only the post author can see how many people liked their post.
8. **Always strip EXIF data from uploaded images before storage.** GPS, camera info, timestamps — all removed.
9. **IP addresses must be deleted after 48 hours.** Use hashed IPs in sessions, auto-clean via cron.

## Tech Stack Details

### Backend (packages/api/)

- **Language:** Rust (edition 2021, stable toolchain)
- **Framework:** Axum 0.8
- **Async runtime:** Tokio (full features)
- **Database:** SQLx 0.8 with PostgreSQL 16 (compile-time checked queries when possible)
- **Auth:** JWT (jsonwebtoken crate) + OAuth2/OIDC (oauth2 crate) for SSO
- **Object storage:** aws-sdk-s3 (connecting to MinIO, S3-compatible)
- **Serialization:** serde + serde_json
- **Validation:** validator crate with derive
- **Error handling:** thiserror for library errors, anyhow for application errors
- **Logging:** tracing + tracing-subscriber with env-filter

### Frontend (apps/web/)

- **Framework:** SvelteKit 5 with Svelte 5 (runes syntax)
- **Styling:** Tailwind CSS 4
- **Adapter:** @sveltejs/adapter-node (for Docker deployment)
- **Design system:** Warm, playful aesthetic. Nunito font. Cream/terracotta/amber palette.
  - Background: `#FFF8F0` (cream)
  - Primary: `#C2593A` (terracotta)
  - Accent: `#E5951B` (amber)
  - Success: `#7A9E7E` (sage)
  - Text: `#2D1F14`
  - Rounded corners everywhere (16–22px border-radius)
  - Friendly, human tone. Never corporate.

### Database

- **PostgreSQL 16** — start single-node, designed for horizontal scaling via Citus
- **Schema:** `packages/db/migrations/001_initial.sql` (20+ tables, fully documented)
- **Key design rule:** ALL user-scoped tables include `user_id` column for future Citus sharding. Every query on user-scoped data MUST filter by `user_id`.
- **Migrations:** SQLx migrations (`sqlx migrate run` from packages/api/)
- **Do NOT use Citus-incompatible features:** no cross-shard foreign keys on distributed tables, no queries that join distributed tables without colocation keys

### Infrastructure

- **Dev:** `docker-compose.dev.yml` in repo root — PostgreSQL, Valkey, MinIO, Mailpit
- **Ports (dev):** DB=5433, Redis=6380, MinIO=9000/9001, Mail=1025/8025, API=8080, Web=5173
- **Prod:** Docker containers behind nginx reverse proxy (config in `infra/nginx/vonk.conf`)
- **Secrets:** Never in code. Always via environment variables or Docker secrets.

## Coding Standards

### Rust

- Run `cargo fmt` before every commit
- Run `cargo clippy -- -D warnings` — zero warnings policy
- Use `#[derive(Debug, Serialize, Deserialize)]` on all API types
- Prefer `thiserror` for custom error types, implement `IntoResponse` for API errors
- Use extractors (`State`, `Json`, `Path`, `Query`) — never manual parsing
- Group routes in modules under `src/routes/`, each module exports a `router()` fn
- Write integration tests using reqwest against a test database
- Document public functions and types

### SvelteKit

- Use Svelte 5 runes syntax (`$state`, `$derived`, `$effect`)
- Components in PascalCase, files match component name
- Use `+page.svelte` / `+page.server.ts` / `+layout.svelte` conventions
- Tailwind for styling — no custom CSS unless absolutely necessary
- Accessible: proper ARIA labels, keyboard navigation, semantic HTML
- i18n-ready: all user-visible strings should be translatable (use `$lib/i18n/`)
- Default language: Dutch (nl), with English (en) as second language

### General

- Commit messages: conventional commits (`feat:`, `fix:`, `chore:`, `docs:`, `ci:`)
- One feature per branch, squash-merge to main
- Never commit `.env`, secrets, or API keys
- All API endpoints under `/api/` prefix
- Public financial endpoints under `/api/open/` — no auth required
- All other API endpoints require JWT auth (except `/api/auth/*`)

## Current MVP Tasks (Priority Order)

### Phase 1: Auth & Users (build this first)

1. **OIDC/OAuth2 login flow** — Google first, then Apple, GitHub
   - `POST /api/auth/login/google` → redirect to Google → callback → create/find user → issue JWT
   - `POST /api/auth/refresh` → refresh JWT
   - `POST /api/auth/logout` → invalidate session
   - Store provider info in `user_auth_providers` table
   - Create session in `sessions` table
   - JWT contains: `user_id`, `username`, `exp`

2. **User profile CRUD**
   - `GET /api/users/:username` → public profile
   - `PATCH /api/users/me` → update own profile
   - `GET /api/users/me` → current user
   - Avatar upload → strip EXIF → resize (3 variants) → store in MinIO

3. **Onboarding flow** (SvelteKit)
   - Step 1: Choose username (validate uniqueness via API)
   - Step 2: Upload avatar (optional, skip button)
   - Step 3: Find friends (search by name) or invite link

### Phase 2: Posts & Feed

4. **Post CRUD**
   - `POST /api/posts` → create post (text + optional images, max 4)
   - `GET /api/posts/:uuid` → single post
   - `PATCH /api/posts/:uuid` → edit own post (marks `is_edited`)
   - `DELETE /api/posts/:uuid` → soft delete own post
   - Image upload: strip EXIF, resize to thumb/medium/full, WebP conversion, store in MinIO

5. **Chronological feed**
   - `GET /api/feed` → posts from followed users, reverse chronological, paginated (cursor-based)
   - No algorithmic sorting. Ever.
   - Include replied-to post context when displaying replies

6. **Follow system**
   - `POST /api/users/:id/follow` → follow (or request if private account)
   - `DELETE /api/users/:id/follow` → unfollow
   - `GET /api/users/:id/followers` → list followers
   - `GET /api/users/:id/following` → list following
   - `POST /api/users/:id/follow/accept` → accept follow request (private accounts)

7. **Comments (replies)**
   - Comments are just posts with `reply_to_id` set
   - `GET /api/posts/:uuid/replies` → list replies, chronological
   - Increment `reply_count` on parent post

8. **Likes** (private — only author sees count)
   - `POST /api/posts/:uuid/like`
   - `DELETE /api/posts/:uuid/like`
   - Update `like_count` on post
   - Do NOT expose like count in public API responses — only include when requesting user is the post author

### Phase 3: Suggestions & Search

9. **People you may know**
   - `GET /api/suggestions/people` → based on `suggested_connections` materialized view
   - Only mutual follows. No content analysis. No behavioral data.
   - Refresh the materialized view nightly via cron/pg_cron

10. **User search**
    - `GET /api/users/search?q=` → search by username and display_name
    - No content search (privacy)
    - Use `ILIKE` or `pg_trgm` for fuzzy matching

### Phase 4: Open Finances

11. **Public financial dashboard**
    - `GET /api/open/summary` → current month income/expenses/net
    - `GET /api/open/transactions?year=&month=` → list all transactions
    - `GET /api/open/reserve` → current reserve amount and target
    - `GET /api/open/charity/current-year` → accumulated charity pot
    - `GET /api/open/charity/history` → past payouts
    - These are public endpoints, no auth required

12. **Waitlist** (pre-launch)
    - `POST /api/waitlist` → add email to waitlist
    - Validate email, deduplicate
    - Wire up the landing page signup form to this endpoint

## API Response Format

Use consistent JSON responses:

```json
// Success
{
  "data": { ... }
}

// Paginated
{
  "data": [ ... ],
  "cursor": "next_cursor_value",
  "has_more": true
}

// Error
{
  "error": {
    "code": "not_found",
    "message": "Post not found"
  }
}
```

HTTP status codes: 200 (ok), 201 (created), 204 (no content), 400 (validation), 401 (unauthorized), 403 (forbidden), 404 (not found), 429 (rate limited), 500 (server error).

## Rate Limiting

Use tower middleware + Valkey:
- Auth endpoints: 10 req/min per IP
- Write endpoints: 30 req/min per user
- Read endpoints: 120 req/min per user
- Public/open endpoints: 60 req/min per IP

## Testing

- **Backend:** `cargo test` in `packages/api/`
- Use a separate test database (`vonk_test`)
- Each test function gets a transaction that's rolled back
- Integration tests: spin up the API, hit endpoints with reqwest
- **Frontend:** Vitest for unit tests, Playwright for e2e (later)

## Development Workflow

```bash
# Start dependencies
docker compose -f docker-compose.dev.yml up -d

# Run backend
cd packages/api
cp ../../.env.example .env  # edit DATABASE_URL to localhost:5433
cargo run

# Run frontend (separate terminal)
cd apps/web
npm install
npm run dev

# Run migrations
cd packages/api
cargo sqlx migrate run

# Format & lint before commit
cargo fmt && cargo clippy -- -D warnings
```

## Verification

After implementing a feature, always verify:
1. `cargo fmt --check` passes
2. `cargo clippy -- -D warnings` passes
3. `cargo test` passes
4. Manual test: hit the endpoint with curl or the frontend
5. Check that no tracking, analytics, or privacy-violating code was introduced

## File Naming Conventions

- Rust: snake_case for files and functions, PascalCase for types
- Svelte: PascalCase for components (`UserProfile.svelte`), kebab-case for routes
- SQL migrations: `NNN_description.sql` (sequential numbering)
- Environment: `.env` (local), `.env.example` (committed, no secrets)

## Notes Directory

Maintain implementation notes in `docs/notes/` — update after completing each phase:
- `docs/notes/auth.md` — Auth implementation decisions
- `docs/notes/feed.md` — Feed query optimization notes
- `docs/notes/media.md` — Media pipeline decisions

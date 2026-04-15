# Contributing to Vonk

Thank you for considering contributing to Vonk! Every contribution helps build a social platform that respects its users.

## Code of Conduct

This project follows the [Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). Be kind, be respectful.

## How to Contribute

### Reporting Bugs

1. Check [existing issues](https://github.com/vonk-social/vonk/issues) first
2. Open a new issue with the **Bug Report** template
3. Include: steps to reproduce, expected vs actual behaviour, browser/device info

### Suggesting Features

1. Open a [Discussion](https://github.com/vonk-social/vonk/discussions) first
2. Explain the *problem* you're solving, not just the solution
3. Major features go through the RFC process (see below)

### Submitting Code

1. Fork the repo and create a branch from `main`
2. Follow the coding standards (Rust: `cargo fmt + clippy`, JS: `prettier + eslint`)
3. Write tests for new functionality
4. Keep PRs focused — one feature or fix per PR
5. Write a clear PR description explaining *what* and *why*

### Translations

Vonk aims to be available in all EU languages. Translation files live in `apps/web/src/lib/i18n/`. See [docs/translations.md](docs/translations.md) for the process.

## RFC Process

Features that change the user experience, data model, or architecture require an RFC:

1. Copy `docs/rfcs/000-template.md` to `docs/rfcs/NNN-your-feature.md`
2. Fill in: motivation, detailed design, alternatives considered, migration plan
3. Open a PR with the RFC
4. Discussion period: minimum 2 weeks
5. Core team reviews and decides

## Development Setup

```bash
# Prerequisites: Docker, Rust (1.75+), Node.js (20+)

git clone https://github.com/vonk-social/vonk.git
cd vonk

# Start dependencies
docker compose -f docker-compose.dev.yml up -d

# Backend
cd packages/api
cp .env.example .env
cargo sqlx migrate run
cargo run

# Frontend (separate terminal)
cd apps/web
npm install
npm run dev
```

## Architecture Decisions

Before making significant changes, please read:

- [docs/architecture.md](docs/architecture.md) — System overview
- [docs/privacy.md](docs/privacy.md) — Privacy requirements (non-negotiable)
- [docs/database.md](docs/database.md) — Schema design and Citus compatibility rules

### Key Rules

- **Never add tracking.** No analytics, no telemetry, no fingerprinting.
- **Never add ads.** Not even "tasteful" ones.
- **Never weaken E2EE.** No server-side message access.
- **Citus-compatible queries.** All queries on user-scoped tables must include `user_id` for future sharding.
- **Privacy by default.** New features should be private-by-default, opt-in for sharing.

## Security Issues

**Do not open a public issue for security vulnerabilities.** See [SECURITY.md](SECURITY.md).

## License

By contributing, you agree that your contributions will be licensed under AGPL-3.0.

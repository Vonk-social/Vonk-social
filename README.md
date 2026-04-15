<p align="center">
  <img src="docs/assets/vonk-logo.svg" alt="Vonk" width="80" />
</p>

<h1 align="center">Vonk</h1>

<p align="center">
  <strong>Sociaal, zonder de prijs.</strong><br />
  An open-source social platform where you are not the product.
</p>

<p align="center">
  <a href="https://vonk.social">Website</a> ·
  <a href="https://vonk.social/open">Open Finances</a> ·
  <a href="#contributing">Contribute</a> ·
  <a href="https://github.com/sponsors/vonk-social">Donate</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-blue" alt="License" />
  <img src="https://img.shields.io/badge/made_in-Europe_🇪🇺-green" alt="Made in Europe" />
  <img src="https://img.shields.io/badge/ads-never-red" alt="No Ads" />
</p>

---

## What is Vonk?

Vonk (Dutch: *spark*) is a social platform that combines the best of Facebook, Instagram, and TikTok — without the exploitation.

**Three non-negotiable rules:**

1. **No advertisements.** Ever. Nowhere. No "promoted posts", no "sponsored content".
2. **No data sales.** Your data never leaves the platform. No third parties, no analytics partnerships.
3. **No algorithmic manipulation.** The only suggestions are *people you may know* based on mutual connections. Your feed is chronological.

## How is Vonk funded?

Vonk runs on donations. That's it.

- [GitHub Sponsors](https://github.com/sponsors/vonk-social)
- [Buy Me a Coffee](https://buymeacoffee.com/vonk)
- Optional in-app donation (€1/€3/€5)

**Every euro is publicly tracked** at [vonk.social/open](https://vonk.social/open). Hosting costs are covered first. Everything left over is **donated annually to charities** focused on world peace and health. The community votes on which organisations receive funding.

## Features

| Feature | Status |
|---|---|
| Text + image posts | 🔨 Building |
| Chronological feed | 🔨 Building |
| SSO login (Google, Apple, GitHub) | 🔨 Building |
| E2EE direct messages | 📋 Planned |
| Stories (24h) | 📋 Planned |
| Short video (60s) | 📋 Planned |
| Groups | 📋 Planned |
| Events | 📋 Planned |
| PWA (installable mobile web app) | 📋 Planned |
| Native iOS + Android app | 📋 Planned |
| ActivityPub federation | 📋 Planned |
| itsme / EU eID login | 📋 Planned |

## Privacy by Design

- **E2EE messages**: The server cannot read your conversations (MLS protocol)
- **EXIF stripping**: All photo metadata (GPS, camera info) is removed before storage
- **IP retention**: IP addresses are deleted after 48 hours
- **No tracking**: No cookies, no fingerprinting, no analytics
- **Data export**: Full GDPR export in one click
- **Account deletion**: Real deletion, not a 30-day soft delete
- **Open source**: Every line of code is public. Verify it yourself.

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | SvelteKit (SSR + SPA) |
| Backend | Rust (Axum) |
| Database | PostgreSQL 16 (→ Citus for scaling) |
| Cache | Valkey (open-source Redis) |
| Object Storage | MinIO (S3-compatible) |
| E2EE | MLS protocol (libsignal) |
| CDN | Bunny.net (EU-based) |

## Quick Start (Development)

```bash
# Clone
git clone https://github.com/vonk-social/vonk.git
cd vonk

# Start services
docker compose -f docker-compose.dev.yml up -d

# Run database migrations
cd packages/api
cargo sqlx migrate run

# Start backend
cargo run

# Start frontend (new terminal)
cd apps/web
npm install
npm run dev
```

Open [http://localhost:5173](http://localhost:5173)

## Project Structure

```
vonk/
├── apps/
│   ├── web/             # SvelteKit frontend
│   ├── mobile/          # Capacitor wrapper (later)
│   └── admin/           # Moderation dashboard
├── packages/
│   ├── api/             # Rust backend (Axum)
│   ├── db/              # SQL migrations
│   ├── crypto/          # E2EE library
│   ├── media/           # Image/video processing
│   └── vonk-ui/         # Shared UI components
├── infra/
│   ├── docker/          # Docker Compose configs
│   ├── nginx/           # Reverse proxy configs
│   └── scripts/         # Deployment scripts
├── docs/
│   ├── architecture.md
│   ├── PRIVACY.md
│   └── api.md
├── docker-compose.yml
├── docker-compose.dev.yml
├── LICENSE              # AGPL-3.0
└── README.md
```

## Self-Hosting

Vonk is designed to run anywhere Docker runs:

```bash
# Generate secrets
mkdir -p secrets
openssl rand -base64 32 > secrets/jwt_secret.txt
openssl rand -base64 32 > secrets/db_password.txt
echo "minioadmin" > secrets/minio_user.txt
openssl rand -base64 32 > secrets/minio_password.txt

# Configure
cp .env.example .env
# Edit .env with your domain, OAuth credentials, etc.

# Launch
docker compose up -d
```

Note: self-hosted instances are standalone. To contribute hosting capacity to the main `vonk.social` platform, see [docs/node-hosting.md](docs/node-hosting.md).

## Contributing

We welcome contributions! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting a PR.

**Ways to help:**

- 🐛 Report bugs
- 💡 Suggest features (via GitHub Discussions)
- 🔧 Submit pull requests
- 🌍 Translate the interface
- 📖 Improve documentation
- 🔒 Report security issues (see [SECURITY.md](SECURITY.md))
- ♥ [Donate](https://github.com/sponsors/vonk-social)

## Governance

Vonk is operated by **VZW Vonk**, a Belgian non-profit (vereniging zonder winstoogmerk). There are no shareholders, no investors, and no profit distribution.

Major features are proposed as RFCs and discussed publicly. The community votes on charity fund distribution annually.

## License

[AGPL-3.0](LICENSE) — You may use, modify, and host Vonk freely. All modifications must remain open source.

---

<p align="center">
  <sub>Made with ♥ in Belgium. Vonk is for everyone.</sub>
</p>

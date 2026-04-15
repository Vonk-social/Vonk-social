# VONK — Open-Source Social Platform

**"Jouw vonk, jouw verhaal."**

> Vonk (Dutch: *spark*) — een Europees, open-source sociaal platform zonder advertenties, zonder dataverkoop, zonder manipulatie. Jij bent geen product.

---

## 1. Visie & Principes

Vonk combineert het beste van Facebook (sociaal netwerk, groepen), Instagram (visuele storytelling) en TikTok (korte video, discovery) in één platform — maar dan met drie ononderhandelbare regels:

1. **Geen advertenties.** Nooit. Nergens. Geen "promoted posts", geen "sponsored content".
2. **Geen dataverkoop.** Je gegevens verlaten nooit het platform. Geen derde partijen, geen analytics-partnerships.
3. **Geen algoritmische manipulatie.** De enige suggesties zijn *mensen die je misschien kent* (op basis van mutual connections, niet op basis van gedragsprofiling). Je feed is chronologisch.

### Waarom "Vonk"?

- Kort, memorabel, werkt in alle Europese talen
- Domein: `vonk.social` (of `vonk.community`)
- Betekenis: een vonk die verbindt, niet verslaaft
- Nederlandstalige roots, Europese identiteit

### Financiering

- **Donaties only**: GitHub Sponsors + Buy Me a Coffee
- Optioneel: vrijwillige maandelijkse bijdrage via de app (€1/€3/€5)
- Transparante kostenverantwoording op `vonk.social/open`
- Geen venture capital, geen investeerders met exit-strategie

---

## 2. Core Features (MVP → V1 → V2)

### MVP (Maand 1–3): Web-first

| Feature | Beschrijving |
|---|---|
| **Profiel** | Naam, bio, avatar, banner. Optioneel: locatie (stad-level, nooit exact). |
| **Posts** | Tekst + afbeeldingen. Markdown-support. Max 4 foto's per post. |
| **Feed** | Puur chronologisch. Alleen van mensen die je volgt. |
| **Volgen** | Asymmetrisch (als Twitter/Instagram). Optioneel wederzijds "vriend". |
| **Suggesties** | Alleen "Mensen die je misschien kent" op basis van mutual follows. |
| **Reacties** | Comments op posts. Geen likes-counter (alleen privé zichtbaar voor auteur). |
| **DMs** | End-to-end encrypted directe berichten (Signal Protocol / MLS). |
| **SSO Onboarding** | Login via OIDC: Google, Apple, GitHub, EU eID (later). Geen wachtwoord nodig. |
| **Zoeken** | Zoek mensen op naam/gebruikersnaam. Geen content-search (privacy). |
| **Doneer-knop** | Link naar GitHub Sponsors / Buy Me a Coffee in footer. |

### V1 (Maand 4–6): Visueel & Video

| Feature | Beschrijving |
|---|---|
| **Stories** | 24-uur verdwijnende foto/video-posts (à la Instagram Stories). |
| **Korte video** | Upload tot 60s video. Geen autoplay-feed — je kiest zelf wat je bekijkt. |
| **Groepen** | Besloten of open groepen met eigen feed. Moderatie door groepsleden. |
| **Events** | Simpele event-aanmaak met RSVP. Geen targeting, geen promotie. |
| **PWA** | Progressive Web App voor mobiel (installeerbaar via browser). |

### V2 (Maand 7–12): Volwassen Platform

| Feature | Beschrijving |
|---|---|
| **Native App** | iOS + Android (React Native of Flutter). |
| **Federatie** | ActivityPub-support zodat Vonk kan communiceren met Mastodon/Pixelfed/etc. |
| **E2EE Groepschat** | Versleutelde groepsgesprekken (MLS protocol). |
| **Data Export** | Volledige GDPR-export in één klik: al je data in een ZIP. |
| **Account Deletion** | Echte verwijdering — geen "soft delete" van 30 dagen. Direct weg. |
| **EU eID / itsme** | Login via Belgische/Europese digitale identiteit. |
| **Self-hosting** | Docker Compose one-liner voor eigen instantie. |

---

## 3. Technische Architectuur

### 3.1 Stack Keuze

```
┌─────────────────────────────────────────────────────┐
│                    FRONTEND                         │
│  Web: SvelteKit (SSR + SPA hybrid)                  │
│  Mobile: Capacitor (wrap de web-app) → later native │
│  UI: Tailwind CSS + eigen design system "Vonk UI"   │
└──────────────────────┬──────────────────────────────┘
                       │ REST + WebSocket
┌──────────────────────▼──────────────────────────────┐
│                   BACKEND API                        │
│  Rust (Axum) — performance + memory safety           │
│  OF: Go (Gin/Echo) — snellere development            │
│  GraphQL voor complexe queries, REST voor simpele    │
└──────────┬───────────┬───────────┬──────────────────┘
           │           │           │
    ┌──────▼──┐  ┌─────▼────┐ ┌───▼──────────┐
    │ Postgres │  │  Redis   │ │ Object Store │
    │ (Citus)  │  │ (Valkey) │ │ (MinIO/S3)   │
    └─────────┘  └──────────┘ └──────────────┘
```

**Waarom deze keuzes:**

- **SvelteKit**: Lichter dan React/Next.js, excellent SSR, kleine bundle size → snelle mobiele ervaring. Open-source friendly community.
- **Rust (Axum)**: Veilig, snel, laag geheugenverbruik. Ideaal voor een platform dat moet schalen zonder cloud-budget. Alternatief: Go als de community meer Go-developers heeft.
- **PostgreSQL + Citus**: Zie §3.2.
- **Valkey** (open-source Redis fork): Sessies, caching, rate limiting, real-time pub/sub voor notificaties.
- **MinIO**: S3-compatible object storage voor foto's/video's. Self-hosted, schaalbaar, multi-datacenter replicatie.

### 3.2 Database: Schaalbaar & Multi-Datacenter

**Kernvereiste**: de database moet makkelijk groeien van 1.000 naar 10.000.000 users, en uiteindelijk multi-datacenter werken.

#### Strategie: PostgreSQL + Citus (Distributed PostgreSQL)

```
┌─────────────────────────────────────────────────┐
│              Citus Coordinator                   │
│         (Query routing & planning)               │
└────────┬──────────┬──────────┬──────────────────┘
         │          │          │
   ┌─────▼───┐ ┌───▼─────┐ ┌─▼───────┐
   │ Worker 1 │ │ Worker 2│ │ Worker 3│  ← Horizontal sharding
   │ (DC-EU1) │ │ (DC-EU1)│ │ (DC-EU2)│
   └─────────┘ └─────────┘ └─────────┘
```

**Waarom Citus/PostgreSQL:**

- **Start simpel**: Begin met gewoon PostgreSQL. Eén server. Geen overhead.
- **Schaal horizontaal**: Wanneer nodig, voeg Citus toe. Distributed queries, automatische sharding op `user_id`.
- **Multi-datacenter**: Citus workers kunnen over datacenters verdeeld worden. Combineer met PostgreSQL streaming replication voor read-replicas per regio.
- **Open source**: Citus is volledig open source (Microsoft/AGPL → nu Apache 2.0).
- **Geen lock-in**: Het blijft gewoon PostgreSQL. Standaard tooling werkt.

#### Sharding Strategie

```sql
-- Alle user-gerelateerde tabellen sharden op user_id
SELECT create_distributed_table('users', 'id');
SELECT create_distributed_table('posts', 'user_id');
SELECT create_distributed_table('follows', 'follower_id');
SELECT create_distributed_table('messages', 'sender_id');

-- Reference tables (klein, op elke node)
SELECT create_reference_table('settings');
SELECT create_reference_table('countries');
```

#### Groeipad

| Fase | Users | Infra | Database |
|---|---|---|---|
| Alpha | 0–1K | 1 VPS (Hetzner) | Gewoon PostgreSQL |
| Beta | 1K–50K | 2–3 servers | PostgreSQL + read replica |
| Growth | 50K–500K | Cluster | Citus (3–5 workers) |
| Scale | 500K–5M | Multi-DC | Citus multi-DC + geo-routing |
| Mature | 5M+ | Federation | Meerdere Vonk-instanties via ActivityPub |

### 3.3 Media Pipeline

```
Upload → Virus scan (ClamAV) → EXIF strip → Resize/transcode → MinIO
                                    ↑
                            Privacy: alle metadata
                            wordt verwijderd vóór opslag
```

- **Foto's**: WebP conversie, 3 formaten (thumb/medium/full), max 10MB upload
- **Video's**: FFmpeg transcode naar H.264/VP9, max 60s, max 100MB upload
- **EXIF stripping**: Automatisch. GPS, camera-info, timestamps → weg.
- **CDN**: Bunny.net (EU-gebaseerd, GDPR-compliant, betaalbaar) of eigen Varnish-cache

### 3.4 Beveiliging & Privacy

| Laag | Maatregel |
|---|---|
| **Transport** | TLS 1.3 everywhere. HSTS. Certificate pinning in app. |
| **Auth** | OIDC/OAuth2 (SSO). TOTP/WebAuthn voor 2FA. Geen wachtwoorden opslaan als SSO. |
| **DMs** | End-to-end encrypted (Signal Protocol via libsignal of MLS). Server ziet niets. |
| **Data at rest** | Database encryption (LUKS). Encrypted backups. |
| **Metadata** | IP-adressen worden na 48h verwijderd. Geen tracking cookies. Geen fingerprinting. |
| **GDPR** | Privacy by design. DPO-contact op website. DPIA gedocumenteerd. Art. 17 (verwijdering) in één klik. |
| **Audit** | Publieke security audit (jaarlijks, betaald uit donaties). Bug bounty programma. |

---

## 4. Onboarding Flow

```
┌──────────────────────────────────────────────┐
│           vonk.social                        │
│                                              │
│   ┌─────────────────────────────────┐        │
│   │  Welkom bij Vonk                │        │
│   │  Jouw plek. Geen advertenties.  │        │
│   │  Geen algoritme. Gewoon mensen. │        │
│   │                                 │        │
│   │  [▶ Login met Google]           │        │
│   │  [▶ Login met Apple]            │        │
│   │  [▶ Login met GitHub]           │        │
│   │  [▶ Login met itsme]  (V2)      │        │
│   │                                 │        │
│   │  of maak een account met email  │        │
│   └─────────────────────────────────┘        │
│                                              │
│   Vonk is open source. Geen data verkoop.    │
│   Bekijk de broncode op GitHub ↗             │
└──────────────────────────────────────────────┘
```

**Na login (3 stappen, max 60 seconden):**

1. **Kies je gebruikersnaam** → `@dimitry@vonk.social`
2. **Upload een profielfoto** (optioneel, skip mogelijk)
3. **Vind je vrienden** → "Ken je iemand op Vonk?" (zoek op naam) of "Nodig iemand uit" (share-link)

Klaar. Geen verplichte interesses, geen "volg deze accounts", geen onboarding-funnel.

---

## 5. Repo-structuur & Open Source

```
github.com/vonk-social/
├── vonk/                    # Monorepo
│   ├── apps/
│   │   ├── web/             # SvelteKit frontend
│   │   ├── mobile/          # Capacitor wrapper
│   │   └── admin/           # Moderatie dashboard
│   ├── packages/
│   │   ├── api/             # Rust/Go backend
│   │   ├── db/              # Migraties (sqlx/diesel)
│   │   ├── crypto/          # E2EE library
│   │   ├── media/           # Media processing pipeline
│   │   └── vonk-ui/         # Shared UI components
│   ├── infra/
│   │   ├── docker/          # Docker Compose (dev + prod)
│   │   ├── terraform/       # IaC voor hosting
│   │   └── k8s/             # Kubernetes manifests (later)
│   ├── docs/
│   │   ├── architecture.md
│   │   ├── PRIVACY.md
│   │   ├── CONTRIBUTING.md
│   │   └── SECURITY.md
│   ├── LICENSE              # AGPL-3.0
│   └── README.md
├── vonk-spec/               # ActivityPub extensies
└── .github/
    ├── FUNDING.yml           # GitHub Sponsors + BMAC
    └── workflows/            # CI/CD
```

**Licentie: AGPL-3.0** — Iedereen mag het gebruiken, aanpassen, en hosten. Maar wijzigingen moeten ook open source blijven. Dit voorkomt dat Big Tech de code pakt en er een gesloten product van maakt.

---

## 6. Hosting & Kosten (Realistisch)

### Fase 1: Alpha (< 1.000 users)

| Component | Provider | Kosten/maand |
|---|---|---|
| App server | Hetzner CX31 (4 vCPU, 8GB) | €8 |
| Database | Zelfde server (PostgreSQL) | €0 |
| Object storage | Hetzner Storage Box 1TB | €4 |
| Domain | vonk.social | €2 |
| CDN | Bunny.net (1TB traffic) | €10 |
| Email | Resend (gratis tier) | €0 |
| **Totaal** | | **~€25/maand** |

### Fase 2: Beta (1.000–10.000 users)

| Component | Provider | Kosten/maand |
|---|---|---|
| App servers (2x) | Hetzner CX41 | €28 |
| Database | Hetzner CCX22 (dedicated) | €35 |
| Object storage | Hetzner 5TB | €18 |
| CDN | Bunny.net (10TB) | €40 |
| Backup | Borgbase | €10 |
| **Totaal** | | **~€130/maand** |

### Fase 3: Growth (10.000–100.000 users)

| Component | Provider | Kosten/maand |
|---|---|---|
| App cluster (4x) | Hetzner | €120 |
| DB cluster (Citus 3-node) | Hetzner | €150 |
| Object storage (50TB) | Hetzner/Backblaze | €100 |
| CDN | Bunny.net | €200 |
| Video transcoding | Hetzner GPU | €80 |
| Monitoring | Grafana Cloud (gratis) | €0 |
| **Totaal** | | **~€650/maand** |

**Break-even met donaties:**
- Fase 1: 5 donateurs à €5/maand
- Fase 2: 25 donateurs à €5/maand
- Fase 3: 130 donateurs à €5/maand (of 1.3% van 10K users)

---

## 7. Development Roadmap

### Maand 1: Foundation
- [ ] Repo setup (monorepo, CI/CD, linting, formatting)
- [ ] Database schema v1 (users, posts, follows, sessions)
- [ ] Auth: OIDC integration (Google, Apple, GitHub)
- [ ] Backend: User CRUD, Post CRUD, Follow/Unfollow
- [ ] Frontend: Landing page, login flow, basic feed

### Maand 2: Social Core
- [ ] Chronological feed (alleen van gevolgde accounts)
- [ ] Comments op posts
- [ ] Image upload + EXIF stripping + resize pipeline
- [ ] Profielpagina's
- [ ] "Mensen die je misschien kent" (mutual follows)
- [ ] Notificaties (in-app)

### Maand 3: MVP Launch
- [ ] DMs (E2EE)
- [ ] Zoeken (gebruikers)
- [ ] Settings (privacy controls, account verwijdering)
- [ ] GDPR data export
- [ ] PWA manifest + service worker
- [ ] Security review
- [ ] Soft launch → `vonk.social` live

### Maand 4–6: V1
- [ ] Stories
- [ ] Video upload + transcoding
- [ ] Groepen
- [ ] Events
- [ ] Verbeterde PWA (offline support, push notifications)
- [ ] Performance optimalisatie

### Maand 7–12: V2
- [ ] Native apps (React Native / Flutter)
- [ ] ActivityPub federatie
- [ ] itsme / EU eID login
- [ ] Multi-datacenter setup
- [ ] Publieke security audit
- [ ] Community governance structuur

---

## 8. Governance & Community

- **Stichting Vonk** (Nederlandse stichting of Belgische VZW): geen aandeelhouders, geen winstoogmerk.
- **Core team**: max 5 maintainers met merge-rechten
- **RFC-proces**: Grote features worden voorgesteld als RFC, community stemt
- **Code of Conduct**: Contributor Covenant
- **Moderatie**: Community-driven (groepsmoderatoren), platform-level alleen voor illegale content
- **Transparantierapport**: Kwartaallijks publiceren hoeveel content verwijderd is en waarom

---

## 9. Hoe Vonk zich onderscheidt

| | Facebook | Instagram | TikTok | **Vonk** |
|---|---|---|---|---|
| Advertenties | Ja | Ja | Ja | **Nee, nooit** |
| Dataverkoop | Ja | Ja | Ja | **Nee, nooit** |
| Algoritme | Engagement-driven | Engagement-driven | Dopamine-loop | **Chronologisch** |
| Suggesties | Behavioral profiling | Behavioral profiling | AI-gestuurd | **Alleen mutual friends** |
| Open source | Nee | Nee | Nee | **Volledig (AGPL-3.0)** |
| Encryptie DMs | Opt-in | Nee | Nee | **Standaard E2EE** |
| EXIF stripping | Deels | Deels | Onbekend | **Volledig, altijd** |
| Self-hostable | Nee | Nee | Nee | **Ja** |
| Eigendom | Meta | Meta | ByteDance | **Stichting/VZW** |

---

## 10. Eerste Stappen (vandaag starten)

1. **Registreer domein**: `vonk.social`
2. **GitHub org aanmaken**: `github.com/vonk-social`
3. **FUNDING.yml**: GitHub Sponsors + Buy Me a Coffee configureren
4. **Repo initialiseren**: Monorepo met SvelteKit + Rust/Go skeleton
5. **Landing page**: Simpele "coming soon" pagina met email signup op `vonk.social`
6. **README + CONTRIBUTING**: Zodat early contributors weten wat de bedoeling is
7. **Database schema**: Eerste migratie schrijven
8. **Auth**: OIDC boilerplate (Google als eerste provider)

---

*Dit document is het levende projectplan voor Vonk. Laatst bijgewerkt: april 2026.*

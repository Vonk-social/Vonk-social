# VONK — Hosting & Distributiemodel (Herzien)

## Het Kernprincipe

Vonk is **één platform** — `vonk.social` — niet duizend losse eilandjes. Er is één gebruikersdatabase, één feed, één community. Wat wél gedistribueerd kan worden is de **infrastructuur eronder**.

```
    ┌─────────────────────────────────────────────────┐
    │              vonk.social                         │
    │         Eén platform, één community              │
    │         Eén account = overal hetzelfde            │
    └──────────────────┬──────────────────────────────┘
                       │
    ╔══════════════════╧════════════════════════╗
    ║        Vonk Infrastructure Layer          ║
    ║                                           ║
    ║  Wie draagt bij?                          ║
    ║  • Stichting Vonk (core servers)          ║
    ║  • Community hosts (extra capaciteit)     ║
    ║  • Sponsors (dedicated hardware)          ║
    ╚══════════════════╤════════════════════════╝
                       │
       ┌───────────────┼───────────────┐
       ▼               ▼               ▼
   ┌────────┐    ┌────────┐    ┌────────┐
   │ Node 1 │    │ Node 2 │    │ Node 3 │
   │ Dimitry│    │ Donor A│    │ Donor B│
   │ België │    │ Duitsl.│    │ Finland│
   └────────┘    └────────┘    └────────┘
```

### Wat het NIET is

| Model | Voorbeeld | Probleem |
|---|---|---|
| Federatie | Mastodon | Elke server is z'n eigen wereld. Fragmentatie. Je moet weten op welke server iemand zit. Verwarrend voor gewone gebruikers. |
| Volledig centraal | Facebook | Eén bedrijf, één datacenter, totale controle. Single point of failure. |

### Wat het WÉL is

**Gedistribueerd gehoste single-tenant applicatie.** Vergelijkbaar met hoe Wikipedia of Signal werkt: één dienst, maar de infra kan gespreid worden.

---

## Fase 0: Op jouw bestaande server

### Setup (dag 1)

Vonk draait als Docker Compose stack naast je bestaande sites. Volledig geïsoleerd via containers.

```
Jouw server (bestaand)
├── nginx (reverse proxy — draait al)
│   ├── wattify.be        → bestaande site
│   ├── wellness...       → bestaande site
│   └── vonk.social       → nieuwe container ←
├── docker compose: vonk
│   ├── vonk-api          (Rust backend)
│   ├── vonk-web          (SvelteKit, SSR)
│   ├── vonk-db           (PostgreSQL 16)
│   ├── vonk-cache        (Valkey/Redis)
│   ├── vonk-media        (MinIO)
│   └── vonk-worker       (video transcoding, jobs)
└── volumes/
    ├── vonk-pgdata/      (database)
    └── vonk-media/       (uploads)
```

### Nginx config (toevoegen aan bestaande setup)

```nginx
server {
    listen 443 ssl http2;
    server_name vonk.social;

    ssl_certificate     /etc/letsencrypt/live/vonk.social/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/vonk.social/privkey.pem;

    # SvelteKit frontend
    location / {
        proxy_pass http://127.0.0.1:3400;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # API
    location /api/ {
        proxy_pass http://127.0.0.1:3401;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # WebSocket (notificaties, DMs)
    location /ws {
        proxy_pass http://127.0.0.1:3401;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $upgrade;
        proxy_set_header Connection "upgrade";
    }

    # Media (direct vanuit MinIO)
    location /media/ {
        proxy_pass http://127.0.0.1:9000/vonk-media/;
        proxy_hide_header x-amz-request-id;
        add_header Cache-Control "public, max-age=31536000, immutable";
    }
}
```

### docker-compose.yml (productie)

```yaml
version: "3.9"

services:
  vonk-db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      POSTGRES_DB: vonk
      POSTGRES_USER: vonk
      POSTGRES_PASSWORD_FILE: /run/secrets/db_password
    volumes:
      - vonk-pgdata:/var/lib/postgresql/data
    ports:
      - "127.0.0.1:5433:5432"   # Niet op 5432 als je al PG draait
    secrets:
      - db_password

  vonk-cache:
    image: valkey/valkey:8-alpine
    restart: unless-stopped
    ports:
      - "127.0.0.1:6380:6379"

  vonk-media:
    image: minio/minio:latest
    restart: unless-stopped
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER_FILE: /run/secrets/minio_user
      MINIO_ROOT_PASSWORD_FILE: /run/secrets/minio_password
    volumes:
      - vonk-media:/data
    ports:
      - "127.0.0.1:9000:9000"
      - "127.0.0.1:9001:9001"
    secrets:
      - minio_user
      - minio_password

  vonk-api:
    image: ghcr.io/vonk-social/vonk-api:latest
    restart: unless-stopped
    depends_on:
      - vonk-db
      - vonk-cache
      - vonk-media
    environment:
      DATABASE_URL: postgres://vonk:${DB_PASS}@vonk-db:5432/vonk
      REDIS_URL: redis://vonk-cache:6379
      S3_ENDPOINT: http://vonk-media:9000
      S3_BUCKET: vonk-media
      OIDC_GOOGLE_CLIENT_ID: ${GOOGLE_CLIENT_ID}
      OIDC_GOOGLE_CLIENT_SECRET: ${GOOGLE_CLIENT_SECRET}
      OIDC_APPLE_CLIENT_ID: ${APPLE_CLIENT_ID}
      JWT_SECRET_FILE: /run/secrets/jwt_secret
      RUST_LOG: info
    ports:
      - "127.0.0.1:3401:8080"
    secrets:
      - jwt_secret

  vonk-web:
    image: ghcr.io/vonk-social/vonk-web:latest
    restart: unless-stopped
    depends_on:
      - vonk-api
    environment:
      PUBLIC_API_URL: https://vonk.social/api
      PUBLIC_WS_URL: wss://vonk.social/ws
    ports:
      - "127.0.0.1:3400:3000"

  vonk-worker:
    image: ghcr.io/vonk-social/vonk-worker:latest
    restart: unless-stopped
    depends_on:
      - vonk-db
      - vonk-cache
      - vonk-media
    environment:
      DATABASE_URL: postgres://vonk:${DB_PASS}@vonk-db:5432/vonk
      REDIS_URL: redis://vonk-cache:6379
      S3_ENDPOINT: http://vonk-media:9000

volumes:
  vonk-pgdata:
  vonk-media:

secrets:
  db_password:
    file: ./secrets/db_password.txt
  jwt_secret:
    file: ./secrets/jwt_secret.txt
  minio_user:
    file: ./secrets/minio_user.txt
  minio_password:
    file: ./secrets/minio_password.txt
```

### Migratie later? Triviaal.

Omdat alles in Docker zit:

```bash
# Op de oude server
docker compose down
pg_dump vonk > vonk_backup.sql
rsync -avz ./vonk-media/ newserver:/opt/vonk/vonk-media/
scp vonk_backup.sql newserver:/tmp/

# Op de nieuwe server
psql vonk < /tmp/vonk_backup.sql
docker compose up -d

# DNS: vonk.social → nieuw IP
# Klaar.
```

Totale downtime: ~5 minuten (DNS propagation even daargelaten).

---

## Het Distributiemoddel: "Vonk Nodes"

### Hoe het werkt

Iemand die hosting wil bijdragen, draait een **Vonk Node** — maar die node is geen apart platform. Het is een worker die meedraait in het Vonk-cluster.

```
┌─────────────────────────────────────────────────┐
│                vonk.social                       │
│            (logisch: één platform)                │
│                                                  │
│  Control Plane (beheerd door Stichting Vonk)     │
│  ├── DNS / Load Balancer (GeoDNS)               │
│  ├── Central DB coordinator (Citus)              │
│  ├── Auth service (centraal, SSO tokens)         │
│  └── Media registry (welke media waar staat)     │
│                                                  │
│  Data Plane (gedistribueerd)                     │
│  ├── Node "core-eu1" — Stichting, Hetzner DE     │
│  ├── Node "donor-be1" — Dimitry, België          │
│  ├── Node "donor-fi1" — Community, Finland       │
│  └── Node "donor-fr1" — Universiteit, Frankrijk  │
└─────────────────────────────────────────────────┘
```

### Wat een Node doet

Een community host draait een subset van de stack:

| Component | Wat het doet | Verplicht? |
|---|---|---|
| **API worker** | Verwerkt API-requests voor users in die regio | Ja |
| **Media cache** | Cached populaire afbeeldingen/video's lokaal | Ja |
| **Media storage** | Slaat media op voor users in die regio | Optioneel |
| **DB read replica** | Lokale read-only kopie van de database | Optioneel |
| **DB shard worker** | Citus worker node (deel van de distributed DB) | Alleen bij schaal |

### Wat de Control Plane doet (altijd centraal)

- **Gebruikersregistratie & authenticatie** → Eén account, overal geldig
- **Database coördinatie** → Citus coordinator bepaalt waar data zit
- **DNS routing** → GeoDNS stuurt gebruikers naar dichtstbijzijnde node
- **Trust management** → Alleen goedgekeurde nodes mogen meedraaien

### Node onboarding (voor een host)

```bash
# 1. Vraag een node-token aan bij Stichting Vonk
curl https://vonk.social/api/infra/register-node \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"name": "donor-be1", "region": "eu-west", "capacity": "medium"}'

# 2. Krijg een config terug
# → node.env met WireGuard keys, DB credentials (read-only), etc.

# 3. Draai de node
docker compose -f docker-compose.node.yml up -d
```

De node communiceert via een **WireGuard VPN tunnel** met de control plane. Geen open poorten nodig, geen complexe firewall-config.

### Vertrouwensmodel

```
Stichting Vonk (core team)
    │
    ├── Vertrouwde nodes (geverifieerde hosts)
    │   → Mogen API-requests verwerken
    │   → Mogen media cachen
    │   → Krijgen read-replica credentials
    │   → Encrypted verbinding via WireGuard
    │
    └── Onvertrouwde nodes → bestaan niet
        Geen open federatie. Je moet goedgekeurd worden.
```

Dit is een bewuste keuze: geen open federatie (Mastodon-model). Het is curated hosting. De stichting bepaalt wie meedraait, met duidelijke criteria:

- Minimum uptime SLA (99%)
- GDPR-compliant hosting (EU only)
- Geen logging van userdata op de node
- Jaarlijkse audit (geautomatiseerd)

---

## Groeipad: Van jouw server naar gedistribueerd

```
Fase 0 (nu)              Fase 1                  Fase 2
─────────────            ────────────            ──────────────
┌───────────┐           ┌───────────┐           ┌─────────────┐
│ Jouw VPS  │           │ Core EU1  │           │ Control     │
│ alles-in-1│  ──→      │ (Hetzner) │           │ Plane       │
│ Docker    │           │ + read    │  ──→      │ (Hetzner)   │
│ Compose   │           │   replica │           ├─────────────┤
└───────────┘           │   op VPS  │           │ Node EU-W   │
                        │   Dimitry │           │ Node EU-N   │
                        └───────────┘           │ Node EU-S   │
                                                └─────────────┘

   1 server               2 servers              Cluster
   < 1K users             < 50K users            50K+ users
   €25/maand              €70/maand              Community
                                                 funded
```

### Fase 0 → 1: Gewoon PostgreSQL streaming replication

```
Jouw server (primary)  ──WAL stream──▶  Hetzner (replica)
  [writes + reads]                       [reads only]
```

Geen Citus nodig. Gewoon `pg_basebackup` + streaming replication. De web-app leest van de replica als die dichter bij de user zit.

### Fase 1 → 2: Citus activeren

Wanneer je merkt dat één PostgreSQL-server het niet meer trekt:

```sql
-- Op de coordinator
CREATE EXTENSION citus;
SELECT citus_set_coordinator_host('core-eu1.vonk.internal');
SELECT citus_add_node('node-eu-w.vonk.internal', 5432);
SELECT citus_add_node('node-eu-n.vonk.internal', 5432);

-- Distribute de grote tabellen
SELECT create_distributed_table('posts', 'user_id');
SELECT create_distributed_table('media', 'user_id');
```

De applicatiecode verandert **niet**. Citus is transparant — het is nog steeds gewoon PostgreSQL.

---

## Samenvatting: De drie lagen

| Laag | Wie beheert het | Wat |
|---|---|---|
| **Platform** | Iedereen (gebruikers) | Eén account, één feed, één community |
| **Control Plane** | Stichting Vonk | Auth, DB coordinator, DNS, trust |
| **Data Plane** | Stichting + community hosts | API workers, media, DB replicas |

De gebruiker merkt er **niets** van. Die gaat naar `vonk.social`, logt in, en gebruikt het platform. Of dat antwoord komt van een server in België of Finland maakt niet uit — het is dezelfde data, dezelfde ervaring.

---

## Checklist: Starten op jouw server

- [ ] Domein `vonk.social` registreren
- [ ] DNS A-record naar jouw server
- [ ] Let's Encrypt cert via certbot
- [ ] `mkdir /opt/vonk && cd /opt/vonk`
- [ ] `docker-compose.yml` plaatsen (zie boven)
- [ ] Secrets genereren (`openssl rand -base64 32 > secrets/jwt_secret.txt` etc.)
- [ ] Nginx vhost toevoegen
- [ ] `docker compose up -d`
- [ ] Google OAuth credentials aanmaken (console.cloud.google.com)
- [ ] Testen: `https://vonk.social` → login scherm

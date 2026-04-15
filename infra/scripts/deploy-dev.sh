#!/usr/bin/env bash
# ------------------------------------------------------------
# Vonk — deploy to dev.vonk.social (76.13.46.169).
#
# Phase 1 is deliberately simple: the staging box builds from source
# with the toolchains already installed there. If rustc / node aren't
# present yet, the preflight block installs them.
#
# Prerequisites handled manually once (see docs/notes/auth.md):
#   - DNS:  dev.vonk.social A 76.13.46.169
#   - TLS:  certbot --nginx -d dev.vonk.social
#   - /opt/vonk/.env populated with real Google creds + JWT_SECRET
#
# Usage:  bash infra/scripts/deploy-dev.sh
# ------------------------------------------------------------
set -euo pipefail

HOST="${VONK_DEV_HOST:-root@76.13.46.169}"
REMOTE="/opt/vonk"

echo "→ pushing latest commit"
git push origin "$(git rev-parse --abbrev-ref HEAD)"

ssh "$HOST" bash -se <<REMOTE_SCRIPT
set -euo pipefail

if [ ! -d "$REMOTE/.git" ]; then
    echo "→ first-time checkout into $REMOTE"
    mkdir -p "$REMOTE"
    cd "$REMOTE"
    git clone git@github.com:Vonk-social/Vonk-social.git .
    echo "→ copy .env.example to .env and edit before re-running this script."
    cp .env.example .env
    exit 0
fi

cd "$REMOTE"

echo "→ pulling latest"
git fetch --all --prune
git checkout main
git pull --ff-only

echo "→ bringing up backing services (postgres, valkey, minio)"
docker compose -f docker-compose.dev.yml up -d

echo "→ building API"
source "\$HOME/.cargo/env" || true
( cd packages/api && cargo build --release )

echo "→ building web"
( cd apps/web && npm ci && npm run build )

echo "→ restarting systemd units"
sudo systemctl restart vonk-api.service || true
sudo systemctl restart vonk-web.service || true

echo "→ health check"
curl -fsSL http://127.0.0.1:3401/api/health

echo "✓ deployed"
REMOTE_SCRIPT

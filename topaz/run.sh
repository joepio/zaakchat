#!/bin/bash
set -e

# Use standard Topaz environment variables
# We avoid overriding TOPAZ_CERTS_DIR and TOPAZ_DB_DIR manually to prevent mount conflicts
# unless we strictly need to. For local dev, system defaults are safer.

echo ">>> Stopping any existing Topaz..."
topaz stop || true

echo ">>> Registering ZaakChat configuration..."
mkdir -p ~/.config/topaz/cfg
cp "$(pwd)/topaz/cfg/config.yaml" ~/.config/topaz/cfg/zaakchat.yaml
topaz config use zaakchat

echo ">>> Starting Topaz..."
topaz start

echo ">>> Waiting for Topaz services to stabilize..."
for i in {1..15}; do
  if curl -s http://localhost:9494/healthz > /dev/null; then
    echo ">>> Topaz is healthy!"
    break
  fi
  if [ $i -eq 15 ]; then
    echo ">>> Topaz failed to start in time. Checking logs..."
    docker logs topaz || true
    exit 1
  fi
  echo ">>> Waiting... ($i/15)"
  sleep 2
done

echo ">>> Applying ZaakChat manifest..."
topaz directory set manifest "$(pwd)/topaz/model/manifest.yaml" --host localhost:9292 -P

echo ">>> Importing ZaakChat seed data..."
topaz directory import -d "$(pwd)/topaz/data" --host localhost:9292 -P

echo ">>> Topaz is ready and configured for ZaakChat!"

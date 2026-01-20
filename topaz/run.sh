#!/usr/bin/env fish

# Setup Topaz for ZaakChat
set -x TOPAZ_BASE_DIR (pwd)/topaz
set -x TOPAZ_CERTS_DIR $TOPAZ_BASE_DIR/certs
set -x TOPAZ_DB_DIR $TOPAZ_BASE_DIR/db

# Ensure directories exist
mkdir -p $TOPAZ_CERTS_DIR
mkdir -p $TOPAZ_DB_DIR
mkdir -p $TOPAZ_BASE_DIR/cfg
mkdir -p $TOPAZ_BASE_DIR/model
mkdir -p $TOPAZ_BASE_DIR/data

# Sync config to Topaz's global config directory
set GLOBAL_CONFIG_DIR "$HOME/.config/topaz/cfg"
mkdir -p $GLOBAL_CONFIG_DIR
ln -sf $TOPAZ_BASE_DIR/cfg/config.yaml $GLOBAL_CONFIG_DIR/zaakchat.yaml

echo ">>> Configuration linked to $GLOBAL_CONFIG_DIR/zaakchat.yaml"

# Generate certificates if missing
if not test -f $TOPAZ_CERTS_DIR/grpc.crt
    echo ">>> Generating certificates..."
    topaz certs generate --certs-dir $TOPAZ_CERTS_DIR
end

echo ">>> Stopping any existing Topaz..."
topaz stop || true

echo ">>> Starting Topaz (zaakchat)..."
topaz config use zaakchat
topaz start

echo ">>> Waiting for Topaz services to stabilize..."
for i in (seq 1 30)
  if curl -s http://localhost:9494/healthz > /dev/null
    if nc -z localhost 9292
      echo ">>> Topaz is healthy and directory is listening!"
      break
    end
  end
  if test $i -eq 30
    echo ">>> Topaz failed to start in time. Checking logs..."
    docker logs topaz || true
    exit 1
  end
  echo ">>> Waiting for services... ($i/30)"
  sleep 2
end

echo ">>> Applying ZaakChat Manifest..."
topaz directory set manifest $TOPAZ_BASE_DIR/model/manifest.yaml --host localhost:9292 --plaintext

echo ">>> Importing Seed Data..."
topaz directory import -d $TOPAZ_BASE_DIR/data/seed.json --host localhost:9292 --plaintext

echo ">>> PDP is ready for ZaakChat!"

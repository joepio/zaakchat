#!/bin/bash
set -e

echo "🚀 Deploying ZaakChat..."

# Configuration
IMAGE="docker.io/joepmeneer/zaakchat:latest"
CONTAINER_NAME="zaakchat"
PORT="8000"
DATA_DIR="/root/zaakchat-data"

# Create data directory if it doesn't exist
mkdir -p "$DATA_DIR"

# Fix permissions: The container runs as non-root user (UID 1000).
# We need to ensure the host directory is writable by this user.
echo "🔧 Fixing permissions for $DATA_DIR..."
chown -R 1000:1000 "$DATA_DIR"

echo "📥 Pulling latest image..."
podman pull "$IMAGE"

echo "🛑 Stopping existing container (if any)..."
if podman ps -a --format "{{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
    podman stop "$CONTAINER_NAME" || true
    podman rm "$CONTAINER_NAME" || true
    echo "✓ Stopped and removed old container"
else
    echo "ℹ️  No existing container found"
fi

echo "🚀 Starting new container..."
podman run -d \
    --name "$CONTAINER_NAME" \
    -p "${PORT}:8000" \
    -v "${DATA_DIR}:/app/data" \
    --restart=unless-stopped \
    "$IMAGE"

echo "✅ Deployment complete!"
echo ""
echo "📊 Container status:"
podman ps --filter "name=${CONTAINER_NAME}"
echo ""
echo "📝 View logs with: podman logs -f ${CONTAINER_NAME}"
echo "🌐 Access at: http://$(hostname -I | awk '{print $1}'):${PORT}"

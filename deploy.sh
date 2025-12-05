#!/bin/bash
set -e

echo "🚀 Deploying ZaakChat..."

# Configuration
IMAGE="docker.io/joepmeneer/zaakchat:latest"
CONTAINER_NAME="zaakchat"
PORT="8000"
VOLUME_NAME="zaakchat_data"

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
    -v "${VOLUME_NAME}:/app/data" \
    -e POSTMARK_API_TOKEN \
    -e POSTMARK_SENDER_EMAIL="noreply@zaakchat.nl" \
    -e BASE_URL="https://zaakchat.nl" \
    --restart=unless-stopped \
    "$IMAGE"

echo "✅ Deployment complete!"
echo ""
echo "📊 Container status:"
podman ps --filter "name=${CONTAINER_NAME}"
echo ""
echo "📝 View logs with: podman logs -f ${CONTAINER_NAME}"
echo "🌐 Access at: http://$(hostname -I | awk '{print $1}'):${PORT}"

#!/bin/bash
set -e

echo "🚀 Deploying ZaakChat..."

# Configuration
IMAGE="docker.io/joepmeneer/zaakchat:latest"
CONTAINER_NAME="zaakchat"
PORT="8000"
VOLUME_NAME="zaakchat_data"

# Check for podman-compose
if ! command -v podman-compose &> /dev/null; then
    echo "🔧 Installing podman-compose..."
    sudo apt-get update && sudo apt-get install -y podman-compose
fi

echo "📥 Pulling latest images..."
podman-compose pull

echo "🚀 Starting services with Docker Compose..."
podman-compose down || true
podman-compose up -d

echo "✅ Deployment complete!"
echo ""
echo "📊 Service status:"
podman-compose ps
echo ""
echo "📝 View logs with: podman-compose logs -f"
echo "🌐 Access at: https://zaakchat.nl"

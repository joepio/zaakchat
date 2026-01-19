# ZaakChat

Dit is een simpele front-end applicatie om te demonstreren hoe JSONCommit (CloudEvents + JSON) zou kunnen werken voor het doen van gesprekken (chats) over lopende zaken.

Dit project bevat:

- **Front-end**, met real-time updates, commenting, planningen, acties, schema-driven formulieren, push-notificaties en een zoekfunctionaliteit. Geschreven in React met Vite.
- **Back-end** met API endpoints voor `/events` voor JSONCommits / CloudEvents en `/schemas` endpoints voor JSON schema serving. Geschreven in Rust met Axum en Tokio.
- **AsyncAPI specificatie** voor het protocol op basis van CloudEvents + SSE, met daarin schemas voor de verschillende berichten. Deze wordt gegenereerd door het back-end.

## Voor inwoners

- **Overzichtelijk**. Al jouw zaken, taken, en de planning in 1 overzicht.
- **Gebruiksvriendelijk**. Een overzichtelijke tijdlijn van alles wat er gebeurt. Direct kunnen chatten met jouw gemeente.
- **Snel**. Geschreven in Rust en React. Sneller is haast niet mogelijk.

## Voor gemeenten

- **Zelf hosten**. Docker + Docker-Compose is beschikbaar. Geen externe database nodig.
- **Open source**. Geen addertjes onder het gras.

![Screenshot](screenshot.png)

## Lokaal draaien

```sh
# Zorg dat node, pnpm en cargo zijn geinstalleerd

# Installeer de vereiste dependencies
pnpm i

# Genereer de schema's
pnpm run generate-all

# Start de front-end applicatie
pnpm run dev

# Start de back-end applicatie
cargo run

# Start de AsyncAPI portal voor de specificaties
pnpm run spec

# Start de tests
pnpm run test
pnpm run test:e2e
pnpm run test:e2e:one "schema form"

# Bouw de docker
docker build -t joepmeneer/zaakchat:latest .
# run de docker
docker run -d \
  -p 8000:8000 \
  -v zaakchat_local_data:/app/data \
  -e MOCK_EMAIL=true \
  docker.io/joepmeneer/zaakchat:latest
```

## Deployment

It's currently running on a VPS on DigitalOcean (161.35.156.229).
The mails are sent through PostMark.
Both are managed by Joep Meindertsma.

### Deploy to VPS

1. Copy the deployment script to your VPS:
```sh
scp deploy.sh root@161.35.156.229:~/deploy-zaakchat.sh
```

2. SSH into your VPS and run the script:
```sh
ssh root@161.35.156.229
chmod +x deploy-zaakchat.sh
# Pull image and restart container
./deploy-zaakchat.sh
```

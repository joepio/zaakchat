# MijnZaken - JSONCommit

Dit is een simpele front-end applicatie om te demonstreren hoe JSONCommit (CloudEvents + JSON) zou kunnen werken voor MijnServices applicaties voor VNG Realisatie.

Dit project bevat:

- **Front-end voor MijnZaken**, met real-time updates, commenting, planningen, acties, schema-driven formulieren, push-notificaties en een zoekfunctionaliteit. Geschreven in React met Vite.
- **Back-end** met API endpoints voor `/events` voor JSONCommits / CloudEvents en `/schemas` endpoints voor JSON schema serving. Geschreven in Rust met Axum en Tokio.
- **AsyncAPI specificatie** voor het protocol op basis van CloudEvents + SSE, met daarin schemas voor de verschillende berichten. Deze wordt gegenereerd door het back-end.

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
docker build -t joepmeneer/sse-demo:latest .
# run de docker
docker run -p 8000:8000 -v "$(pwd)/data:/app/data" docker.io/joepmeneer/sse-demo:latest
```

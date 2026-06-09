# 128

Browser-based multiplayer point-and-click mini-games.

## Workspace

- `apps/web`: browser client
- `apps/server`: Rust/WebTransport authoritative server for the single active game instance
- `packages/protocol`: network message definitions
- `packages/game-core`: backend-independent game primitives
- `packages/games`: mini-game implementations
- `tooling`: shared project configuration

## Getting Started

Install current stable Rust, Node.js, and enable Corepack. Then install the browser
workspace dependencies:

```sh
corepack enable
pnpm install
```

Start the Rust server:

```sh
pnpm dev:server
```

The server creates a WebTransport-compatible development certificate on first startup and
prints its SHA-256 hash. Start the web client in another terminal using that value:

```sh
VITE_WEBTRANSPORT_CERT_HASH=<printed-hash> pnpm dev:web
```

Pointer movement uses unreliable WebTransport datagrams. Discrete events use reliable
unidirectional streams. Production deployment should use a trusted TLS certificate rather
than the generated development certificate.

Browser-pinned WebTransport development certificates are valid for at most two weeks.
Delete `apps/server/certs/cert.pem` and `apps/server/certs/key.pem` to regenerate them.

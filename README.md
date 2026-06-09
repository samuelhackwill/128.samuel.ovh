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

The authoritative server defines a `1920 x 1080` logical world. Browsers preserve its
16:9 aspect ratio with letterboxing or pillarboxing and translate local pointer positions
into logical world coordinates before sending them. Cursor targets are simulated centrally
with bounded movement, world constraints, and server-authoritative collisions against the
visible pointer silhouette. The server sends that same pointer polygon to browsers for
batched Pixi rendering.

Use the `player` query parameter to request a visible pointer number:

```text
https://128.samuel.ovh/?player=12
```

Missing, invalid, or already-used numbers are replaced by the first available number, and
the browser updates its URL to show the number assigned by the server.

Browser-pinned WebTransport development certificates are valid for at most two weeks.
Delete `apps/server/certs/cert.pem` and `apps/server/certs/key.pem` to regenerate them.

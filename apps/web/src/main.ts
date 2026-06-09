import type { ServerEvent } from "@128/protocol";

import { createAppRoot } from "./app/index.js";
import { listenForMouseInput } from "./input/mouse.js";
import { connectToGame } from "./network/client.js";
import { createRenderer } from "./rendering/create-renderer.js";
import { createPlayerCursorRenderer } from "./rendering/player-cursors.js";
import "./styles.css";

async function bootstrap(): Promise<void> {
  const container = document.querySelector<HTMLElement>("#app");

  if (!container) {
    throw new Error("Missing #app container");
  }

  const appRoot = createAppRoot(container);
  const renderer = await createRenderer(appRoot);
  const playerCursors = createPlayerCursorRenderer(renderer.stage, renderer.ticker);

  const connection = connectToGame(
    import.meta.env.VITE_WEBTRANSPORT_URL ?? "https://localhost:4433/game",
    (event) => handleServerEvent(event, playerCursors),
  );
  await connection.ready;
  listenForMouseInput(appRoot, (event) => connection.send(event));
}

function handleServerEvent(
  event: ServerEvent,
  cursors: ReturnType<typeof createPlayerCursorRenderer>,
): void {
  if (event.type === "connected") {
    cursors.setLocalPlayerId(event.playerId);
  } else if (event.type === "state") {
    cursors.update(event.players);
  }
}

void bootstrap().catch((error: unknown) => {
  console.error("Failed to start 128", error);
});

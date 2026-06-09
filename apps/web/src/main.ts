import type { ServerEvent } from "@128/protocol";

import { createAppRoot } from "./app/index.js";
import { listenForMouseInput } from "./input/mouse.js";
import { connectToGame } from "./network/client.js";
import { createRenderer } from "./rendering/create-renderer.js";
import { createPlayerCursorRenderer } from "./rendering/player-cursors.js";
import { createWorldViewport } from "./rendering/world-viewport.js";
import "./styles.css";

async function bootstrap(): Promise<void> {
  const container = document.querySelector<HTMLElement>("#app");

  if (!container) {
    throw new Error("Missing #app container");
  }

  const appRoot = createAppRoot(container);
  const renderer = await createRenderer(appRoot);
  const worldViewport = createWorldViewport(renderer);
  const playerCursors = createPlayerCursorRenderer(worldViewport.container, renderer.ticker);
  let resolveWorldReady = (): void => {};
  const worldReady = new Promise<void>((resolve) => {
    resolveWorldReady = resolve;
  });

  const connection = connectToGame(
    createGameUrl(import.meta.env.VITE_WEBTRANSPORT_URL ?? "https://localhost:4433/game"),
    (event) => {
      if (event.type === "connected") {
        worldViewport.setSize(event.world);
        playerCursors.setPointerShape(event.world.pointer);
        setPagePlayerNumber(event.playerNumber);
        resolveWorldReady();
      }
      handleServerEvent(event, playerCursors);
    },
  );
  await Promise.all([connection.ready, worldReady]);
  listenForMouseInput(
    renderer.canvas,
    (clientX, clientY) => worldViewport.clientToWorld(clientX, clientY),
    (event) => connection.send(event),
  );
}

function handleServerEvent(
  event: ServerEvent,
  cursors: ReturnType<typeof createPlayerCursorRenderer>,
): void {
  if (event.type === "state") {
    cursors.update(event.players);
  }
}

function createGameUrl(baseUrl: string): string {
  const url = new URL(baseUrl, window.location.href);
  const playerNumber = readRequestedPlayerNumber();

  if (playerNumber !== undefined) {
    url.searchParams.set("player", String(playerNumber));
  }

  return url.toString();
}

function readRequestedPlayerNumber(): number | undefined {
  const value = new URLSearchParams(window.location.search).get("player");
  if (!value || !/^\d+$/.test(value)) {
    return undefined;
  }

  const number = Number(value);
  return Number.isInteger(number) && number > 0 && number <= 65_535 ? number : undefined;
}

function setPagePlayerNumber(playerNumber: number): void {
  const url = new URL(window.location.href);
  url.searchParams.set("player", String(playerNumber));
  window.history.replaceState(null, "", url);
}

void bootstrap().catch((error: unknown) => {
  console.error("Failed to start 128", error);
});

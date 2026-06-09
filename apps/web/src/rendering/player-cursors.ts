import type { ServerEvent } from "@128/protocol";
import {
  BitmapFont,
  BitmapText,
  Container,
  Graphics,
  GraphicsContext,
  type Ticker,
} from "pixi.js";

import type { PointerShape } from "./world-viewport.js";

type StateEvent = Extract<ServerEvent, { type: "state" }>;

interface Cursor {
  label: BitmapText;
  targetX: number;
  targetY: number;
  view: Container;
}

export interface PlayerCursorRenderer {
  destroy(): void;
  setPointerShape(pointer: PointerShape): void;
  update(players: StateEvent["players"]): void;
}

const CURSOR_SMOOTHING = 0.35;
const PLAYER_NUMBER_FONT = "128-player-number";
const PLAYER_NUMBER_FONT_SIZE = 14;
let playerNumberFontInstalled = false;

export function createPlayerCursorRenderer(
  stage: Container,
  ticker: Ticker,
): PlayerCursorRenderer {
  const layer = new Container();
  const cursors = new Map<string, Cursor>();
  let pointer: PointerShape | undefined;
  let pointerContext: GraphicsContext | undefined;

  installPlayerNumberFont();
  stage.addChild(layer);

  const animate = (): void => {
    for (const cursor of cursors.values()) {
      cursor.view.x += (cursor.targetX - cursor.view.x) * CURSOR_SMOOTHING;
      cursor.view.y += (cursor.targetY - cursor.view.y) * CURSOR_SMOOTHING;
    }
  };

  const clearCursors = (): void => {
    for (const cursor of cursors.values()) {
      cursor.view.destroy({ children: true });
    }
    cursors.clear();
  };

  ticker.add(animate);

  return {
    destroy: () => {
      ticker.remove(animate);
      clearCursors();
      pointerContext?.destroy();
      layer.destroy();
    },
    setPointerShape: (nextPointer) => {
      clearCursors();
      pointerContext?.destroy();
      pointer = nextPointer;
      pointerContext = createPointerContext(nextPointer);
    },
    update: (players) => {
      if (!pointer || !pointerContext) {
        return;
      }

      const activePlayers = new Set(players.map((player) => player.id));

      for (const [id, cursor] of cursors) {
        if (!activePlayers.has(id)) {
          cursor.view.destroy({ children: true });
          cursors.delete(id);
        }
      }

      for (const player of players) {
        let cursor = cursors.get(player.id);

        if (!cursor) {
          cursor = createCursor(pointer, pointerContext, player.number);
          cursor.view.position.set(player.x, player.y);
          cursor.targetX = player.x;
          cursor.targetY = player.y;
          layer.addChild(cursor.view);
          cursors.set(player.id, cursor);
        } else if (cursor.label.text !== String(player.number)) {
          cursor.label.text = String(player.number);
        }

        cursor.targetX = player.x;
        cursor.targetY = player.y;
      }
    },
  };
}

function createPointerContext(pointer: PointerShape): GraphicsContext {
  const points = pointer.points.flatMap((point) => [point.x, point.y]);

  return new GraphicsContext()
    .poly(points)
    .fill(0x000000)
    .stroke({
      color: 0xffffff,
      join: "miter",
      miterLimit: 4,
      pixelLine: true,
      width: 1,
    });
}

function createCursor(
  pointer: PointerShape,
  pointerContext: GraphicsContext,
  playerNumber: number,
): Cursor {
  const view = new Container();
  const graphic = new Graphics({ context: pointerContext, roundPixels: true });
  const label = new BitmapText({
    roundPixels: true,
    style: {
      fill: 0xffffff,
      fontFamily: PLAYER_NUMBER_FONT,
      fontSize: PLAYER_NUMBER_FONT_SIZE,
      stroke: { color: 0x000000, width: 3 },
    },
    text: String(playerNumber),
  });

  label.anchor.set(0, 1);
  label.position.set(pointer.width + 4, pointer.height);
  view.addChild(graphic, label);

  return {
    label,
    targetX: 0,
    targetY: 0,
    view,
  };
}

function installPlayerNumberFont(): void {
  if (playerNumberFontInstalled) {
    return;
  }

  BitmapFont.install({
    chars: "0123456789",
    name: PLAYER_NUMBER_FONT,
    padding: 4,
    resolution: Math.min(window.devicePixelRatio, 2),
    style: {
      fill: 0xffffff,
      fontFamily: "Courier New",
      fontSize: PLAYER_NUMBER_FONT_SIZE,
      fontWeight: "bold",
      stroke: { color: 0x000000, width: 3 },
    },
  });
  playerNumberFontInstalled = true;
}

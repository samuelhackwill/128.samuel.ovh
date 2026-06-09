import type { ServerEvent } from "@128/protocol";
import { Container, Graphics, type Ticker } from "pixi.js";

type StateEvent = Extract<ServerEvent, { type: "state" }>;

interface Cursor {
  graphic: Graphics;
  targetX: number;
  targetY: number;
}

export interface PlayerCursorRenderer {
  destroy(): void;
  setLocalPlayerId(playerId: string): void;
  update(players: StateEvent["players"]): void;
}

const CURSOR_SMOOTHING = 0.35;

export function createPlayerCursorRenderer(
  stage: Container,
  ticker: Ticker,
): PlayerCursorRenderer {
  const layer = new Container();
  const cursors = new Map<string, Cursor>();
  let localPlayerId: string | undefined;

  stage.addChild(layer);

  const animate = (): void => {
    for (const cursor of cursors.values()) {
      cursor.graphic.x += (cursor.targetX - cursor.graphic.x) * CURSOR_SMOOTHING;
      cursor.graphic.y += (cursor.targetY - cursor.graphic.y) * CURSOR_SMOOTHING;
    }
  };

  ticker.add(animate);

  return {
    destroy: () => {
      ticker.remove(animate);
      layer.destroy({ children: true });
      cursors.clear();
    },
    setLocalPlayerId: (playerId) => {
      localPlayerId = playerId;
      for (const [id, cursor] of cursors) {
        cursor.graphic.alpha = id === localPlayerId ? 1 : 0.8;
      }
    },
    update: (players) => {
      const activePlayers = new Set(players.map((player) => player.id));

      for (const [id, cursor] of cursors) {
        if (!activePlayers.has(id)) {
          cursor.graphic.destroy();
          cursors.delete(id);
        }
      }

      for (const player of players) {
        let cursor = cursors.get(player.id);

        if (!cursor) {
          const graphic = createCursorGraphic(colorFromPlayerId(player.id));
          graphic.x = player.x;
          graphic.y = player.y;
          graphic.alpha = player.id === localPlayerId ? 1 : 0.8;
          layer.addChild(graphic);

          cursor = {
            graphic,
            targetX: player.x,
            targetY: player.y,
          };
          cursors.set(player.id, cursor);
        }

        cursor.targetX = player.x;
        cursor.targetY = player.y;
      }
    },
  };
}

function createCursorGraphic(color: number): Graphics {
  return new Graphics()
    .poly([0, 0, 0, 26, 7, 19, 12, 31, 18, 28, 13, 17, 23, 17])
    .fill(color)
    .stroke({ color: 0xffffff, width: 2 });
}

function colorFromPlayerId(playerId: string): number {
  let hash = 0;
  for (const character of playerId) {
    hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
  }

  const red = 80 + (hash & 0x7f);
  const green = 80 + ((hash >>> 8) & 0x7f);
  const blue = 80 + ((hash >>> 16) & 0x7f);
  return (red << 16) | (green << 8) | blue;
}

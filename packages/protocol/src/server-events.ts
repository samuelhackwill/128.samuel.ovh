export type ServerEvent =
  | {
      type: "connected";
      playerId: string;
      world: {
        width: number;
        height: number;
        cursorRadius: number;
      };
    }
  | {
      type: "player-joined";
      playerId: string;
    }
  | {
      type: "player-left";
      playerId: string;
    }
  | {
      type: "state";
      serverTime: number;
      tick: number;
      players: Array<{
        id: string;
        x: number;
        y: number;
      }>;
    };

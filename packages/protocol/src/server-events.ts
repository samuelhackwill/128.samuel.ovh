export type ServerEvent =
  | {
      type: "connected";
      playerId: string;
      playerNumber: number;
      world: {
        width: number;
        height: number;
        pointer: {
          width: number;
          height: number;
          points: Array<{
            x: number;
            y: number;
          }>;
        };
      };
    }
  | {
      type: "player-joined";
      playerId: string;
      playerNumber: number;
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
        number: number;
        x: number;
        y: number;
      }>;
    };

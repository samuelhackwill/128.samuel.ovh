import type { Game } from "@128/game-core";

export interface ExampleGameState {
  tick: number;
}

export const exampleGame: Game<ExampleGameState> = {
  createState: () => ({ tick: 0 }),
  update: (state) => {
    state.tick += 1;
  },
};

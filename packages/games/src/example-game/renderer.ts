import type { ExampleGameState } from "./simulation.js";

export interface ExampleGameRenderer {
  render(state: ExampleGameState): void;
}

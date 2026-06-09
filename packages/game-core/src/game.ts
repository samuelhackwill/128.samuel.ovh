export interface GameState {
  tick: number;
}

export interface Game<TState extends GameState = GameState> {
  createState(): TState;
  update(state: TState, deltaSeconds: number): void;
}

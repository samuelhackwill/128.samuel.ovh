export type ClientEvent =
  | {
      type: "transport-capabilities";
      datagrams: boolean;
    }
  | {
      type: "pointer-move";
      sequence: number;
      clientTime: number;
      x: number;
      y: number;
    }
  | {
      type: "pointer-button";
      button: number;
      pressed: boolean;
      sequence: number;
      clientTime: number;
    };

import type { ClientEvent } from "@128/protocol";

export function listenForMouseInput(
  target: HTMLElement,
  send: (event: ClientEvent) => void,
): () => void {
  let sequence = 0;

  const onPointerMove = (event: PointerEvent): void => {
    send({
      type: "pointer-move",
      sequence: sequence++,
      clientTime: performance.now(),
      x: event.clientX,
      y: event.clientY,
    });
  };
  const onPointerDown = (event: PointerEvent): void => {
    send({
      type: "pointer-button",
      button: event.button,
      pressed: true,
      sequence: sequence++,
      clientTime: performance.now(),
    });
  };
  const onPointerUp = (event: PointerEvent): void => {
    send({
      type: "pointer-button",
      button: event.button,
      pressed: false,
      sequence: sequence++,
      clientTime: performance.now(),
    });
  };

  target.addEventListener("pointermove", onPointerMove);
  target.addEventListener("pointerdown", onPointerDown);
  target.addEventListener("pointerup", onPointerUp);

  return () => {
    target.removeEventListener("pointermove", onPointerMove);
    target.removeEventListener("pointerdown", onPointerDown);
    target.removeEventListener("pointerup", onPointerUp);
  };
}

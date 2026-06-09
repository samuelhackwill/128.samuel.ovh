import { Application, Container, Graphics } from "pixi.js";

export interface WorldPoint {
  x: number;
  y: number;
}

export interface WorldSize {
  width: number;
  height: number;
  cursorRadius: number;
}

export interface WorldViewport {
  readonly container: Container;
  clientToWorld(clientX: number, clientY: number): WorldPoint;
  destroy(): void;
  setSize(size: WorldSize): void;
}

export function createWorldViewport(application: Application): WorldViewport {
  const container = new Container();
  const background = new Graphics();
  let width = 1;
  let height = 1;
  let scale = 1;
  let lastScreenWidth = 0;
  let lastScreenHeight = 0;

  container.addChild(background);
  application.stage.addChild(container);

  const layout = (): void => {
    const screenWidth = application.screen.width;
    const screenHeight = application.screen.height;

    if (screenWidth === lastScreenWidth && screenHeight === lastScreenHeight) {
      return;
    }

    lastScreenWidth = screenWidth;
    lastScreenHeight = screenHeight;
    scale = Math.min(screenWidth / width, screenHeight / height);
    container.scale.set(scale);
    container.position.set(
      (screenWidth - width * scale) / 2,
      (screenHeight - height * scale) / 2,
    );
  };

  application.ticker.add(layout);

  return {
    container,
    clientToWorld: (clientX, clientY) => {
      const bounds = application.canvas.getBoundingClientRect();
      const screenX = (clientX - bounds.left) * (application.screen.width / bounds.width);
      const screenY = (clientY - bounds.top) * (application.screen.height / bounds.height);

      return {
        x: clamp((screenX - container.x) / scale, 0, width),
        y: clamp((screenY - container.y) / scale, 0, height),
      };
    },
    destroy: () => {
      application.ticker.remove(layout);
      container.destroy({ children: true });
    },
    setSize: (size) => {
      if (
        !Number.isFinite(size.width) ||
        !Number.isFinite(size.height) ||
        !Number.isFinite(size.cursorRadius) ||
        size.width <= 0 ||
        size.height <= 0 ||
        size.cursorRadius <= 0 ||
        size.cursorRadius * 2 > Math.min(size.width, size.height)
      ) {
        throw new Error("Server provided an invalid world size");
      }

      width = size.width;
      height = size.height;
      background.clear().rect(0, 0, width, height).fill(0x111111);
      lastScreenWidth = 0;
      lastScreenHeight = 0;
      layout();
    },
  };
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(Math.max(value, minimum), maximum);
}

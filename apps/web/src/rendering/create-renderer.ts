import { Application } from "pixi.js";

export async function createRenderer(container: HTMLElement): Promise<Application> {
  const application = new Application();
  await application.init({
    antialias: true,
    autoDensity: true,
    background: "#030303",
    resizeTo: container,
    resolution: Math.min(window.devicePixelRatio, 2),
  });
  container.append(application.canvas);

  return application;
}

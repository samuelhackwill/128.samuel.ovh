import { Application } from "pixi.js";

export async function createRenderer(container: HTMLElement): Promise<Application> {
  const application = new Application();
  await application.init({ background: "#030303", resizeTo: container });
  container.append(application.canvas);

  return application;
}

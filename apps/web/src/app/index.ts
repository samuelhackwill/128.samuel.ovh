export function createAppRoot(container: HTMLElement): HTMLElement {
  const root = document.createElement("div");
  root.dataset.app = "128";
  container.append(root);

  return root;
}

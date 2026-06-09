export interface Circle {
  radius: number;
  x: number;
  y: number;
}

export function circlesOverlap(a: Circle, b: Circle): boolean {
  const x = a.x - b.x;
  const y = a.y - b.y;
  const radius = a.radius + b.radius;

  return x * x + y * y <= radius * radius;
}

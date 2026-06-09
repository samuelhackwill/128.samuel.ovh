use crate::protocol::{POINTER_HEIGHT, POINTER_POINTS, POINTER_WIDTH, Point, WORLD_CONFIG};

const CURSOR_MAX_SPEED: f64 = 9000.0;
const MAX_SUBSTEP_DISTANCE: f64 = 4.0;
const COLLISION_ITERATIONS: usize = 12;
const COLLISION_EPSILON: f64 = 0.001;

// Convex decomposition of the canonical concave pointer polygon for SAT collision checks.
const HEAD_LEFT: [Point; 3] = [POINTER_POINTS[0], POINTER_POINTS[1], POINTER_POINTS[2]];
const HEAD_MIDDLE: [Point; 3] = [POINTER_POINTS[0], POINTER_POINTS[2], POINTER_POINTS[5]];
const HEAD_RIGHT: [Point; 3] = [POINTER_POINTS[0], POINTER_POINTS[5], POINTER_POINTS[6]];
const STEM: [Point; 4] = [
    POINTER_POINTS[2],
    POINTER_POINTS[3],
    POINTER_POINTS[4],
    POINTER_POINTS[5],
];
const COLLISION_PIECES: [&[Point]; 4] = [&HEAD_LEFT, &HEAD_MIDDLE, &HEAD_RIGHT, &STEM];

#[derive(Clone, Copy, Debug)]
pub struct CursorBody {
    pub x: f64,
    pub y: f64,
    target_x: f64,
    target_y: f64,
}

#[derive(Clone, Copy)]
struct Collision {
    normal_x: f64,
    normal_y: f64,
    depth: f64,
}

impl CursorBody {
    pub fn centered() -> Self {
        let x = (WORLD_CONFIG.width - POINTER_WIDTH) / 2.0;
        let y = (WORLD_CONFIG.height - POINTER_HEIGHT) / 2.0;

        Self {
            x,
            y,
            target_x: x,
            target_y: y,
        }
    }

    pub fn set_target(&mut self, x: f64, y: f64) {
        self.target_x = x;
        self.target_y = y;
    }
}

pub fn simulate_cursors(bodies: &mut [CursorBody], delta_seconds: f64) {
    if bodies.is_empty() || !delta_seconds.is_finite() || delta_seconds <= 0.0 {
        return;
    }

    let maximum_movement = bodies
        .iter()
        .map(|body| {
            distance(body.x, body.y, body.target_x, body.target_y)
                .min(CURSOR_MAX_SPEED * delta_seconds)
        })
        .fold(0.0, f64::max);
    let substeps = (maximum_movement / MAX_SUBSTEP_DISTANCE).ceil().max(1.0) as usize;
    let movement_per_substep = CURSOR_MAX_SPEED * delta_seconds / substeps as f64;

    for _ in 0..substeps {
        for body in &mut *bodies {
            move_toward_target(body, movement_per_substep);
            constrain_to_world(body);
        }

        for _ in 0..COLLISION_ITERATIONS {
            resolve_collisions(bodies);
            for body in &mut *bodies {
                constrain_to_world(body);
            }
        }
    }
}

fn move_toward_target(body: &mut CursorBody, maximum_distance: f64) {
    let dx = body.target_x - body.x;
    let dy = body.target_y - body.y;
    let distance = dx.hypot(dy);

    if distance <= maximum_distance {
        body.x = body.target_x;
        body.y = body.target_y;
    } else if distance > 0.0 {
        let scale = maximum_distance / distance;
        body.x += dx * scale;
        body.y += dy * scale;
    }
}

fn resolve_collisions(bodies: &mut [CursorBody]) {
    for left_index in 0..bodies.len() {
        for right_index in (left_index + 1)..bodies.len() {
            let (left, right) = bodies.split_at_mut(right_index);
            let left = &mut left[left_index];
            let right = &mut right[0];

            let Some(collision) = pointer_collision(left, right, left_index, right_index) else {
                continue;
            };
            let correction = (collision.depth + COLLISION_EPSILON) / 2.0;

            left.x -= collision.normal_x * correction;
            left.y -= collision.normal_y * correction;
            right.x += collision.normal_x * correction;
            right.y += collision.normal_y * correction;
        }
    }
}

fn pointer_collision(
    left: &CursorBody,
    right: &CursorBody,
    left_index: usize,
    right_index: usize,
) -> Option<Collision> {
    if !aabbs_overlap(left, right) {
        return None;
    }

    let fallback_direction = separation_direction(left_index, right_index);
    let direction = if (right.x - left.x).hypot(right.y - left.y) > f64::EPSILON {
        (right.x - left.x, right.y - left.y)
    } else {
        fallback_direction
    };
    let mut deepest: Option<Collision> = None;

    for left_piece in COLLISION_PIECES {
        for right_piece in COLLISION_PIECES {
            let Some(collision) = convex_collision(left_piece, left, right_piece, right, direction)
            else {
                continue;
            };

            if deepest.is_none_or(|current| collision.depth > current.depth) {
                deepest = Some(collision);
            }
        }
    }

    deepest
}

fn convex_collision(
    left_piece: &[Point],
    left: &CursorBody,
    right_piece: &[Point],
    right: &CursorBody,
    direction: (f64, f64),
) -> Option<Collision> {
    let mut minimum = Collision {
        normal_x: 0.0,
        normal_y: 0.0,
        depth: f64::INFINITY,
    };

    for piece in [left_piece, right_piece] {
        for edge_index in 0..piece.len() {
            let start = piece[edge_index];
            let end = piece[(edge_index + 1) % piece.len()];
            let edge_x = end.x - start.x;
            let edge_y = end.y - start.y;
            let length = edge_x.hypot(edge_y);

            if length <= f64::EPSILON {
                continue;
            }

            let mut normal_x = -edge_y / length;
            let mut normal_y = edge_x / length;
            if normal_x * direction.0 + normal_y * direction.1 < 0.0 {
                normal_x = -normal_x;
                normal_y = -normal_y;
            }

            let (left_min, left_max) = project(left_piece, left, normal_x, normal_y);
            let (right_min, right_max) = project(right_piece, right, normal_x, normal_y);
            let overlap = left_max.min(right_max) - left_min.max(right_min);

            if overlap <= COLLISION_EPSILON {
                return None;
            }
            if overlap < minimum.depth {
                minimum = Collision {
                    normal_x,
                    normal_y,
                    depth: overlap,
                };
            }
        }
    }

    minimum.depth.is_finite().then_some(minimum)
}

fn project(piece: &[Point], body: &CursorBody, axis_x: f64, axis_y: f64) -> (f64, f64) {
    piece
        .iter()
        .map(|point| (point.x + body.x) * axis_x + (point.y + body.y) * axis_y)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
            (min.min(value), max.max(value))
        })
}

fn aabbs_overlap(left: &CursorBody, right: &CursorBody) -> bool {
    left.x < right.x + POINTER_WIDTH
        && left.x + POINTER_WIDTH > right.x
        && left.y < right.y + POINTER_HEIGHT
        && left.y + POINTER_HEIGHT > right.y
}

fn constrain_to_world(body: &mut CursorBody) {
    body.x = body.x.clamp(0.0, WORLD_CONFIG.width - POINTER_WIDTH);
    body.y = body.y.clamp(0.0, WORLD_CONFIG.height - POINTER_HEIGHT);
}

fn separation_direction(left_index: usize, right_index: usize) -> (f64, f64) {
    match (left_index.wrapping_mul(31) ^ right_index.wrapping_mul(17)) % 4 {
        0 => (1.0, 0.0),
        1 => (0.0, 1.0),
        2 => (-1.0, 0.0),
        _ => (0.0, -1.0),
    }
}

fn distance(from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> f64 {
    (to_x - from_x).hypot(to_y - from_y)
}

#[cfg(test)]
mod tests {
    use super::{
        CURSOR_MAX_SPEED, CursorBody, POINTER_HEIGHT, POINTER_WIDTH, pointer_collision,
        simulate_cursors,
    };

    const TICK_SECONDS: f64 = 1.0 / 60.0;

    #[test]
    fn moves_toward_target_at_bounded_speed() {
        let mut bodies = [body_at(100.0, 100.0, 1000.0, 100.0)];

        simulate_cursors(&mut bodies, TICK_SECONDS);

        assert!((bodies[0].x - (100.0 + CURSOR_MAX_SPEED * TICK_SECONDS)).abs() < 0.001);
        assert_eq!(bodies[0].y, 100.0);
    }

    #[test]
    fn prevents_fast_cursors_from_tunneling_through_each_other() {
        let mut bodies = [
            body_at(100.0, 200.0, 500.0, 200.0),
            body_at(220.0, 200.0, 220.0, 200.0),
        ];

        simulate_cursors(&mut bodies, TICK_SECONDS);

        assert!(!overlap(&bodies[0], &bodies[1]));
    }

    #[test]
    fn separates_overlapping_cursors() {
        let mut bodies = [
            body_at(500.0, 500.0, 500.0, 500.0),
            body_at(500.0, 500.0, 500.0, 500.0),
        ];

        simulate_cursors(&mut bodies, TICK_SECONDS);

        assert!(!overlap(&bodies[0], &bodies[1]));
    }

    #[test]
    fn collision_follows_pointer_shape_instead_of_its_bounding_box() {
        let left = body_at(100.0, 100.0, 100.0, 100.0);
        let right = body_at(120.0, 125.0, 120.0, 125.0);

        assert!(super::aabbs_overlap(&left, &right));
        assert!(!overlap(&left, &right));
    }

    #[test]
    fn collides_when_pointer_silhouettes_overlap() {
        let left = body_at(100.0, 100.0, 100.0, 100.0);
        let right = body_at(108.0, 108.0, 108.0, 108.0);

        assert!(overlap(&left, &right));
    }

    #[test]
    fn constrains_pointer_silhouette_to_world_bounds() {
        let mut bodies = [body_at(30.0, 30.0, -100.0, -100.0)];

        simulate_cursors(&mut bodies, TICK_SECONDS);
        assert_eq!(bodies[0].x, 0.0);
        assert_eq!(bodies[0].y, 0.0);

        bodies[0].set_target(super::WORLD_CONFIG.width, super::WORLD_CONFIG.height);
        simulate_cursors(&mut bodies, 1.0);
        assert_eq!(bodies[0].x, super::WORLD_CONFIG.width - POINTER_WIDTH);
        assert_eq!(bodies[0].y, super::WORLD_CONFIG.height - POINTER_HEIGHT);
    }

    #[test]
    fn separates_cursors_that_target_the_same_world_edge() {
        let mut bodies = [
            body_at(0.0, 500.0, 0.0, 500.0),
            body_at(0.0, 500.0, 0.0, 500.0),
        ];

        simulate_cursors(&mut bodies, TICK_SECONDS);

        assert!(!overlap(&bodies[0], &bodies[1]));
    }

    fn overlap(left: &CursorBody, right: &CursorBody) -> bool {
        pointer_collision(left, right, 0, 1).is_some()
    }

    fn body_at(x: f64, y: f64, target_x: f64, target_y: f64) -> CursorBody {
        CursorBody {
            x,
            y,
            target_x,
            target_y,
        }
    }
}

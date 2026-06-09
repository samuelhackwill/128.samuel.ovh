use crate::protocol::WORLD_CONFIG;

pub const CURSOR_RADIUS: f64 = WORLD_CONFIG.cursor_radius;
const CURSOR_MAX_SPEED: f64 = 9000.0;
const MAX_SUBSTEP_DISTANCE: f64 = CURSOR_RADIUS / 2.0;
const COLLISION_ITERATIONS: usize = 6;

#[derive(Clone, Copy, Debug)]
pub struct CursorBody {
    pub x: f64,
    pub y: f64,
    target_x: f64,
    target_y: f64,
}

impl CursorBody {
    pub fn centered() -> Self {
        let x = WORLD_CONFIG.width / 2.0;
        let y = WORLD_CONFIG.height / 2.0;

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
    let minimum_distance = CURSOR_RADIUS * 2.0;

    for left_index in 0..bodies.len() {
        for right_index in (left_index + 1)..bodies.len() {
            let (left, right) = bodies.split_at_mut(right_index);
            let left = &mut left[left_index];
            let right = &mut right[0];
            let dx = right.x - left.x;
            let dy = right.y - left.y;
            let distance_squared = dx * dx + dy * dy;

            if distance_squared >= minimum_distance * minimum_distance {
                continue;
            }

            let (normal_x, normal_y, distance) = if distance_squared > f64::EPSILON {
                let distance = distance_squared.sqrt();
                (dx / distance, dy / distance, distance)
            } else {
                separation_direction(left_index, right_index)
            };
            let correction = (minimum_distance - distance) / 2.0;

            left.x -= normal_x * correction;
            left.y -= normal_y * correction;
            right.x += normal_x * correction;
            right.y += normal_y * correction;
        }
    }
}

fn constrain_to_world(body: &mut CursorBody) {
    body.x = body
        .x
        .clamp(CURSOR_RADIUS, WORLD_CONFIG.width - CURSOR_RADIUS);
    body.y = body
        .y
        .clamp(CURSOR_RADIUS, WORLD_CONFIG.height - CURSOR_RADIUS);
}

fn separation_direction(left_index: usize, right_index: usize) -> (f64, f64, f64) {
    match (left_index.wrapping_mul(31) ^ right_index.wrapping_mul(17)) % 4 {
        0 => (1.0, 0.0, 0.0),
        1 => (0.0, 1.0, 0.0),
        2 => (-1.0, 0.0, 0.0),
        _ => (0.0, -1.0, 0.0),
    }
}

fn distance(from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> f64 {
    (to_x - from_x).hypot(to_y - from_y)
}

#[cfg(test)]
mod tests {
    use super::{CURSOR_MAX_SPEED, CURSOR_RADIUS, CursorBody, simulate_cursors};

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

        assert!(bodies[0].x < bodies[1].x);
        assert!(bodies[1].x - bodies[0].x >= CURSOR_RADIUS * 2.0 - 0.001);
    }

    #[test]
    fn separates_overlapping_cursors() {
        let mut bodies = [
            body_at(500.0, 500.0, 500.0, 500.0),
            body_at(500.0, 500.0, 500.0, 500.0),
        ];

        simulate_cursors(&mut bodies, TICK_SECONDS);

        let distance = (bodies[1].x - bodies[0].x).hypot(bodies[1].y - bodies[0].y);
        assert!(distance >= CURSOR_RADIUS * 2.0 - 0.001);
    }

    #[test]
    fn constrains_cursor_centers_to_world_bounds() {
        let mut bodies = [body_at(30.0, 30.0, 0.0, 0.0)];

        simulate_cursors(&mut bodies, TICK_SECONDS);

        assert_eq!(bodies[0].x, CURSOR_RADIUS);
        assert_eq!(bodies[0].y, CURSOR_RADIUS);
    }

    #[test]
    fn separates_cursors_that_target_the_same_world_edge() {
        let mut bodies = [
            body_at(CURSOR_RADIUS, 500.0, 0.0, 500.0),
            body_at(CURSOR_RADIUS, 500.0, 0.0, 500.0),
        ];

        simulate_cursors(&mut bodies, TICK_SECONDS);

        let distance = (bodies[1].x - bodies[0].x).hypot(bodies[1].y - bodies[0].y);
        assert!(distance >= CURSOR_RADIUS * 2.0 - 0.001);
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

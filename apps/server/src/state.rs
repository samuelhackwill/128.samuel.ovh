use std::collections::HashMap;
use std::time::Instant;

use tokio::sync::RwLock;
use tracing::warn;
use uuid::Uuid;
use wtransport::Connection;

use crate::protocol::{
    ClientEvent, POINTER_HEIGHT, POINTER_WIDTH, PlayerSnapshot, ServerEvent, WORLD_CONFIG,
};
use crate::simulation::{CursorBody, simulate_cursors};

#[derive(Debug)]
struct Player {
    id: Uuid,
    number: u16,
    supports_datagrams: bool,
    body: CursorBody,
    last_sequence: u64,
}

pub struct SharedState {
    started_at: Instant,
    tick: RwLock<u64>,
    players: RwLock<HashMap<Uuid, Player>>,
    connections: RwLock<HashMap<Uuid, Connection>>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            started_at: Instant::now(),
            tick: RwLock::new(0),
            players: RwLock::new(HashMap::new()),
            connections: RwLock::new(HashMap::new()),
        }
    }
}

impl SharedState {
    pub async fn add_player(&self, id: Uuid, requested_number: Option<u16>) -> u16 {
        let mut players = self.players.write().await;
        let number = available_player_number(&players, requested_number);
        players.insert(
            id,
            Player {
                id,
                number,
                supports_datagrams: false,
                body: CursorBody::centered(),
                last_sequence: 0,
            },
        );
        number
    }

    pub async fn add_connection(&self, id: Uuid, connection: Connection) {
        self.connections.write().await.insert(id, connection);
    }

    pub async fn remove_player(&self, id: Uuid) {
        self.players.write().await.remove(&id);
        self.connections.write().await.remove(&id);
        self.broadcast_reliable(&ServerEvent::PlayerLeft { player_id: id })
            .await;
    }

    pub async fn apply_event(&self, id: Uuid, event: ClientEvent) {
        let mut players = self.players.write().await;
        let Some(player) = players.get_mut(&id) else {
            return;
        };

        match event {
            ClientEvent::TransportCapabilities { datagrams } => {
                player.supports_datagrams = datagrams;
            }
            ClientEvent::PointerMove {
                sequence,
                client_time,
                x,
                y,
            } => {
                let _ = client_time;
                if sequence >= player.last_sequence
                    && let Some((x, y)) = clamp_to_world(x, y)
                {
                    player.last_sequence = sequence;
                    player.body.set_target(x, y);
                }
            }
            ClientEvent::PointerButton {
                button,
                pressed,
                sequence,
                client_time,
            } => {
                let _ = (button, pressed, sequence, client_time);
                // Mini-games will handle button events here.
            }
        }
    }

    pub async fn advance_simulation(&self, delta_seconds: f64) -> u64 {
        let mut players = self.players.write().await;
        let ids: Vec<_> = players.keys().copied().collect();
        let mut bodies: Vec<_> = ids.iter().map(|id| players[id].body).collect();

        simulate_cursors(&mut bodies, delta_seconds);
        for (id, body) in ids.into_iter().zip(bodies) {
            if let Some(player) = players.get_mut(&id) {
                player.body = body;
            }
        }
        drop(players);

        let mut tick = self.tick.write().await;
        *tick += 1;
        *tick
    }

    pub async fn broadcast_snapshot(&self) {
        let tick = *self.tick.read().await;
        let players = self
            .players
            .read()
            .await
            .values()
            .map(|player| PlayerSnapshot {
                id: player.id,
                number: player.number,
                x: player.body.x,
                y: player.body.y,
            })
            .collect();
        let event = ServerEvent::State {
            server_time: self.started_at.elapsed().as_millis() as u64,
            tick,
            players,
        };
        let Ok(payload) = serde_json::to_vec(&event) else {
            warn!("failed to serialize state snapshot");
            return;
        };

        let players = self.players.read().await;
        let connections: Vec<_> = self
            .connections
            .read()
            .await
            .iter()
            .filter_map(|(id, connection)| {
                players
                    .get(id)
                    .map(|player| (connection.clone(), player.supports_datagrams))
            })
            .collect();

        for (connection, supports_datagrams) in connections {
            if supports_datagrams {
                if let Err(error) = connection.send_datagram(&payload) {
                    warn!(?error, "failed to send state datagram");
                }
            } else {
                let payload = payload.clone();
                tokio::spawn(async move {
                    if let Err(error) = send_reliable(&connection, &payload).await {
                        warn!(?error, "failed to send reliable state snapshot");
                    }
                });
            }
        }
    }

    pub async fn broadcast_reliable(&self, event: &ServerEvent) {
        let connections: Vec<_> = self.connections.read().await.values().cloned().collect();
        for connection in connections {
            let event = serde_json::to_vec(event);
            tokio::spawn(async move {
                let Ok(payload) = event else {
                    return;
                };
                if let Err(error) = send_reliable(&connection, &payload).await {
                    warn!(?error, "failed to send reliable event");
                }
            });
        }
    }
}

fn available_player_number(players: &HashMap<Uuid, Player>, requested: Option<u16>) -> u16 {
    if let Some(number) = requested
        .filter(|number| *number > 0 && players.values().all(|player| player.number != *number))
    {
        return number;
    }

    (1..=u16::MAX)
        .find(|number| players.values().all(|player| player.number != *number))
        .expect("all player numbers are occupied")
}

pub async fn send_reliable(connection: &Connection, payload: &[u8]) -> anyhow::Result<()> {
    let mut stream = connection.open_uni().await?.await?;
    stream.write_all(payload).await?;
    stream.finish().await?;
    Ok(())
}

fn clamp_to_world(x: f64, y: f64) -> Option<(f64, f64)> {
    if !x.is_finite() || !y.is_finite() {
        return None;
    }

    Some((
        x.clamp(0.0, WORLD_CONFIG.width - POINTER_WIDTH),
        y.clamp(0.0, WORLD_CONFIG.height - POINTER_HEIGHT),
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Player, available_player_number, clamp_to_world};
    use crate::simulation::CursorBody;
    use uuid::Uuid;

    #[test]
    fn clamps_pointer_positions_to_the_world() {
        assert_eq!(clamp_to_world(-10.0, 2000.0), Some((0.0, 1049.0)));
        assert_eq!(clamp_to_world(960.0, 540.0), Some((960.0, 540.0)));
    }

    #[test]
    fn rejects_non_finite_pointer_positions() {
        assert_eq!(clamp_to_world(f64::NAN, 0.0), None);
        assert_eq!(clamp_to_world(0.0, f64::INFINITY), None);
    }

    #[test]
    fn uses_an_available_requested_player_number() {
        let players = players_with_numbers(&[1, 3]);

        assert_eq!(available_player_number(&players, Some(12)), 12);
    }

    #[test]
    fn replaces_missing_invalid_or_duplicate_player_numbers() {
        let players = players_with_numbers(&[1, 3]);

        assert_eq!(available_player_number(&players, None), 2);
        assert_eq!(available_player_number(&players, Some(0)), 2);
        assert_eq!(available_player_number(&players, Some(3)), 2);
    }

    fn players_with_numbers(numbers: &[u16]) -> HashMap<Uuid, Player> {
        numbers
            .iter()
            .map(|number| {
                let id = Uuid::new_v4();
                (
                    id,
                    Player {
                        id,
                        number: *number,
                        supports_datagrams: false,
                        body: CursorBody::centered(),
                        last_sequence: 0,
                    },
                )
            })
            .collect()
    }
}

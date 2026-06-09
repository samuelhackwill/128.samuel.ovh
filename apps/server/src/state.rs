use std::collections::HashMap;
use std::time::Instant;

use tokio::sync::RwLock;
use tracing::warn;
use uuid::Uuid;
use wtransport::Connection;

use crate::protocol::{ClientEvent, PlayerSnapshot, ServerEvent};

#[derive(Debug)]
struct Player {
    id: Uuid,
    supports_datagrams: bool,
    x: f64,
    y: f64,
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
    pub async fn add_player(&self, id: Uuid, connection: Connection) {
        self.players.write().await.insert(
            id,
            Player {
                id,
                supports_datagrams: false,
                x: 0.0,
                y: 0.0,
                last_sequence: 0,
            },
        );
        self.connections.write().await.insert(id, connection);
        self.broadcast_reliable(&ServerEvent::PlayerJoined { player_id: id })
            .await;
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
                if sequence >= player.last_sequence {
                    player.last_sequence = sequence;
                    player.x = x;
                    player.y = y;
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

    pub async fn advance_tick(&self) -> u64 {
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
                x: player.x,
                y: player.y,
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

pub async fn send_reliable(connection: &Connection, payload: &[u8]) -> anyhow::Result<()> {
    let mut stream = connection.open_uni().await?.await?;
    stream.write_all(payload).await?;
    stream.finish().await?;
    Ok(())
}

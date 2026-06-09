use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ClientEvent {
    TransportCapabilities {
        datagrams: bool,
    },
    PointerMove {
        sequence: u64,
        #[serde(rename = "clientTime")]
        client_time: f64,
        x: f64,
        y: f64,
    },
    PointerButton {
        button: u16,
        pressed: bool,
        sequence: u64,
        #[serde(rename = "clientTime")]
        client_time: f64,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ServerEvent {
    Connected {
        #[serde(rename = "playerId")]
        player_id: Uuid,
    },
    PlayerJoined {
        #[serde(rename = "playerId")]
        player_id: Uuid,
    },
    PlayerLeft {
        #[serde(rename = "playerId")]
        player_id: Uuid,
    },
    State {
        #[serde(rename = "serverTime")]
        server_time: u64,
        tick: u64,
        players: Vec<PlayerSnapshot>,
    },
}

#[derive(Debug, Serialize)]
pub struct PlayerSnapshot {
    pub id: Uuid,
    pub x: f64,
    pub y: f64,
}

#[cfg(test)]
mod tests {
    use super::{ClientEvent, ServerEvent};
    use uuid::Uuid;

    #[test]
    fn server_events_match_the_typescript_wire_format() {
        let event = ServerEvent::Connected {
            player_id: Uuid::nil(),
        };

        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"type":"connected","playerId":"00000000-0000-0000-0000-000000000000"}"#
        );
    }

    #[test]
    fn client_events_match_the_typescript_wire_format() {
        let event: ClientEvent = serde_json::from_str(
            r#"{"type":"pointer-move","sequence":3,"clientTime":12.5,"x":100,"y":200}"#,
        )
        .unwrap();

        assert!(matches!(
            event,
            ClientEvent::PointerMove {
                sequence: 3,
                client_time: 12.5,
                x: 100.0,
                y: 200.0,
            }
        ));
    }

    #[test]
    fn transport_capability_events_match_the_typescript_wire_format() {
        let event: ClientEvent =
            serde_json::from_str(r#"{"type":"transport-capabilities","datagrams":false}"#).unwrap();

        assert!(matches!(
            event,
            ClientEvent::TransportCapabilities { datagrams: false }
        ));
    }
}

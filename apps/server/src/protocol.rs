use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const POINTER_WIDTH: f64 = 23.0;
pub const POINTER_HEIGHT: f64 = 31.0;
pub const POINTER_POINTS: [Point; 7] = [
    Point { x: 0.0, y: 0.0 },
    Point { x: 0.0, y: 26.0 },
    Point { x: 7.0, y: 19.0 },
    Point { x: 12.0, y: 31.0 },
    Point { x: 18.0, y: 28.0 },
    Point { x: 13.0, y: 17.0 },
    Point { x: 23.0, y: 17.0 },
];

pub const WORLD_CONFIG: WorldConfig = WorldConfig {
    width: 1920.0,
    height: 1080.0,
    pointer: PointerConfig {
        width: POINTER_WIDTH,
        height: POINTER_HEIGHT,
        points: &POINTER_POINTS,
    },
};

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
        #[serde(rename = "playerNumber")]
        player_number: u16,
        world: WorldConfig,
    },
    PlayerJoined {
        #[serde(rename = "playerId")]
        player_id: Uuid,
        #[serde(rename = "playerNumber")]
        player_number: u16,
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

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldConfig {
    pub width: f64,
    pub height: f64,
    pub pointer: PointerConfig,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PointerConfig {
    pub width: f64,
    pub height: f64,
    pub points: &'static [Point],
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Serialize)]
pub struct PlayerSnapshot {
    pub id: Uuid,
    pub number: u16,
    pub x: f64,
    pub y: f64,
}

#[cfg(test)]
mod tests {
    use super::{ClientEvent, ServerEvent, WORLD_CONFIG};
    use uuid::Uuid;

    #[test]
    fn server_events_match_the_typescript_wire_format() {
        let event = ServerEvent::Connected {
            player_id: Uuid::nil(),
            player_number: 12,
            world: WORLD_CONFIG,
        };

        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"type":"connected","playerId":"00000000-0000-0000-0000-000000000000","playerNumber":12,"world":{"width":1920.0,"height":1080.0,"pointer":{"width":23.0,"height":31.0,"points":[{"x":0.0,"y":0.0},{"x":0.0,"y":26.0},{"x":7.0,"y":19.0},{"x":12.0,"y":31.0},{"x":18.0,"y":28.0},{"x":13.0,"y":17.0},{"x":23.0,"y":17.0}]}}}"#
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

use serde::{Serialize, Deserialize};
use crate::command::Command;

/// Unique match identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MatchId(pub String);

/// Unique player identifier within a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub u8);

/// Player info sent in lobby messages.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: PlayerId,
    pub name: String,
    pub is_ai: bool,
    pub is_ready: bool,
}

/// Match configuration sent when creating a lobby.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchConfig {
    pub map_width: u32,
    pub map_height: u32,
    pub player_count: u32,
    pub seed: u32,
    pub tick_rate_ms: u32,
}

impl Default for MatchConfig {
    fn default() -> Self {
        MatchConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
            tick_rate_ms: 50,
        }
    }
}

/// Entity snapshot for network transmission (fog-filtered).
/// Replaces the raw 32-byte render buffer entry with a structured type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub entity_id: u32,
    pub x: f32,
    pub y: f32,
    pub sprite_id: u16,
    pub frame: u16,
    pub health_pct: u8,
    pub facing: u8,
    pub owner: u8,
    pub flags: u8,
    pub scale: f32,
}

/// Event snapshot for network transmission.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventSnapshot {
    pub event_type: u16,
    pub entity_id: u32,
    pub x: f32,
    pub y: f32,
    pub payload: [u8; 16],
}

/// Economy state for a specific player.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EconomySnapshot {
    pub energy_bank: f32,
    pub income: f32,
    pub expense: f32,
    pub strain: f32,
    pub strain_income_penalty: f32,
}

/// Production line snapshot.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionLineSnapshot {
    pub unit_type: Option<u16>,
    pub progress: f32,
    pub total_time: f32,
}

/// Capture point snapshot for network transmission.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapturePointSnapshot {
    pub entity_id: u32,
    pub x: f32,
    pub y: f32,
    pub point_index: u8,
    pub owner: u8,
    pub progress: f32,
    pub capturing_player: u8,
    pub contested: bool,
}

/// Client -> Server messages.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Join a lobby.
    Join {
        lobby_id: String,
        player_name: String,
    },
    /// Signal ready to start.
    Ready,
    /// Send a game command.
    Cmd {
        cmd: Command,
    },
    /// Ping for latency measurement.
    Ping {
        seq: u32,
    },
}

/// Server -> Client messages.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Incremental state update (every tick).
    State {
        tick: u32,
        entities: Vec<EntitySnapshot>,
        events: Vec<EventSnapshot>,
        fog: Vec<u8>,
        economy: EconomySnapshot,
        production: Vec<ProductionLineSnapshot>,
        capture_points: Vec<CapturePointSnapshot>,
    },
    /// Full state on join/reconnect.
    FullState {
        tick: u32,
        entities: Vec<EntitySnapshot>,
        fog: Vec<u8>,
        economy: EconomySnapshot,
        production: Vec<ProductionLineSnapshot>,
        capture_points: Vec<CapturePointSnapshot>,
        map_width: u32,
        map_height: u32,
        map_tiles: Vec<u8>,
    },
    /// Pong response.
    Pong {
        seq: u32,
        server_tick: u32,
    },
    /// Lobby state update.
    Lobby {
        players: Vec<PlayerInfo>,
        status: String,
    },
    /// Match ended.
    End {
        winner: u8,
        reason: String,
    },
    /// Error message.
    Error {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_roundtrip() {
        let msgs = vec![
            ClientMessage::Join {
                lobby_id: "test-lobby".into(),
                player_name: "Player1".into(),
            },
            ClientMessage::Ready,
            ClientMessage::Cmd {
                cmd: Command::Move {
                    unit_ids: vec![0, 1, 2],
                    target_x: 10.5,
                    target_y: 20.5,
                },
            },
            ClientMessage::Ping { seq: 42 },
        ];

        for msg in &msgs {
            let bytes = rmp_serde::to_vec(msg).expect("serialize");
            let decoded: ClientMessage = rmp_serde::from_slice(&bytes).expect("deserialize");
            // Verify roundtrip by re-serializing
            let bytes2 = rmp_serde::to_vec(&decoded).expect("re-serialize");
            assert_eq!(bytes, bytes2);
        }
    }

    #[test]
    fn test_server_message_roundtrip() {
        let state_msg = ServerMessage::State {
            tick: 100,
            entities: vec![
                EntitySnapshot {
                    entity_id: 1,
                    x: 5.5,
                    y: 10.5,
                    sprite_id: 0,
                    frame: 2,
                    health_pct: 80,
                    facing: 3,
                    owner: 0,
                    flags: 0,
                    scale: 0.09375,
                },
            ],
            events: vec![
                EventSnapshot {
                    event_type: 0,
                    entity_id: 1,
                    x: 5.5,
                    y: 10.5,
                    payload: [0u8; 16],
                },
            ],
            fog: vec![0, 1, 2, 2, 1, 0],
            economy: EconomySnapshot {
                energy_bank: 500.0,
                income: 5.0,
                expense: 1.5,
                strain: 10.0,
                strain_income_penalty: 0.0,
            },
            production: vec![
                ProductionLineSnapshot {
                    unit_type: Some(0),
                    progress: 2.5,
                    total_time: 5.0,
                },
            ],
            capture_points: vec![
                CapturePointSnapshot {
                    entity_id: 100,
                    x: 32.5,
                    y: 32.5,
                    point_index: 0,
                    owner: 255,
                    progress: 50.0,
                    capturing_player: 0,
                    contested: false,
                },
            ],
        };

        let bytes = rmp_serde::to_vec(&state_msg).expect("serialize");
        let decoded: ServerMessage = rmp_serde::from_slice(&bytes).expect("deserialize");
        let bytes2 = rmp_serde::to_vec(&decoded).expect("re-serialize");
        assert_eq!(bytes, bytes2);
    }

    #[test]
    fn test_full_state_message_roundtrip() {
        let msg = ServerMessage::FullState {
            tick: 0,
            entities: vec![],
            fog: vec![0; 64 * 64],
            economy: EconomySnapshot {
                energy_bank: 500.0,
                income: 5.0,
                expense: 0.0,
                strain: 0.0,
                strain_income_penalty: 0.0,
            },
            production: vec![],
            capture_points: vec![],
            map_width: 64,
            map_height: 64,
            map_tiles: vec![0; 64 * 64],
        };

        let bytes = rmp_serde::to_vec(&msg).expect("serialize");
        let decoded: ServerMessage = rmp_serde::from_slice(&bytes).expect("deserialize");
        let bytes2 = rmp_serde::to_vec(&decoded).expect("re-serialize");
        assert_eq!(bytes, bytes2);
    }

    #[test]
    fn test_command_serialization_roundtrip() {
        let commands = vec![
            Command::Move { unit_ids: vec![0, 1], target_x: 10.0, target_y: 20.0 },
            Command::Stop { unit_ids: vec![3] },
            Command::Attack { unit_ids: vec![4, 5], target_id: 6 },
            Command::AttackMove { unit_ids: vec![7], target_x: 15.0, target_y: 25.0 },
            Command::Build { player: 0, building_type: 4, tile_x: 10, tile_y: 10 },
            Command::Produce { player: 0, unit_type: 0 },
            Command::CancelProduction { player: 0, line_index: 1 },
            Command::SetRally { player: 0, x: 30.0, y: 30.0 },
            Command::Deploy { player: 0, cp_x: 8.0, cp_y: 8.0 },
            Command::ConfirmDeployment { player: 0 },
            Command::UpgradeForge { player: 0, upgrade: 1 },
            Command::CampaignResearch { player: 0, tech_id: 2 },
            Command::CampaignDispatch { player: 0, source_site: 0, target_site: 1, units: vec![(0, 5)] },
            Command::CampaignWithdraw { player: 0, site_id: 3 },
        ];

        for cmd in &commands {
            let bytes = rmp_serde::to_vec(cmd).expect("serialize");
            let decoded: Command = rmp_serde::from_slice(&bytes).expect("deserialize");
            let bytes2 = rmp_serde::to_vec(&decoded).expect("re-serialize");
            assert_eq!(bytes, bytes2, "Roundtrip failed for: {:?}", cmd);
        }
    }

    #[test]
    fn test_match_config_default() {
        let config = MatchConfig::default();
        assert_eq!(config.map_width, 64);
        assert_eq!(config.map_height, 64);
        assert_eq!(config.player_count, 2);
        assert_eq!(config.tick_rate_ms, 50);
    }

    #[test]
    fn test_pong_message_roundtrip() {
        let msg = ServerMessage::Pong { seq: 42, server_tick: 100 };
        let bytes = rmp_serde::to_vec(&msg).expect("serialize");
        let decoded: ServerMessage = rmp_serde::from_slice(&bytes).expect("deserialize");
        let bytes2 = rmp_serde::to_vec(&decoded).expect("re-serialize");
        assert_eq!(bytes, bytes2);
    }

    #[test]
    fn test_lobby_message_roundtrip() {
        let msg = ServerMessage::Lobby {
            players: vec![
                PlayerInfo {
                    id: PlayerId(0),
                    name: "Player1".into(),
                    is_ai: false,
                    is_ready: true,
                },
                PlayerInfo {
                    id: PlayerId(1),
                    name: "AI Bot".into(),
                    is_ai: true,
                    is_ready: true,
                },
            ],
            status: "Waiting".into(),
        };
        let bytes = rmp_serde::to_vec(&msg).expect("serialize");
        let decoded: ServerMessage = rmp_serde::from_slice(&bytes).expect("deserialize");
        let bytes2 = rmp_serde::to_vec(&decoded).expect("re-serialize");
        assert_eq!(bytes, bytes2);
    }
}

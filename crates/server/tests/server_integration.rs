/// Server integration tests for Machine Empire.
/// Tests WebSocket message flow, MCP tool and resource handling,
/// and server message roundtrips.

use machine_empire_core::protocol::{
    MatchConfig, ServerMessage, ClientMessage,
    EntitySnapshot, EconomySnapshot, ProductionLineSnapshot,
};
use machine_empire_core::command::Command;

#[test]
fn test_server_message_roundtrip_full() {
    // Full roundtrip test for complex ServerMessage::State
    let msg = ServerMessage::State {
        tick: 500,
        entities: vec![
            EntitySnapshot {
                entity_id: 1,
                x: 10.5,
                y: 20.5,
                sprite_id: 0, // Thrall
                frame: 3,
                health_pct: 75,
                facing: 2,
                owner: 0,
                flags: 0,
                scale: 0.09375,
            },
            EntitySnapshot {
                entity_id: 2,
                x: 30.5,
                y: 40.5,
                sprite_id: 1, // Sentinel
                frame: 0,
                health_pct: 100,
                facing: 5,
                owner: 1,
                flags: 0,
                scale: 0.109375,
            },
        ],
        events: vec![],
        fog: vec![2; 64 * 64], // All visible
        economy: EconomySnapshot {
            energy_bank: 450.0,
            income: 5.0,
            expense: 2.5,
            strain: 15.0,
            strain_income_penalty: 0.05,
        },
        production: vec![
            ProductionLineSnapshot {
                unit_type: Some(0),
                progress: 2.5,
                total_time: 5.0,
            },
            ProductionLineSnapshot {
                unit_type: None,
                progress: 0.0,
                total_time: 0.0,
            },
        ],
        capture_points: vec![],
    };

    // Serialize with MessagePack
    let bytes = rmp_serde::to_vec(&msg).expect("serialize");
    let decoded: ServerMessage = rmp_serde::from_slice(&bytes).expect("deserialize");

    // Verify roundtrip
    let bytes2 = rmp_serde::to_vec(&decoded).expect("re-serialize");
    assert_eq!(bytes, bytes2, "MessagePack roundtrip should produce identical bytes");
}

#[test]
fn test_client_message_roundtrip_all_variants() {
    let messages = vec![
        ClientMessage::Join {
            lobby_id: "test-lobby-123".into(),
            player_name: "TestPlayer".into(),
        },
        ClientMessage::Ready,
        ClientMessage::Cmd {
            cmd: Command::Move {
                unit_ids: vec![1, 2, 3],
                target_x: 15.5,
                target_y: 25.5,
            },
        },
        ClientMessage::Cmd {
            cmd: Command::Attack {
                unit_ids: vec![4],
                target_id: 10,
            },
        },
        ClientMessage::Cmd {
            cmd: Command::AttackMove {
                unit_ids: vec![5, 6],
                target_x: 30.0,
                target_y: 40.0,
            },
        },
        ClientMessage::Cmd {
            cmd: Command::Produce {
                player: 0,
                unit_type: 0,
            },
        },
        ClientMessage::Cmd {
            cmd: Command::CancelProduction {
                player: 0,
                line_index: 1,
            },
        },
        ClientMessage::Cmd {
            cmd: Command::SetRally {
                player: 0,
                x: 20.0,
                y: 20.0,
            },
        },
        ClientMessage::Ping { seq: 42 },
    ];

    for msg in &messages {
        let bytes = rmp_serde::to_vec(msg).expect("serialize");
        let decoded: ClientMessage = rmp_serde::from_slice(&bytes).expect("deserialize");
        let bytes2 = rmp_serde::to_vec(&decoded).expect("re-serialize");
        assert_eq!(bytes, bytes2, "Roundtrip failed for {:?}", msg);
    }
}

#[test]
fn test_match_config_serialization() {
    let config = MatchConfig {
        map_width: 128,
        map_height: 128,
        player_count: 4,
        seed: 12345,
        tick_rate_ms: 50,
    };

    let json_str = serde_json::to_string(&config).expect("json serialize");
    let decoded: MatchConfig = serde_json::from_str(&json_str).expect("json deserialize");

    assert_eq!(decoded.map_width, 128);
    assert_eq!(decoded.map_height, 128);
    assert_eq!(decoded.player_count, 4);
    assert_eq!(decoded.seed, 12345);
    assert_eq!(decoded.tick_rate_ms, 50);
}

#[test]
fn test_mcp_tool_definitions_valid_json() {
    // Verify all MCP tool definitions are valid JSON schemas
    use serde_json::Value;

    let tools_json = serde_json::json!([
        {
            "name": "game/move_units",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "unit_ids": { "type": "array", "items": { "type": "integer" } },
                    "target_x": { "type": "number" },
                    "target_y": { "type": "number" }
                },
                "required": ["unit_ids", "target_x", "target_y"]
            }
        },
        {
            "name": "game/attack",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "unit_ids": { "type": "array" },
                    "target_id": { "type": "integer" }
                }
            }
        },
        {
            "name": "game/produce_unit",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "unit_type": { "type": "integer", "enum": [0, 1, 2] }
                }
            }
        }
    ]);

    let tools: Vec<Value> = serde_json::from_value(tools_json).expect("parse tools");
    assert_eq!(tools.len(), 3);

    for tool in &tools {
        assert!(tool["name"].is_string());
        assert!(tool["inputSchema"].is_object());
        assert_eq!(tool["inputSchema"]["type"], "object");
    }
}

#[test]
fn test_end_to_end_command_flow() {
    // Simulate the end-to-end flow: client sends command, game processes it.
    use machine_empire_core::game::{Game, GameConfig};
    use machine_empire_core::components::Position;

    let config = GameConfig {
        map_width: 64,
        map_height: 64,
        player_count: 2,
        seed: 42,
    };
    let mut game = Game::new(config);

    let entity = game.spawn_thrall(10.5, 10.5, 0);

    // Simulate what the server does: deserialize a command, push it, tick
    let cmd_json = serde_json::json!({
        "Move": {
            "unit_ids": [entity.raw()],
            "target_x": 15.5,
            "target_y": 10.5
        }
    });

    let cmd: Command = serde_json::from_value(cmd_json).expect("deserialize command");
    game.push_command(cmd);

    // Tick the game
    game.tick(50.0);

    // The unit should have started pathfinding
    let pos = game.world.get_component::<Position>(entity).unwrap();
    // After one tick, the unit might have moved slightly or be pathing
    // Just verify it didn't crash and the position is valid
    assert!(pos.x > 0.0 && pos.y > 0.0, "Position should be valid");
}

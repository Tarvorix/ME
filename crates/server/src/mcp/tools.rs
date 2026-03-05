use serde_json::{json, Value};
use machine_empire_core::command::Command;
use super::types::{ToolDefinition, INVALID_PARAMS};

/// Get all tool definitions.
pub fn list_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "game/move_units".into(),
            description: "Move selected units to a target position on the map.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "unit_ids": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "description": "Entity IDs of units to move"
                    },
                    "target_x": {
                        "type": "number",
                        "description": "Target X position (tile coordinate)"
                    },
                    "target_y": {
                        "type": "number",
                        "description": "Target Y position (tile coordinate)"
                    }
                },
                "required": ["unit_ids", "target_x", "target_y"]
            }),
        },
        ToolDefinition {
            name: "game/attack".into(),
            description: "Order units to attack a specific enemy entity.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "unit_ids": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "description": "Entity IDs of attacking units"
                    },
                    "target_id": {
                        "type": "integer",
                        "description": "Entity ID of the target to attack"
                    }
                },
                "required": ["unit_ids", "target_id"]
            }),
        },
        ToolDefinition {
            name: "game/attack_move".into(),
            description: "Order units to attack-move toward a position, engaging enemies along the way.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "unit_ids": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "description": "Entity IDs of units to attack-move"
                    },
                    "target_x": {
                        "type": "number",
                        "description": "Target X position"
                    },
                    "target_y": {
                        "type": "number",
                        "description": "Target Y position"
                    }
                },
                "required": ["unit_ids", "target_x", "target_y"]
            }),
        },
        ToolDefinition {
            name: "game/produce_unit".into(),
            description: "Queue production of a unit type. Types: 0=Thrall, 1=Sentinel, 2=HoverTank.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "unit_type": {
                        "type": "integer",
                        "description": "Unit type to produce: 0=Thrall, 1=Sentinel, 2=HoverTank",
                        "enum": [0, 1, 2]
                    }
                },
                "required": ["unit_type"]
            }),
        },
        ToolDefinition {
            name: "game/cancel_production".into(),
            description: "Cancel production on a specific production line.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "line_index": {
                        "type": "integer",
                        "description": "Index of the production line to cancel (0=infantry, 1=armor)"
                    }
                },
                "required": ["line_index"]
            }),
        },
        ToolDefinition {
            name: "game/set_rally_point".into(),
            description: "Set the rally point where newly produced units will move to.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "number",
                        "description": "Rally point X position"
                    },
                    "y": {
                        "type": "number",
                        "description": "Rally point Y position"
                    }
                },
                "required": ["x", "y"]
            }),
        },
        ToolDefinition {
            name: "game/get_suggestions".into(),
            description: "Get AI-powered strategic suggestions for the current game state.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    ]
}

/// Execute a tool call and return the result.
/// Returns Ok((command, description)) or Err((error_code, message)).
pub fn execute_tool(
    tool_name: &str,
    params: &Value,
    player_id: u8,
) -> Result<(Option<Command>, String), (i32, String)> {
    match tool_name {
        "game/move_units" => {
            let unit_ids = extract_unit_ids(params)?;
            let target_x = extract_f32(params, "target_x")?;
            let target_y = extract_f32(params, "target_y")?;

            let cmd = Command::Move {
                unit_ids: unit_ids.clone(),
                target_x,
                target_y,
            };

            Ok((Some(cmd), format!("Moving {} units to ({:.1}, {:.1})", unit_ids.len(), target_x, target_y)))
        }

        "game/attack" => {
            let unit_ids = extract_unit_ids(params)?;
            let target_id = extract_u32(params, "target_id")?;

            let cmd = Command::Attack {
                unit_ids: unit_ids.clone(),
                target_id,
            };

            Ok((Some(cmd), format!("Attacking target {} with {} units", target_id, unit_ids.len())))
        }

        "game/attack_move" => {
            let unit_ids = extract_unit_ids(params)?;
            let target_x = extract_f32(params, "target_x")?;
            let target_y = extract_f32(params, "target_y")?;

            let cmd = Command::AttackMove {
                unit_ids: unit_ids.clone(),
                target_x,
                target_y,
            };

            Ok((Some(cmd), format!("Attack-moving {} units to ({:.1}, {:.1})", unit_ids.len(), target_x, target_y)))
        }

        "game/produce_unit" => {
            let unit_type = extract_u32(params, "unit_type")? as u16;

            // Validate unit type
            let type_name = match unit_type {
                0 => "Thrall",
                1 => "Sentinel",
                2 => "HoverTank",
                _ => return Err((INVALID_PARAMS, format!("Invalid unit_type: {}. Must be 0 (Thrall), 1 (Sentinel), or 2 (HoverTank)", unit_type))),
            };

            let cmd = Command::Produce {
                player: player_id,
                unit_type,
            };

            Ok((Some(cmd), format!("Queued production of {}", type_name)))
        }

        "game/cancel_production" => {
            let line_index = extract_u32(params, "line_index")? as u8;

            let cmd = Command::CancelProduction {
                player: player_id,
                line_index,
            };

            Ok((Some(cmd), format!("Cancelled production on line {}", line_index)))
        }

        "game/set_rally_point" => {
            let x = extract_f32(params, "x")?;
            let y = extract_f32(params, "y")?;

            let cmd = Command::SetRally {
                player: player_id,
                x,
                y,
            };

            Ok((Some(cmd), format!("Set rally point to ({:.1}, {:.1})", x, y)))
        }

        "game/get_suggestions" => {
            // Return strategic suggestions based on current state
            // This doesn't generate a command, just analysis
            Ok((None, "Use game://state/threats resource to analyze threats, then decide: produce units if economy is strong, attack weak sectors, defend your base if under pressure.".into()))
        }

        _ => Err((super::types::METHOD_NOT_FOUND, format!("Unknown tool: {}", tool_name))),
    }
}

/// Extract unit_ids array from params.
fn extract_unit_ids(params: &Value) -> Result<Vec<u32>, (i32, String)> {
    let arr = params.get("unit_ids")
        .and_then(|v| v.as_array())
        .ok_or_else(|| (INVALID_PARAMS, "Missing or invalid 'unit_ids' array".into()))?;

    let ids: Result<Vec<u32>, _> = arr.iter().map(|v| {
        v.as_u64()
            .map(|n| n as u32)
            .ok_or_else(|| (INVALID_PARAMS, "unit_ids must contain integers".to_string()))
    }).collect();

    ids
}

/// Extract a f32 value from params by key.
fn extract_f32(params: &Value, key: &str) -> Result<f32, (i32, String)> {
    params.get(key)
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .ok_or_else(|| (INVALID_PARAMS, format!("Missing or invalid '{}' number", key)))
}

/// Extract a u32 value from params by key.
fn extract_u32(params: &Value, key: &str) -> Result<u32, (i32, String)> {
    params.get(key)
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .ok_or_else(|| (INVALID_PARAMS, format!("Missing or invalid '{}' integer", key)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_list_complete() {
        let tools = list_tools();
        assert_eq!(tools.len(), 7, "Should have 7 tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"game/move_units"));
        assert!(names.contains(&"game/attack"));
        assert!(names.contains(&"game/attack_move"));
        assert!(names.contains(&"game/produce_unit"));
        assert!(names.contains(&"game/cancel_production"));
        assert!(names.contains(&"game/set_rally_point"));
        assert!(names.contains(&"game/get_suggestions"));
    }

    #[test]
    fn test_move_tool_execution() {
        let params = json!({
            "unit_ids": [1, 2, 3],
            "target_x": 10.5,
            "target_y": 20.5
        });

        let result = execute_tool("game/move_units", &params, 0);
        assert!(result.is_ok());

        let (cmd, desc) = result.unwrap();
        assert!(cmd.is_some());
        assert!(desc.contains("Moving 3 units"));

        match cmd.unwrap() {
            Command::Move { unit_ids, target_x, target_y } => {
                assert_eq!(unit_ids, vec![1, 2, 3]);
                assert!((target_x - 10.5).abs() < 0.01);
                assert!((target_y - 20.5).abs() < 0.01);
            }
            _ => panic!("Expected Move command"),
        }
    }

    #[test]
    fn test_produce_tool_execution() {
        let params = json!({ "unit_type": 0 });
        let result = execute_tool("game/produce_unit", &params, 1);
        assert!(result.is_ok());

        let (cmd, desc) = result.unwrap();
        assert!(desc.contains("Thrall"));

        match cmd.unwrap() {
            Command::Produce { player, unit_type } => {
                assert_eq!(player, 1);
                assert_eq!(unit_type, 0); // Thrall
            }
            _ => panic!("Expected Produce command"),
        }
    }

    #[test]
    fn test_produce_invalid_unit_type() {
        let params = json!({ "unit_type": 99 });
        let result = execute_tool("game/produce_unit", &params, 0);
        assert!(result.is_err());
        let (code, _msg) = result.unwrap_err();
        assert_eq!(code, INVALID_PARAMS);
    }

    #[test]
    fn test_attack_tool_execution() {
        let params = json!({
            "unit_ids": [5],
            "target_id": 10
        });

        let result = execute_tool("game/attack", &params, 0);
        assert!(result.is_ok());

        let (cmd, _) = result.unwrap();
        match cmd.unwrap() {
            Command::Attack { unit_ids, target_id } => {
                assert_eq!(unit_ids, vec![5]);
                assert_eq!(target_id, 10);
            }
            _ => panic!("Expected Attack command"),
        }
    }

    #[test]
    fn test_unknown_tool() {
        let result = execute_tool("game/nonexistent", &json!({}), 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_params() {
        let result = execute_tool("game/move_units", &json!({}), 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_suggestions_tool() {
        let result = execute_tool("game/get_suggestions", &json!({}), 0);
        assert!(result.is_ok());
        let (cmd, desc) = result.unwrap();
        assert!(cmd.is_none()); // Suggestions don't produce a command
        assert!(!desc.is_empty());
    }
}

use serde_json::{json, Value};
use machine_empire_core::ecs::World;
use machine_empire_core::state_snapshot;
use machine_empire_core::ai::mcts::MctsPlanner;
use machine_empire_core::protocol::MatchConfig;
use super::types::ResourceDefinition;

/// Get all resource definitions.
pub fn list_resources() -> Vec<ResourceDefinition> {
    vec![
        ResourceDefinition {
            uri: "game://state".into(),
            name: "Game State".into(),
            description: "Full game state including all visible entities, economy, production, and fog.".into(),
            mime_type: "application/json".into(),
        },
        ResourceDefinition {
            uri: "game://state/my_units".into(),
            name: "My Units".into(),
            description: "List of all your combat units with positions, health, and facing.".into(),
            mime_type: "application/json".into(),
        },
        ResourceDefinition {
            uri: "game://state/my_buildings".into(),
            name: "My Buildings".into(),
            description: "List of your buildings (Command Post, Forge) with positions and health.".into(),
            mime_type: "application/json".into(),
        },
        ResourceDefinition {
            uri: "game://state/enemies".into(),
            name: "Visible Enemies".into(),
            description: "List of all visible enemy entities (fog-filtered).".into(),
            mime_type: "application/json".into(),
        },
        ResourceDefinition {
            uri: "game://state/map".into(),
            name: "Map".into(),
            description: "Map dimensions and terrain tile data.".into(),
            mime_type: "application/json".into(),
        },
        ResourceDefinition {
            uri: "game://state/economy".into(),
            name: "Economy".into(),
            description: "Your economy state: energy bank, income, expenses, strain.".into(),
            mime_type: "application/json".into(),
        },
        ResourceDefinition {
            uri: "game://state/fog".into(),
            name: "Fog of War".into(),
            description: "Fog of war grid showing explored/visible/unexplored tiles.".into(),
            mime_type: "application/json".into(),
        },
        ResourceDefinition {
            uri: "game://state/threats".into(),
            name: "Threat Analysis".into(),
            description: "Influence map analysis showing threat levels and safe positions.".into(),
            mime_type: "application/json".into(),
        },
        ResourceDefinition {
            uri: "game://match".into(),
            name: "Match Info".into(),
            description: "Match configuration and status.".into(),
            mime_type: "application/json".into(),
        },
    ]
}

/// Read a resource by URI, returning a JSON value.
/// `world` must be a reference to the current game world.
/// `player_id` is the requesting player (for fog filtering).
pub fn read_resource(
    uri: &str,
    world: &World,
    player_id: u8,
    config: &MatchConfig,
    tick: u32,
) -> Result<Value, String> {
    match uri {
        "game://state" => read_full_state(world, player_id, config, tick),
        "game://state/my_units" => read_my_units(world, player_id),
        "game://state/my_buildings" => read_my_buildings(world, player_id),
        "game://state/enemies" => read_enemies(world, player_id),
        "game://state/map" => read_map(world, config),
        "game://state/economy" => read_economy(world, player_id),
        "game://state/fog" => read_fog(world, player_id),
        "game://state/threats" => read_threats(world, player_id, config),
        "game://match" => read_match_info(config, tick),
        _ => Err(format!("Unknown resource URI: {}", uri)),
    }
}

/// Full game state for the requesting player.
fn read_full_state(world: &World, player_id: u8, config: &MatchConfig, tick: u32) -> Result<Value, String> {
    let entities = state_snapshot::snapshot_entities_for_player(world, player_id);
    let events = state_snapshot::snapshot_events_for_player(world, player_id);
    let fog = state_snapshot::snapshot_fog(world, player_id);
    let economy = state_snapshot::snapshot_economy(world, player_id);
    let production = state_snapshot::snapshot_production(world, player_id);

    Ok(json!({
        "tick": tick,
        "entities": entities.iter().map(|e| json!({
            "entity_id": e.entity_id,
            "x": e.x,
            "y": e.y,
            "sprite_id": e.sprite_id,
            "frame": e.frame,
            "health_pct": e.health_pct,
            "facing": e.facing,
            "owner": e.owner,
            "flags": e.flags,
            "type": sprite_id_name(e.sprite_id)
        })).collect::<Vec<_>>(),
        "economy": {
            "energy_bank": economy.energy_bank,
            "income": economy.income,
            "expense": economy.expense,
            "strain": economy.strain,
            "strain_income_penalty": economy.strain_income_penalty
        },
        "production": production.iter().enumerate().map(|(i, p)| json!({
            "line_index": i,
            "unit_type": p.unit_type,
            "progress": p.progress,
            "total_time": p.total_time,
            "type_name": p.unit_type.map(|t| sprite_id_name(t))
        })).collect::<Vec<_>>(),
        "fog_visible_count": fog.iter().filter(|&&v| v == 2).count(),
        "fog_explored_count": fog.iter().filter(|&&v| v == 1).count(),
        "events_count": events.len(),
        "map_width": config.map_width,
        "map_height": config.map_height
    }))
}

/// List of all your combat units.
fn read_my_units(world: &World, player_id: u8) -> Result<Value, String> {
    let entities = state_snapshot::snapshot_entities_for_player(world, player_id);
    let combat_units: Vec<_> = entities.iter()
        .filter(|e| e.owner == player_id && is_combat_unit(e.sprite_id))
        .map(|e| json!({
            "entity_id": e.entity_id,
            "type": sprite_id_name(e.sprite_id),
            "x": e.x,
            "y": e.y,
            "health_pct": e.health_pct,
            "facing": e.facing
        }))
        .collect();

    Ok(json!({
        "units": combat_units,
        "count": combat_units.len()
    }))
}

/// List of your buildings.
fn read_my_buildings(world: &World, player_id: u8) -> Result<Value, String> {
    let entities = state_snapshot::snapshot_entities_for_player(world, player_id);
    let buildings: Vec<_> = entities.iter()
        .filter(|e| e.owner == player_id && is_building(e.sprite_id))
        .map(|e| json!({
            "entity_id": e.entity_id,
            "type": sprite_id_name(e.sprite_id),
            "x": e.x,
            "y": e.y,
            "health_pct": e.health_pct
        }))
        .collect();

    Ok(json!({
        "buildings": buildings,
        "count": buildings.len()
    }))
}

/// Visible enemy entities (fog-filtered).
fn read_enemies(world: &World, player_id: u8) -> Result<Value, String> {
    let entities = state_snapshot::snapshot_entities_for_player(world, player_id);
    let enemies: Vec<_> = entities.iter()
        .filter(|e| e.owner != player_id)
        .map(|e| json!({
            "entity_id": e.entity_id,
            "type": sprite_id_name(e.sprite_id),
            "owner": e.owner,
            "x": e.x,
            "y": e.y,
            "health_pct": e.health_pct,
            "is_building": is_building(e.sprite_id),
            "is_combat": is_combat_unit(e.sprite_id)
        }))
        .collect();

    Ok(json!({
        "enemies": enemies,
        "count": enemies.len()
    }))
}

/// Map dimensions and terrain data.
fn read_map(world: &World, config: &MatchConfig) -> Result<Value, String> {
    let tiles = state_snapshot::snapshot_map_tiles(world);

    Ok(json!({
        "width": config.map_width,
        "height": config.map_height,
        "total_tiles": tiles.len(),
        "terrain_summary": {
            "open_tiles": tiles.iter().filter(|&&t| (t & 0x0F) == 0).count(),
            "impassable_tiles": tiles.iter().filter(|&&t| (t & 0x0F) != 0).count()
        }
    }))
}

/// Economy state for the requesting player.
fn read_economy(world: &World, player_id: u8) -> Result<Value, String> {
    let economy = state_snapshot::snapshot_economy(world, player_id);

    Ok(json!({
        "energy_bank": economy.energy_bank,
        "income": economy.income,
        "expense": economy.expense,
        "net_rate": economy.income - economy.expense,
        "strain": economy.strain,
        "strain_income_penalty": economy.strain_income_penalty,
        "can_afford_thrall": economy.energy_bank >= 30.0,
        "can_afford_sentinel": economy.energy_bank >= 120.0,
        "can_afford_hover_tank": economy.energy_bank >= 300.0
    }))
}

/// Fog of war summary.
fn read_fog(world: &World, player_id: u8) -> Result<Value, String> {
    let fog = state_snapshot::snapshot_fog(world, player_id);
    let unexplored = fog.iter().filter(|&&v| v == 0).count();
    let explored = fog.iter().filter(|&&v| v == 1).count();
    let visible = fog.iter().filter(|&&v| v == 2).count();

    Ok(json!({
        "total_tiles": fog.len(),
        "unexplored": unexplored,
        "explored": explored,
        "visible": visible,
        "exploration_pct": ((explored + visible) as f64 / fog.len().max(1) as f64 * 100.0).round()
    }))
}

/// Threat analysis from influence maps.
fn read_threats(world: &World, player_id: u8, config: &MatchConfig) -> Result<Value, String> {
    // Extract MCTS state for sector-level analysis
    let state = MctsPlanner::extract_state(
        world,
        2, // Default to 2 players for threat analysis
        config.map_width,
        config.map_height,
    );

    let own_strength = state.total_strength(player_id);
    let mut enemy_strength = 0.0f32;
    for pid in 0..state.player_count {
        if pid == player_id {
            continue;
        }
        enemy_strength += state.total_strength(pid);
    }

    // Find threatening sectors
    let mut threat_sectors = Vec::new();
    for sector in 0..64u8 {
        let mut sector_enemy_strength = 0.0f32;
        for pid in 0..state.player_count as usize {
            if pid == player_id as usize {
                continue;
            }
            sector_enemy_strength += state.sector_units[pid][sector as usize].strength();
        }
        if sector_enemy_strength > 0.0 {
            let (cx, cy) = state.sector_center(sector);
            threat_sectors.push(json!({
                "sector": sector,
                "center_x": cx,
                "center_y": cy,
                "enemy_strength": sector_enemy_strength,
                "own_strength": state.sector_units[player_id as usize][sector as usize].strength()
            }));
        }
    }

    Ok(json!({
        "own_total_strength": own_strength,
        "enemy_total_strength": enemy_strength,
        "strength_ratio": if own_strength + enemy_strength > 0.0 {
            own_strength / (own_strength + enemy_strength)
        } else { 0.5 },
        "threat_sectors": threat_sectors,
        "own_units": state.total_units(player_id),
        "recommendation": if own_strength > enemy_strength * 1.5 {
            "Attack - you have a significant army advantage"
        } else if own_strength < enemy_strength * 0.5 {
            "Defend and produce - enemy army is stronger"
        } else {
            "Build up forces - armies are roughly equal"
        }
    }))
}

/// Match configuration and status.
fn read_match_info(config: &MatchConfig, tick: u32) -> Result<Value, String> {
    Ok(json!({
        "map_width": config.map_width,
        "map_height": config.map_height,
        "player_count": config.player_count,
        "tick_rate_ms": config.tick_rate_ms,
        "current_tick": tick,
        "elapsed_seconds": tick as f64 * config.tick_rate_ms as f64 / 1000.0
    }))
}

/// Convert sprite_id to a human-readable name.
fn sprite_id_name(sprite_id: u16) -> &'static str {
    match sprite_id {
        0 => "Thrall",
        1 => "Sentinel",
        2 => "HoverTank",
        3 => "CommandPost",
        4 => "Forge",
        _ => "Unknown",
    }
}

/// Check if a sprite_id is a combat unit.
fn is_combat_unit(sprite_id: u16) -> bool {
    matches!(sprite_id, 0 | 1 | 2)
}

/// Check if a sprite_id is a building.
fn is_building(sprite_id: u16) -> bool {
    matches!(sprite_id, 3 | 4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use machine_empire_core::game::{Game, GameConfig};

    fn test_game() -> (Game, MatchConfig) {
        let config = GameConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
        };
        let mut game = Game::new(config);
        game.spawn_starting_units(0, 8.0, 8.0);
        game.spawn_starting_units(1, 56.0, 56.0);
        game.tick(50.0);

        let match_config = MatchConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
            tick_rate_ms: 50,
        };
        (game, match_config)
    }

    #[test]
    fn test_resource_list_complete() {
        let resources = list_resources();
        assert_eq!(resources.len(), 9, "Should have 9 resources");

        let uris: Vec<&str> = resources.iter().map(|r| r.uri.as_str()).collect();
        assert!(uris.contains(&"game://state"));
        assert!(uris.contains(&"game://state/my_units"));
        assert!(uris.contains(&"game://state/my_buildings"));
        assert!(uris.contains(&"game://state/enemies"));
        assert!(uris.contains(&"game://state/map"));
        assert!(uris.contains(&"game://state/economy"));
        assert!(uris.contains(&"game://state/fog"));
        assert!(uris.contains(&"game://state/threats"));
        assert!(uris.contains(&"game://match"));
    }

    #[test]
    fn test_state_resource_read() {
        let (game, config) = test_game();
        let result = read_resource("game://state", &game.world, 0, &config, 1);
        assert!(result.is_ok());

        let state = result.unwrap();
        assert_eq!(state["tick"], 1);
        assert!(state["entities"].as_array().unwrap().len() > 0);
        assert!(state["economy"]["energy_bank"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn test_my_units_filtered() {
        let (game, config) = test_game();
        let result = read_resource("game://state/my_units", &game.world, 0, &config, 1);
        assert!(result.is_ok());

        let data = result.unwrap();
        let units = data["units"].as_array().unwrap();
        // Player 0 starts with 3 Thralls (combat units)
        assert_eq!(units.len(), 3, "Player 0 should have 3 combat units, got {}", units.len());
    }

    #[test]
    fn test_my_buildings_filtered() {
        let (game, config) = test_game();
        let result = read_resource("game://state/my_buildings", &game.world, 0, &config, 1);
        assert!(result.is_ok());

        let data = result.unwrap();
        let buildings = data["buildings"].as_array().unwrap();
        // Player 0 has 1 CP + 1 Forge = 2 buildings
        assert_eq!(buildings.len(), 2, "Player 0 should have 2 buildings, got {}", buildings.len());
    }

    #[test]
    fn test_enemies_fog_filtered() {
        let (game, config) = test_game();
        let result = read_resource("game://state/enemies", &game.world, 0, &config, 1);
        assert!(result.is_ok());

        let data = result.unwrap();
        // Player 1's units are far away in fog, should not be visible
        let enemies = data["enemies"].as_array().unwrap();
        // All visible enemies should NOT be player 0
        for enemy in enemies {
            assert_ne!(enemy["owner"].as_u64().unwrap(), 0);
        }
    }

    #[test]
    fn test_economy_resource() {
        let (game, config) = test_game();
        let result = read_resource("game://state/economy", &game.world, 0, &config, 1);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(data["energy_bank"].as_f64().unwrap() > 0.0);
        assert!(data["income"].as_f64().unwrap() > 0.0);
        assert!(data["can_afford_thrall"].as_bool().unwrap());
    }

    #[test]
    fn test_fog_resource() {
        let (game, config) = test_game();
        let result = read_resource("game://state/fog", &game.world, 0, &config, 1);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(data["visible"].as_u64().unwrap() > 0);
        assert!(data["total_tiles"].as_u64().unwrap() == 64 * 64);
    }

    #[test]
    fn test_threats_resource() {
        let (game, config) = test_game();
        let result = read_resource("game://state/threats", &game.world, 0, &config, 1);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(data["own_total_strength"].as_f64().unwrap() > 0.0);
        assert!(data["recommendation"].as_str().is_some());
    }

    #[test]
    fn test_match_info_resource() {
        let (_game, config) = test_game();
        let result = read_match_info(&config, 100);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["map_width"], 64);
        assert_eq!(data["current_tick"], 100);
        assert!(data["elapsed_seconds"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn test_unknown_resource() {
        let (game, config) = test_game();
        let result = read_resource("game://nonexistent", &game.world, 0, &config, 1);
        assert!(result.is_err());
    }
}

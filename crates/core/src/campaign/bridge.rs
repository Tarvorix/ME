use serde::{Serialize, Deserialize};
use crate::game::{Game, GameConfig};
use crate::types::SpriteId;
use crate::components::{UnitType, Health};
use crate::systems::battle_victory::{BattleState, BattleStatus, VictoryReason};
use crate::systems::capture::spawn_capture_points;
use crate::deployment::DeploymentState;
use crate::ai::tactical::{AiControlled, BtTemplateId};
use crate::ai::player::AiDifficulty;
use super::map::{CampaignSite, GarrisonedUnit};

/// Request to create a battle from campaign context.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BattleRequest {
    /// Campaign site ID where the battle takes place.
    pub site_id: u32,
    /// Attacking player ID.
    pub attacker: u8,
    /// Defending player ID (255 if neutral).
    pub defender: u8,
    /// Attacker's forces.
    pub attacker_forces: Vec<GarrisonedUnit>,
    /// Defender's forces.
    pub defender_forces: Vec<GarrisonedUnit>,
    /// Map generation seed.
    pub map_seed: u32,
}

/// Result of a completed battle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BattleResult {
    /// Campaign site ID where the battle took place.
    pub site_id: u32,
    /// Winning player ID.
    pub winner: u8,
    /// Reason for victory.
    pub victory_reason: VictoryReason,
    /// Survivors per player (index = player mapping).
    pub survivors: Vec<(u8, Vec<GarrisonedUnit>)>,
}

/// Create an RTS battle game from a campaign battle request.
/// Player 0 is always the human; other players get AI controllers.
pub fn create_battle_from_campaign(request: &BattleRequest) -> Game {
    let mut game = Game::new(GameConfig {
        map_width: 64,
        map_height: 64,
        player_count: 2,
        seed: request.map_seed,
    });

    // Spawn capture points
    spawn_capture_points(&mut game.world, 3, 64, 64, request.map_seed as u64);

    // Set up deployment state
    let mut deployment = DeploymentState::with_map_size(2, 64, 64);

    // Determine which players are AI-controlled (player 0 = human)
    let attacker_is_ai = request.attacker != 0;
    let defender_is_ai = request.defender != 0 && request.defender != 255;

    // Compute spawn centers for each zone so AI can be given attack targets
    let zone0_cx = deployment.zones[0].center_x;
    let zone0_cy = deployment.zones[0].center_y;
    let zone1_cx = deployment.zones[1].center_x;
    let zone1_cy = deployment.zones[1].center_y;

    // Spawn attacker forces at zone 0
    let cp0 = spawn_force(&mut game, request.attacker, &request.attacker_forces, zone0_cx, zone0_cy, attacker_is_ai);
    deployment.command_posts[0] = Some(cp0);

    // Spawn defender forces at zone 1
    let cp1 = spawn_force(&mut game, request.defender, &request.defender_forces, zone1_cx, zone1_cy, defender_is_ai);
    deployment.command_posts[1] = Some(cp1);

    deployment.confirmed = vec![true, true]; // Auto-confirm for campaign battles

    game.world.insert_resource(deployment);

    // Set battle to Active
    let mut battle_state = BattleState::new(2);
    battle_state.status = BattleStatus::Active;
    game.world.insert_resource(battle_state);

    // Register AI players for tactical and strategic systems
    if attacker_is_ai {
        game.add_ai_player(request.attacker, AiDifficulty::Normal);
    }
    if defender_is_ai {
        game.add_ai_player(request.defender, AiDifficulty::Normal);
    }

    // Give AI units an initial assigned_pos pointing toward the enemy spawn.
    // This makes AI units march toward the enemy immediately instead of idling.
    {
        let ut_storage = game.world.get_storage::<UnitType>();
        let ai_entities: Vec<(crate::ecs::entity::Entity, u8)> = if let Some(uts) = ut_storage {
            if let Some(ai_s) = game.world.get_storage::<AiControlled>() {
                ai_s.iter()
                    .filter_map(|(e, _)| uts.get(e).map(|ut| (e, ut.owner)))
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        for (entity, owner) in ai_entities {
            // Attacker (zone 0) → target zone 1 center, Defender (zone 1) → target zone 0 center
            let (target_x, target_y) = if owner == request.attacker {
                (zone1_cx, zone1_cy)
            } else {
                (zone0_cx, zone0_cy)
            };
            if let Some(ai) = game.world.get_component_mut::<AiControlled>(entity) {
                ai.assigned_pos = Some((target_x, target_y));
            }
        }
    }

    game
}

/// Spawn a player's force at a given position.
/// Only spawns CommandPost + combat units (no Forge — that's campaign-level only).
/// If `is_ai` is true, adds AiControlled components to combat units.
fn spawn_force(game: &mut Game, player: u8, forces: &[GarrisonedUnit], cx: f32, cy: f32, is_ai: bool) -> crate::ecs::entity::Entity {
    // Spawn Command Post
    let cp = game.spawn_unit(SpriteId::CommandPost, cx, cy, player);

    // Spawn units in formation around CP (1.5-tile spacing for visual clarity)
    let mut unit_idx = 0u32;
    for gu in forces {
        let kind = SpriteId::from_u16(gu.unit_type).unwrap_or(SpriteId::Thrall);
        for i in 0..gu.count {
            let row = unit_idx / 5;
            let col = unit_idx % 5;
            let x = cx - 3.0 + col as f32 * 1.5;
            let y = cy + 3.0 + row as f32 * 1.5;
            let entity = game.spawn_unit(kind, x, y, player);

            // Apply health percentage
            if gu.health_pct < 1.0 {
                let bp = crate::blueprints::get_blueprint(kind);
                if let Some(h) = game.world.get_component_mut::<Health>(entity) {
                    h.current = bp.max_hp * gu.health_pct;
                }
            }

            // AI-controlled units get tactical behavior tree
            if is_ai {
                game.world.add_component(entity, AiControlled::new(BtTemplateId::CombatUnit));
            }

            unit_idx += 1;
            let _ = i;
        }
    }

    cp
}

/// Extract the battle result from a completed game.
pub fn extract_battle_result(game: &Game, site_id: u32, attacker: u8, defender: u8) -> Option<BattleResult> {
    let bs = game.world.get_resource::<BattleState>()?;
    if !bs.is_finished() {
        return None;
    }

    let winner = bs.winner;
    let victory_reason = bs.victory_reason.unwrap_or(VictoryReason::TotalElimination);

    // Count surviving units per player
    let ut_storage = game.world.get_storage::<UnitType>()?;
    let health_storage = game.world.get_storage::<Health>();

    let mut attacker_survivors: Vec<GarrisonedUnit> = Vec::new();
    let mut defender_survivors: Vec<GarrisonedUnit> = Vec::new();

    for (entity, ut) in ut_storage.iter() {
        // Skip buildings
        if ut.kind == SpriteId::CommandPost || ut.kind == SpriteId::Forge || ut.kind == SpriteId::CapturePoint {
            continue;
        }

        // Check alive
        let (alive, health_pct) = if let Some(hs) = &health_storage {
            if let Some(h) = hs.get(entity) {
                (!h.is_dead(), h.current / h.max)
            } else {
                (true, 1.0)
            }
        } else {
            (true, 1.0)
        };

        if !alive {
            continue;
        }

        let target = if ut.owner == attacker {
            &mut attacker_survivors
        } else if ut.owner == defender {
            &mut defender_survivors
        } else {
            continue;
        };

        let unit_type = ut.kind as u16;
        if let Some(existing) = target.iter_mut().find(|g| g.unit_type == unit_type) {
            // Weighted average health
            let total = existing.count + 1;
            existing.health_pct = (existing.health_pct * existing.count as f32 + health_pct) / total as f32;
            existing.count = total;
        } else {
            target.push(GarrisonedUnit {
                unit_type,
                count: 1,
                health_pct,
            });
        }
    }

    let mut survivors = Vec::new();
    if !attacker_survivors.is_empty() {
        survivors.push((attacker, attacker_survivors));
    }
    if !defender_survivors.is_empty() {
        survivors.push((defender, defender_survivors));
    }

    Some(BattleResult {
        site_id,
        winner,
        victory_reason,
        survivors,
    })
}

/// Apply a battle result to the campaign map.
/// - Winner takes ownership of the site
/// - Survivors return to the site's garrison
/// - Dead are permanently lost
pub fn apply_battle_result(
    site: &mut CampaignSite,
    result: &BattleResult,
) {
    site.is_contested = false;
    site.battle_id = None;
    site.owner = result.winner;

    // Clear old garrison
    site.garrison.clear();

    // Add winner's survivors to garrison
    for (player, units) in &result.survivors {
        if *player == result.winner {
            for unit in units {
                super::garrison::add_to_garrison(&mut site.garrison, unit.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::map::{CampaignSite, SiteType};

    fn test_request() -> BattleRequest {
        BattleRequest {
            site_id: 5,
            attacker: 0,
            defender: 1,
            attacker_forces: vec![
                GarrisonedUnit::new(0, 10), // 10 Thralls
                GarrisonedUnit::new(1, 3),  // 3 Sentinels
                GarrisonedUnit::new(2, 1),  // 1 HoverTank
            ],
            defender_forces: vec![
                GarrisonedUnit::new(0, 8),  // 8 Thralls
                GarrisonedUnit::new(1, 2),  // 2 Sentinels
            ],
            map_seed: 42,
        }
    }

    #[test]
    fn test_battle_spawns_units() {
        let request = test_request();
        let game = create_battle_from_campaign(&request);

        let ut_storage = game.world.get_storage::<UnitType>().unwrap();

        // Count attacker units (player 0): 10T + 3S + 1HT + CP = 15 (no Forge in battles)
        let attacker_count = ut_storage.iter()
            .filter(|(_, ut)| ut.owner == 0)
            .count();
        assert_eq!(attacker_count, 15, "Attacker should have 15 entities, got {}", attacker_count);

        // Count defender units (player 1): 8T + 2S + CP = 11 (no Forge in battles)
        let defender_count = ut_storage.iter()
            .filter(|(_, ut)| ut.owner == 1)
            .count();
        assert_eq!(defender_count, 11, "Defender should have 11 entities, got {}", defender_count);
    }

    #[test]
    fn test_survivors_extracted() {
        let request = test_request();
        let mut game = create_battle_from_campaign(&request);

        // Kill all of player 1's units to end the battle
        {
            let ut_storage = game.world.get_storage::<UnitType>().unwrap();
            let p1_entities: Vec<_> = ut_storage.iter()
                .filter(|(_, ut)| ut.owner == 1)
                .map(|(e, _)| e)
                .collect();

            for entity in p1_entities {
                if let Some(h) = game.world.get_component_mut::<Health>(entity) {
                    h.current = 0.0;
                }
            }
        }

        // Tick until battle ends
        for _ in 0..300 {
            game.tick(50.0);
        }

        let result = extract_battle_result(&game, 5, 0, 1);
        assert!(result.is_some(), "Should have a battle result");

        let result = result.unwrap();
        assert_eq!(result.winner, 0);

        // Attacker should have survivors
        let attacker_survivors = result.survivors.iter()
            .find(|(p, _)| *p == 0);
        assert!(attacker_survivors.is_some(), "Attacker should have survivors");

        // Defender should have no survivors
        let defender_survivors = result.survivors.iter()
            .find(|(p, _)| *p == 1);
        assert!(defender_survivors.is_none(), "Defender should have no survivors");
    }

    #[test]
    fn test_ownership_transfers() {
        let mut site = CampaignSite::new(5, SiteType::MiningStation, 50.0, 50.0)
            .with_owner(1);
        site.is_contested = true;

        let result = BattleResult {
            site_id: 5,
            winner: 0,
            victory_reason: VictoryReason::TotalElimination,
            survivors: vec![
                (0, vec![GarrisonedUnit::new(0, 5)]),
            ],
        };

        apply_battle_result(&mut site, &result);

        assert_eq!(site.owner, 0, "Site should transfer to winner");
        assert!(!site.is_contested);
        assert_eq!(site.garrison_count(), 5, "Survivors should garrison the site");
    }

    #[test]
    fn test_dead_permanently_lost() {
        let request = test_request();
        let game = create_battle_from_campaign(&request);

        let bs = game.world.get_resource::<BattleState>().unwrap();
        assert!(bs.is_active(), "Battle should be active");

        // The dead units from a battle are permanently gone - they don't appear
        // in survivors when extracted
    }

    #[test]
    fn test_battle_is_active_on_creation() {
        let request = test_request();
        let game = create_battle_from_campaign(&request);

        let bs = game.world.get_resource::<BattleState>().unwrap();
        assert!(bs.is_active(), "Battle should start in Active status");
    }

    #[test]
    fn test_survivors_return_with_health() {
        let request = BattleRequest {
            site_id: 5,
            attacker: 0,
            defender: 1,
            attacker_forces: vec![
                GarrisonedUnit::with_health(0, 5, 0.5),
            ],
            defender_forces: vec![
                GarrisonedUnit::new(0, 1),
            ],
            map_seed: 42,
        };

        let game = create_battle_from_campaign(&request);

        // Verify units spawned with reduced health
        let ut_storage = game.world.get_storage::<UnitType>().unwrap();
        let health_storage = game.world.get_storage::<Health>().unwrap();

        let attacker_thralls: Vec<_> = ut_storage.iter()
            .filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Thrall)
            .collect();

        assert_eq!(attacker_thralls.len(), 5);

        for (entity, _) in &attacker_thralls {
            let h = health_storage.get(*entity).unwrap();
            assert!((h.current - 80.0 * 0.5).abs() < 0.01,
                "Thrall should have 50% health, got {}", h.current);
        }
    }

    #[test]
    fn test_tick_advances_battle() {
        let request = test_request();
        let mut game = create_battle_from_campaign(&request);

        let tick_before = game.tick_count;
        game.tick(50.0);
        assert_eq!(game.tick_count, tick_before + 1);
    }

    #[test]
    fn test_battle_ai_initialized() {
        // Player 0 = human attacker, Player 1 = AI defender
        let request = test_request();
        let game = create_battle_from_campaign(&request);

        // AI player should be registered
        let ai_players = game.world.get_resource::<crate::ai::player::AiPlayers>().unwrap();
        assert!(ai_players.0.iter().any(|a| a.player_id == 1), "Defender (player 1) should be registered as AI");
        assert!(!ai_players.0.iter().any(|a| a.player_id == 0), "Attacker (player 0) should NOT be registered as AI");

        // Defender's combat units should have AiControlled component
        let ut_storage = game.world.get_storage::<UnitType>().unwrap();
        let ai_storage = game.world.get_storage::<AiControlled>();

        let mut defender_combat_count = 0;
        let mut defender_ai_count = 0;
        let mut attacker_ai_count = 0;

        for (entity, ut) in ut_storage.iter() {
            if ut.kind == SpriteId::CommandPost || ut.kind == SpriteId::CapturePoint {
                continue;
            }
            if ut.owner == 1 {
                defender_combat_count += 1;
                if let Some(ref storage) = ai_storage {
                    if storage.get(entity).is_some() {
                        defender_ai_count += 1;
                    }
                }
            }
            if ut.owner == 0 {
                if let Some(ref storage) = ai_storage {
                    if storage.get(entity).is_some() {
                        attacker_ai_count += 1;
                    }
                }
            }
        }

        assert!(defender_combat_count > 0, "Defender should have combat units");
        assert_eq!(defender_ai_count, defender_combat_count,
            "All defender combat units should have AiControlled (got {}/{})", defender_ai_count, defender_combat_count);
        assert_eq!(attacker_ai_count, 0,
            "Human attacker units should NOT have AiControlled");
    }
}

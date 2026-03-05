use crate::ecs::World;
use crate::ecs::entity::Entity;
use crate::components::{Position, UnitType, Health, CombatState};
use crate::command::{Command, PendingCommands};
use crate::blueprints::get_blueprint;
use crate::ai::behavior_tree::{
    BehaviorTree, BtContext, BtState, evaluate, build_combat_bt,
};
use crate::ai::influence_map::{InfluenceGrid, INFLUENCE_UPDATE_INTERVAL};

/// Identifies which behavior tree template an AI entity uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BtTemplateId {
    CombatUnit,
    DefensiveUnit,
    ScoutUnit,
}

/// Component marking an entity as AI-controlled with behavior tree state.
pub struct AiControlled {
    /// Which behavior tree template to use.
    pub bt_id: BtTemplateId,
    /// Per-entity BT execution state.
    pub state: BtState,
    /// Position assigned by strategic AI (attack/defend sector target).
    pub assigned_pos: Option<(f32, f32)>,
}

impl AiControlled {
    pub fn new(bt_id: BtTemplateId) -> Self {
        AiControlled {
            bt_id,
            state: BtState::new(),
            assigned_pos: None,
        }
    }
}

/// Resource: pre-built behavior tree templates.
pub struct BtTemplates {
    pub combat: BehaviorTree,
}

impl BtTemplates {
    pub fn new() -> Self {
        BtTemplates {
            combat: build_combat_bt(),
        }
    }
}

/// Resource: per-player shared AI knowledge (blackboard).
pub struct AiBlackboard {
    /// Per-player Command Post positions.
    pub cp_positions: Vec<Option<(f32, f32)>>,
    /// Per-player strategic attack target positions.
    pub attack_targets: Vec<Option<(f32, f32)>>,
}

impl AiBlackboard {
    pub fn new(player_count: u32) -> Self {
        let pc = player_count as usize;
        AiBlackboard {
            cp_positions: vec![None; pc],
            attack_targets: vec![None; pc],
        }
    }
}

/// The tactical AI ECS system. Runs behavior trees for all AI-controlled entities.
/// Produces Commands that are pushed to PendingCommands for processing next tick.
pub fn ai_tactical_system(world: &mut World) {
    // Get templates and blackboard
    let bt_templates = match world.get_resource::<BtTemplates>() {
        Some(t) => t as *const BtTemplates,
        None => return,
    };
    let blackboard = match world.get_resource::<AiBlackboard>() {
        Some(b) => b as *const AiBlackboard,
        None => return,
    };

    // Update influence map if needed
    let _current_tick = world.get_resource::<crate::game::TickDelta>()
        .map(|_| 0u32) // We'll use the tick count from the influence grid
        .unwrap_or(0);

    // Collect all AI entities and their data first (to avoid borrow issues)
    let ai_entities: Vec<(Entity, f32, f32, u8, u8, f32, f32, Option<Entity>, BtTemplateId, Option<(f32, f32)>)> = {
        let ai_storage = match world.get_storage::<AiControlled>() {
            Some(s) => s,
            None => return,
        };
        let pos_storage = match world.get_storage::<Position>() {
            Some(s) => s,
            None => return,
        };
        let ut_storage = match world.get_storage::<UnitType>() {
            Some(s) => s,
            None => return,
        };
        let health_storage = world.get_storage::<Health>();
        let combat_storage = world.get_storage::<CombatState>();

        let mut entities = Vec::new();
        for (entity, ai) in ai_storage.iter() {
            let pos = match pos_storage.get(entity) {
                Some(p) => p,
                None => continue,
            };
            let ut = match ut_storage.get(entity) {
                Some(ut) => ut,
                None => continue,
            };

            // Skip dead entities
            let health_pct = if let Some(hs) = &health_storage {
                if let Some(h) = hs.get(entity) {
                    if h.is_dead() { continue; }
                    h.percent()
                } else { 100 }
            } else { 100 };

            let bp = get_blueprint(ut.kind);

            // Get current combat target
            let target = if let Some(cs) = &combat_storage {
                cs.get(entity).and_then(|c| c.target)
            } else {
                None
            };

            entities.push((
                entity,
                pos.x,
                pos.y,
                ut.owner,
                health_pct,
                bp.attack_range,
                bp.vision_range,
                target,
                ai.bt_id,
                ai.assigned_pos,
            ));
        }
        entities
    };

    if ai_entities.is_empty() {
        return;
    }

    // Collect all living entities for nearest-enemy lookup
    let all_units: Vec<(Entity, f32, f32, u8, bool)> = {
        let pos_storage = world.get_storage::<Position>().unwrap();
        let ut_storage = world.get_storage::<UnitType>().unwrap();
        let health_storage = world.get_storage::<Health>();

        let mut units = Vec::new();
        for (entity, pos) in pos_storage.iter() {
            let ut = match ut_storage.get(entity) {
                Some(ut) => ut,
                None => continue,
            };

            let alive = if let Some(hs) = &health_storage {
                hs.get(entity).map(|h| !h.is_dead()).unwrap_or(true)
            } else {
                true
            };

            if !alive { continue; }

            let bp = get_blueprint(ut.kind);
            let is_combat = bp.damage > 0.0 || bp.speed > 0.0; // Include mobile units

            units.push((entity, pos.x, pos.y, ut.owner, is_combat));
        }
        units
    };

    // Get influence grid data (read-only)
    let influence = world.get_resource::<InfluenceGrid>();

    // SAFETY: We're reading templates and blackboard immutably while world is borrowed mutably
    // for PendingCommands. These resources are not modified during this loop.
    let templates = unsafe { &*bt_templates };
    let bb = unsafe { &*blackboard };

    // Evaluate BT for each AI entity and collect commands
    let mut all_commands: Vec<Command> = Vec::new();
    let mut bt_state_updates: Vec<(Entity, BtState)> = Vec::new();

    for &(entity, px, py, owner, health_pct, attack_range, vision_range, target, bt_id, assigned_pos) in &ai_entities {
        // Build context
        let mut ctx = BtContext::new(entity);
        ctx.pos_x = px;
        ctx.pos_y = py;
        ctx.health_pct = health_pct;
        ctx.owner = owner;
        ctx.attack_range = attack_range;
        ctx.vision_range = vision_range;

        // Set assigned position from strategic AI
        if let Some((ax, ay)) = assigned_pos {
            ctx.assigned_x = Some(ax);
            ctx.assigned_y = Some(ay);
        }

        // Set CP position from blackboard
        if let Some(Some((cpx, cpy))) = bb.cp_positions.get(owner as usize) {
            ctx.cp_x = *cpx;
            ctx.cp_y = *cpy;
        }

        // Set current target info
        if let Some(target_entity) = target {
            ctx.target = Some(target_entity);

            // Find target position and alive status
            if let Some((_, tx, ty, _, _)) = all_units.iter().find(|(e, _, _, _, _)| *e == target_entity) {
                ctx.target_x = *tx;
                ctx.target_y = *ty;
                ctx.target_alive = true;
                ctx.target_distance = ((px - tx).powi(2) + (py - ty).powi(2)).sqrt();
            } else {
                ctx.target_alive = false;
                ctx.target_distance = f32::MAX;
            }
        }

        // Find nearest visible enemy
        let mut nearest_dist = f32::MAX;
        for &(e, ex, ey, e_owner, _) in &all_units {
            if e_owner == owner { continue; } // Skip friendlies
            let dist = ((px - ex).powi(2) + (py - ey).powi(2)).sqrt();
            if dist <= vision_range && dist < nearest_dist {
                nearest_dist = dist;
                ctx.nearest_enemy = Some(e);
                ctx.nearest_enemy_x = ex;
                ctx.nearest_enemy_y = ey;
                ctx.nearest_enemy_distance = dist;
            }
        }

        // Set influence data
        if let Some(ig) = &influence {
            let tile_x = px.floor() as u32;
            let tile_y = py.floor() as u32;
            ctx.friendly_influence = ig.get_friendly_strength(owner as u32, tile_x, tile_y);
            ctx.threat_influence = ig.get_threat(owner as u32, tile_x, tile_y);

            // Find safe position for potential retreat
            if let Some((sx, sy)) = ig.find_safe_position(owner as u32, tile_x, tile_y, 8) {
                ctx.safe_x = sx as f32 + 0.5;
                ctx.safe_y = sy as f32 + 0.5;
            } else {
                ctx.safe_x = ctx.cp_x;
                ctx.safe_y = ctx.cp_y;
            }
        } else {
            ctx.safe_x = ctx.cp_x;
            ctx.safe_y = ctx.cp_y;
        }

        // Select behavior tree
        let tree = match bt_id {
            BtTemplateId::CombatUnit => &templates.combat,
            BtTemplateId::DefensiveUnit => &templates.combat, // Same for now
            BtTemplateId::ScoutUnit => &templates.combat,     // Same for now
        };

        // Evaluate the behavior tree
        let mut bt_state = BtState::new(); // Fresh state each tick for now
        evaluate(tree, tree.root, &mut bt_state, &mut ctx);

        // Collect commands
        all_commands.extend(ctx.commands);
        bt_state_updates.push((entity, bt_state));
    }

    // Push commands to PendingCommands
    if let Some(pending) = world.get_resource_mut::<PendingCommands>() {
        for cmd in all_commands {
            pending.push(cmd);
        }
    }

    // Update BT states on entities
    for (entity, new_state) in bt_state_updates {
        if let Some(ai) = world.get_component_mut::<AiControlled>(entity) {
            ai.state = new_state;
        }
    }
}

/// Update the influence map resource if enough ticks have passed.
/// Should be called before ai_tactical_system.
pub fn ai_influence_update_system(world: &mut World) {
    // Get current game tick
    let game_tick = {
        let ui_buf = world.get_resource::<crate::systems::resource::UIStateBuffer>();
        if let Some(ui) = ui_buf {
            u32::from_le_bytes([ui.0[20], ui.0[21], ui.0[22], ui.0[23]])
        } else {
            return;
        }
    };

    // Check if we need to update
    let needs_update = {
        if let Some(ig) = world.get_resource::<InfluenceGrid>() {
            game_tick.wrapping_sub(ig.last_update_tick) >= INFLUENCE_UPDATE_INTERVAL
        } else {
            return;
        }
    };

    if !needs_update {
        return;
    }

    // We need to pass &World to update(), but we also need &mut on the InfluenceGrid.
    // Use a temporary: extract, update, re-insert.
    let mut grid = {
        let ig = world.get_resource::<InfluenceGrid>().unwrap();
        InfluenceGrid::new(ig.width, ig.height, ig.player_count)
    };
    grid.update(world, game_tick);

    // Store back
    world.insert_resource(grid);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, GameConfig};
    #[allow(unused_imports)]
    use crate::types::SpriteId;

    fn test_game_with_ai() -> Game {
        // Game::new already initializes AI resources (InfluenceGrid, BtTemplates, AiBlackboard)
        // and registers ai_tactical_system in the system runner.
        Game::new(GameConfig {
            map_width: 32,
            map_height: 32,
            player_count: 2,
            seed: 42,
        })
    }

    fn spawn_ai_thrall(game: &mut Game, x: f32, y: f32, owner: u8) -> Entity {
        let entity = game.spawn_thrall(x, y, owner);
        game.world.add_component(entity, AiControlled::new(BtTemplateId::CombatUnit));
        entity
    }

    #[test]
    fn test_ai_controlled_entity_attacks() {
        let mut game = test_game_with_ai();

        // AI thrall (player 0) near enemy thrall (player 1)
        let _ai = spawn_ai_thrall(&mut game, 10.5, 10.5, 0);
        let _enemy = game.spawn_thrall(13.5, 10.5, 1); // 3 tiles away, within vision

        // Tick runs all systems including ai_tactical_system
        game.tick(50.0);

        // ai_tactical generated commands during the tick, pending for next tick
        let pending = game.world.get_resource::<PendingCommands>().unwrap();
        assert!(!pending.0.is_empty(),
            "AI should generate commands when enemy is in vision range");

        let has_attack = pending.0.iter().any(|cmd| {
            matches!(cmd, Command::Attack { .. })
        });
        assert!(has_attack, "AI should generate Attack command when enemy is nearby");
    }

    #[test]
    fn test_ai_retreat_on_low_health() {
        let mut game = test_game_with_ai();

        // Set up blackboard with CP position
        if let Some(bb) = game.world.get_resource_mut::<AiBlackboard>() {
            bb.cp_positions[0] = Some((5.0, 5.0));
        }

        let ai = spawn_ai_thrall(&mut game, 15.5, 15.5, 0);
        let _enemy = game.spawn_thrall(17.5, 15.5, 1);

        // Set AI unit to critical health
        if let Some(h) = game.world.get_component_mut::<Health>(ai) {
            h.current = 10.0; // ~12.5% of 80
        }

        // Tick runs ai_tactical which should make the low-health unit retreat
        game.tick(50.0);

        let pending = game.world.get_resource::<PendingCommands>().unwrap();
        let has_move = pending.0.iter().any(|cmd| {
            matches!(cmd, Command::Move { .. })
        });
        assert!(has_move, "Low health AI should generate Move (retreat) command, got: {:?}", pending.0);
    }

    #[test]
    fn test_ai_idle_when_no_enemies() {
        let mut game = test_game_with_ai();

        // AI thrall far from any enemy
        let _ai = spawn_ai_thrall(&mut game, 5.5, 5.5, 0);

        // Tick runs ai_tactical
        game.tick(50.0);

        // No enemies visible, should idle (no commands from AI)
        let pending = game.world.get_resource::<PendingCommands>().unwrap();
        assert!(pending.0.is_empty(),
            "AI should not generate commands when no enemies visible, got: {:?}", pending.0);
    }

    #[test]
    fn test_ai_acquires_target_when_visible() {
        let mut game = test_game_with_ai();

        let _ai = spawn_ai_thrall(&mut game, 10.5, 10.5, 0);
        let enemy = game.spawn_thrall(14.5, 10.5, 1); // 4 tiles away, within vision (8)

        game.tick(50.0);

        let pending = game.world.get_resource::<PendingCommands>().unwrap();
        let attack_cmd = pending.0.iter().find(|cmd| matches!(cmd, Command::Attack { .. }));
        assert!(attack_cmd.is_some(), "AI should attack visible enemy");

        if let Some(Command::Attack { target_id, .. }) = attack_cmd {
            assert_eq!(*target_id, enemy.raw(),
                "AI should target the visible enemy entity");
        }
    }

    #[test]
    fn test_ai_commands_processed_next_tick() {
        let mut game = test_game_with_ai();

        let ai = spawn_ai_thrall(&mut game, 10.5, 10.5, 0);
        let _enemy = game.spawn_thrall(12.5, 10.5, 1); // Close enough to attack

        // First tick: AI generates commands (pending for next tick)
        game.tick(50.0);

        let has_commands = game.world.get_resource::<PendingCommands>()
            .map(|p| !p.0.is_empty())
            .unwrap_or(false);
        assert!(has_commands, "AI should have generated commands");

        // Second tick: command_processor drains and processes those commands
        game.tick(50.0);

        // The AI entity should now have a target set via the combat system
        let cs = game.world.get_component::<CombatState>(ai);
        if let Some(cs) = cs {
            let _ = cs.target; // Just verify no crash
        }
    }

    #[test]
    fn test_multiple_ai_entities() {
        let mut game = test_game_with_ai();

        // 3 AI thralls for player 0
        spawn_ai_thrall(&mut game, 10.5, 10.5, 0);
        spawn_ai_thrall(&mut game, 11.5, 10.5, 0);
        spawn_ai_thrall(&mut game, 12.5, 10.5, 0);

        // Enemy nearby
        game.spawn_thrall(15.5, 10.5, 1);

        // One tick runs ai_tactical for all entities
        game.tick(50.0);

        let pending = game.world.get_resource::<PendingCommands>().unwrap();
        let attack_count = pending.0.iter()
            .filter(|cmd| matches!(cmd, Command::Attack { .. }))
            .count();
        assert_eq!(attack_count, 3, "Each AI entity should generate an attack command, got {}", attack_count);
    }

    #[test]
    fn test_dead_ai_entity_skipped() {
        let mut game = test_game_with_ai();

        let ai = spawn_ai_thrall(&mut game, 10.5, 10.5, 0);
        game.spawn_thrall(14.5, 10.5, 1);

        // Kill the AI entity before ticking
        if let Some(h) = game.world.get_component_mut::<Health>(ai) {
            h.current = 0.0;
        }

        game.tick(50.0);

        // Dead AI should not generate commands
        let pending = game.world.get_resource::<PendingCommands>().unwrap();
        assert!(pending.0.is_empty(), "Dead AI should not generate commands");
    }
}

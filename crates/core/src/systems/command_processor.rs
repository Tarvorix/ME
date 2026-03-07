use crate::ecs::World;
use crate::ecs::entity::Entity;
use crate::command::{Command, PendingCommands};
use crate::components::{Position, PathState, CombatState, AttackMoveTarget};
use crate::map::BattleMap;
use crate::pathfinding::find_path;
use crate::targeting::{is_entity_alive, is_hostile_attack_target};
use crate::types::SpriteId;
use crate::systems::production::Productions;

pub fn command_processor_system(world: &mut World) {
    // Drain pending commands
    let commands = if let Some(pending) = world.get_resource_mut::<PendingCommands>() {
        pending.drain()
    } else {
        return;
    };

    for cmd in commands {
        match cmd {
            Command::Move { unit_ids, target_x, target_y } => {
                process_move(world, &unit_ids, target_x, target_y);
            }
            Command::Stop { unit_ids } => {
                process_stop(world, &unit_ids);
            }
            Command::Attack { unit_ids, target_id } => {
                process_attack(world, &unit_ids, target_id);
            }
            Command::AttackMove { unit_ids, target_x, target_y } => {
                process_attack_move(world, &unit_ids, target_x, target_y);
            }
            Command::Build { .. } => {
                // Building placement — implemented in future chunk
            }
            Command::Produce { player, unit_type } => {
                process_produce(world, player, unit_type);
            }
            Command::CancelProduction { player, line_index } => {
                process_cancel_production(world, player, line_index);
            }
            Command::SetRally { player, x, y } => {
                process_set_rally(world, player, x, y);
            }
            Command::Deploy { player, cp_x, cp_y } => {
                crate::deployment::process_deploy(world, player, cp_x, cp_y);
            }
            Command::ConfirmDeployment { player } => {
                crate::deployment::process_confirm_deployment(world, player);
            }
            Command::UpgradeNode { player, upgrade } => {
                process_upgrade_node(world, player, upgrade);
            }
            Command::CampaignResearch { .. } |
            Command::CampaignDispatch { .. } |
            Command::CampaignWithdraw { .. } |
            Command::RequestReinforcement { .. } => {
                // Campaign commands and reinforcements are handled at the CampaignGame level, not the RTS level
            }
        }
    }
}

fn process_move(world: &mut World, unit_ids: &[u32], target_x: f32, target_y: f32) {
    let goal_tx = target_x.floor().max(0.0) as u32;
    let goal_ty = target_y.floor().max(0.0) as u32;

    // Read map dimensions for clamping
    let (map_w, map_h) = {
        let map = world.get_resource::<BattleMap>().unwrap();
        (map.width, map.height)
    };

    let goal_tx = goal_tx.min(map_w - 1);
    let goal_ty = goal_ty.min(map_h - 1);

    for &raw_id in unit_ids {
        let entity = Entity::from_raw(raw_id);
        if !is_entity_alive(world, entity) {
            continue;
        }

        // Get current position
        let (start_tx, start_ty) = if let Some(pos) = world.get_component::<Position>(entity) {
            (pos.x.floor().max(0.0) as u32, pos.y.floor().max(0.0) as u32)
        } else {
            continue;
        };

        let start_tx = start_tx.min(map_w - 1);
        let start_ty = start_ty.min(map_h - 1);

        // Look up unit kind for terrain-aware pathfinding
        let unit_kind = world.get_component::<crate::components::UnitType>(entity)
            .map(|ut| ut.kind);

        // Run A* pathfinding
        let path = {
            let map = world.get_resource::<BattleMap>().unwrap();
            find_path(map, (start_tx, start_ty), (goal_tx, goal_ty), unit_kind)
        };

        // Set path on the entity
        if let Some(path) = path {
            if let Some(path_state) = world.get_component_mut::<PathState>(entity) {
                path_state.path = path;
                path_state.current_index = 1; // skip start tile (we're already there)
            }
        }

        if let Some(cs) = world.get_component_mut::<CombatState>(entity) {
            cs.target = None;
            cs.in_range = false;
        }

        world.remove_component::<AttackMoveTarget>(entity);
    }
}

fn process_stop(world: &mut World, unit_ids: &[u32]) {
    for &raw_id in unit_ids {
        let entity = Entity::from_raw(raw_id);
        if !is_entity_alive(world, entity) {
            continue;
        }
        if let Some(path_state) = world.get_component_mut::<PathState>(entity) {
            path_state.clear();
        }
        // Also clear attack target
        if let Some(cs) = world.get_component_mut::<CombatState>(entity) {
            cs.target = None;
            cs.in_range = false;
        }
        // Remove attack-move target
        world.remove_component::<AttackMoveTarget>(entity);
    }
}

fn process_attack(world: &mut World, unit_ids: &[u32], target_id: u32) {
    let target = Entity::from_raw(target_id);

    for &raw_id in unit_ids {
        let entity = Entity::from_raw(raw_id);
        if !is_entity_alive(world, entity) {
            continue;
        }

        let attacker_owner = match world.get_component::<crate::components::UnitType>(entity) {
            Some(ut) => ut.owner,
            None => continue,
        };
        if !is_hostile_attack_target(world, attacker_owner, target) {
            continue;
        }

        // Set combat target
        if let Some(cs) = world.get_component_mut::<CombatState>(entity) {
            cs.target = Some(target);
            cs.in_range = false;
        }

        // Clear any existing path — combat system will handle chase pathfinding
        if let Some(ps) = world.get_component_mut::<PathState>(entity) {
            ps.clear();
        }

        // Remove attack-move target (explicit attack overrides)
        world.remove_component::<AttackMoveTarget>(entity);
    }
}

fn process_attack_move(world: &mut World, unit_ids: &[u32], target_x: f32, target_y: f32) {
    // Pathfind to destination first
    process_move(world, unit_ids, target_x, target_y);

    // Add AttackMoveTarget component so combat system knows to engage enemies en route
    for &raw_id in unit_ids {
        let entity = Entity::from_raw(raw_id);
        if !is_entity_alive(world, entity) {
            continue;
        }
        world.add_component(entity, AttackMoveTarget { x: target_x, y: target_y });
    }
}

fn process_produce(world: &mut World, player: u8, unit_type: u16) {
    let sprite_id = SpriteId::from_u16(unit_type);
    if sprite_id.is_none() {
        return;
    }
    let sprite_id = sprite_id.unwrap();

    if let Some(prods) = world.get_resource_mut::<Productions>() {
        let idx = player as usize;
        if idx < prods.0.len() {
            prods.0[idx].queue_unit(sprite_id);
        }
    }
}

fn process_cancel_production(world: &mut World, player: u8, line_index: u8) {
    if let Some(prods) = world.get_resource_mut::<Productions>() {
        let idx = player as usize;
        if idx < prods.0.len() {
            prods.0[idx].cancel_line(line_index);
        }
    }
}

fn process_upgrade_node(world: &mut World, player: u8, upgrade: u8) {
    if let Some(prods) = world.get_resource_mut::<Productions>() {
        let idx = player as usize;
        if idx < prods.0.len() {
            match upgrade {
                // Infantry2: add second infantry line (400 energy, already deducted at campaign level)
                0 => {
                    if prods.0[idx].infantry_lines.len() < 2 {
                        prods.0[idx].infantry_lines.push(None);
                    }
                }
                // Armor2: add second armor line (600 energy)
                1 => {
                    if prods.0[idx].armor_lines.len() < 2 {
                        prods.0[idx].armor_lines.push(None);
                    }
                }
                // Infantry3: add third infantry line (800 energy)
                2 => {
                    if prods.0[idx].infantry_lines.len() < 3 {
                        prods.0[idx].infantry_lines.push(None);
                    }
                }
                // Armor3: add third armor line (1200 energy)
                3 => {
                    if prods.0[idx].armor_lines.len() < 3 {
                        prods.0[idx].armor_lines.push(None);
                    }
                }
                _ => {}
            }
        }
    }
}

fn process_set_rally(world: &mut World, player: u8, x: f32, y: f32) {
    if let Some(prods) = world.get_resource_mut::<Productions>() {
        let idx = player as usize;
        if idx < prods.0.len() {
            prods.0[idx].rally_x = x;
            prods.0[idx].rally_y = y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::game::{Game, GameConfig};
    use crate::types::SpriteId;

    fn test_game() -> Game {
        Game::new(GameConfig {
            map_width: 32,
            map_height: 32,
            player_count: 2,
            seed: 42,
        })
    }

    #[test]
    fn move_orders_clear_attack_move_stance() {
        let mut game = test_game();
        let unit = game.spawn_thrall(5.5, 5.5, 0);

        game.push_command(Command::AttackMove {
            unit_ids: vec![unit.raw()],
            target_x: 10.5,
            target_y: 5.5,
        });
        game.tick(50.0);
        assert!(game.world.has_component::<AttackMoveTarget>(unit));

        game.push_command(Command::Move {
            unit_ids: vec![unit.raw()],
            target_x: 12.5,
            target_y: 5.5,
        });
        game.tick(50.0);

        assert!(
            !game.world.has_component::<AttackMoveTarget>(unit),
            "Move orders should clear the defend/attack-move stance",
        );
    }

    #[test]
    fn explicit_attack_rejects_neutral_objectives() {
        let mut game = test_game();
        let attacker = game.spawn_thrall(5.5, 5.5, 0);
        let capture_point = game.spawn_unit(SpriteId::CapturePoint, 8.5, 5.5, 255);

        game.push_command(Command::Attack {
            unit_ids: vec![attacker.raw()],
            target_id: capture_point.raw(),
        });
        game.tick(50.0);

        let combat_state = game.world.get_component::<CombatState>(attacker).unwrap();
        assert_eq!(combat_state.target, None);
    }
}

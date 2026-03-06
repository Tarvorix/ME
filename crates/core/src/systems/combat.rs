use crate::ecs::World;
use crate::ecs::entity::Entity;
use crate::components::{Position, CombatState, RenderState, Health, UnitType, PathState, VisionRange, AttackMoveTarget};
use crate::blueprints::get_blueprint;
use crate::game::{TickDelta, write_event};
use crate::map::{BattleMap, damage_reduction, TerrainType, HAZARD_DPS};
use crate::pathfinding::find_path;
use crate::types::{AnimState, Direction, EventType, SpriteId};

pub fn combat_system(world: &mut World) {
    let delta_secs = if let Some(td) = world.get_resource::<TickDelta>() {
        td.0
    } else {
        return;
    };

    // Collect entities that have CombatState
    let combat_entities: Vec<(Entity, Option<Entity>, f32)> = {
        let cs_storage = world.get_storage::<CombatState>();
        if cs_storage.is_none() {
            return;
        }
        cs_storage.unwrap().iter()
            .map(|(entity, cs)| (entity, cs.target, cs.attack_cooldown))
            .collect()
    };

    for (entity, target_opt, _cooldown) in combat_entities {
        if !world.is_alive(entity) {
            continue;
        }

        // Decrement attack cooldown
        if let Some(cs) = world.get_component_mut::<CombatState>(entity) {
            cs.attack_cooldown = (cs.attack_cooldown - delta_secs).max(0.0);
        }

        // If no explicit target, check for attack-move auto-targeting
        let effective_target = if target_opt.is_some() {
            target_opt
        } else {
            // Only auto-target if this entity is on attack-move
            if world.has_component::<AttackMoveTarget>(entity) {
                find_nearest_enemy(world, entity)
            } else {
                None
            }
        };

        let target = match effective_target {
            Some(t) if world.is_alive(t) => t,
            Some(_) => {
                // Target is dead — clear it
                if let Some(cs) = world.get_component_mut::<CombatState>(entity) {
                    cs.target = None;
                    cs.in_range = false;
                }
                // If attack-moving, resume path (path is still set)
                if let Some(rs) = world.get_component_mut::<RenderState>(entity) {
                    if rs.anim_state == AnimState::Attack {
                        rs.anim_state = AnimState::Idle;
                        rs.anim_timer = 0.0;
                        rs.frame = 0;
                    }
                }
                continue;
            }
            None => continue,
        };

        // If we auto-targeted, set it on the CombatState
        if target_opt.is_none() {
            if let Some(cs) = world.get_component_mut::<CombatState>(entity) {
                cs.target = Some(target);
            }
        }

        // Get attacker info
        let (attacker_x, attacker_y) = match world.get_component::<Position>(entity) {
            Some(pos) => (pos.x, pos.y),
            None => continue,
        };

        let attacker_sprite_id = match world.get_component::<UnitType>(entity) {
            Some(ut) => ut.kind,
            None => continue,
        };

        let bp = get_blueprint(attacker_sprite_id);
        if bp.damage <= 0.0 {
            continue; // Buildings with no damage
        }

        // Get target position
        let (target_x, target_y) = match world.get_component::<Position>(target) {
            Some(pos) => (pos.x, pos.y),
            None => continue,
        };

        // Distance check
        let dx = target_x - attacker_x;
        let dy = target_y - attacker_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= bp.attack_range {
            // In range
            if let Some(cs) = world.get_component_mut::<CombatState>(entity) {
                cs.in_range = true;
            }

            // Stop moving while attacking (unless attack-moving — resume after target dies)
            let is_attack_moving = world.has_component::<AttackMoveTarget>(entity);
            if !is_attack_moving {
                if let Some(ps) = world.get_component_mut::<PathState>(entity) {
                    ps.clear();
                }
            }

            // Face the target
            let face_dir = Direction::from_delta(dx, dy);
            if let Some(rs) = world.get_component_mut::<RenderState>(entity) {
                rs.facing = face_dir as u8;
            }

            // Check cooldown
            let current_cooldown = if let Some(cs) = world.get_component::<CombatState>(entity) {
                cs.attack_cooldown
            } else {
                1.0
            };

            if current_cooldown <= 0.0 {
                // Fire! Deal damage to target, applying terrain damage reduction
                let terrain_dr = {
                    let target_tx = target_x.floor().max(0.0) as u32;
                    let target_ty = target_y.floor().max(0.0) as u32;
                    if let Some(map) = world.get_resource::<BattleMap>() {
                        let tx = target_tx.min(map.width.saturating_sub(1));
                        let ty = target_ty.min(map.height.saturating_sub(1));
                        let terrain = map.get(tx, ty).terrain_type();
                        damage_reduction(terrain)
                    } else {
                        0.0
                    }
                };
                let effective_damage = bp.damage * (1.0 - terrain_dr);

                let target_dead = if let Some(health) = world.get_component_mut::<Health>(target) {
                    health.current -= effective_damage;
                    health.current <= 0.0
                } else {
                    false
                };

                // Reset cooldown
                if let Some(cs) = world.get_component_mut::<CombatState>(entity) {
                    cs.attack_cooldown = bp.attack_cooldown;
                }

                // Set attack animation
                if let Some(rs) = world.get_component_mut::<RenderState>(entity) {
                    if rs.anim_state != AnimState::Attack {
                        rs.anim_state = AnimState::Attack;
                        rs.anim_timer = 0.0;
                        rs.frame = 0;
                    }
                }

                // Write Shot event
                let mut payload = [0u8; 16];
                payload[0..4].copy_from_slice(&target.raw().to_le_bytes());
                payload[4..8].copy_from_slice(&effective_damage.to_le_bytes());
                payload[8..10].copy_from_slice(&(attacker_sprite_id as u16).to_le_bytes());
                write_event(world, EventType::Shot, entity.raw(), attacker_x, attacker_y, &payload);

                // If target died, clear our target
                if target_dead {
                    if let Some(cs) = world.get_component_mut::<CombatState>(entity) {
                        cs.target = None;
                        cs.in_range = false;
                    }
                }
            }
        } else {
            // Not in range — chase the target
            if let Some(cs) = world.get_component_mut::<CombatState>(entity) {
                cs.in_range = false;
            }

            chase_target(world, entity, attacker_x, attacker_y, target_x, target_y);
        }
    }

    // Apply Hazard tile damage to non-hover ground units
    apply_hazard_damage(world, delta_secs);

    // Revert attack animation to idle/move for entities that finished their attack anim
    revert_attack_anim(world);
}

/// Chase a target by pathfinding to their current position.
fn chase_target(
    world: &mut World,
    entity: Entity,
    _from_x: f32,
    _from_y: f32,
    target_x: f32,
    target_y: f32,
) {
    let (map_w, map_h) = {
        let map = world.get_resource::<BattleMap>().unwrap();
        (map.width, map.height)
    };

    let start = if let Some(pos) = world.get_component::<Position>(entity) {
        (pos.x.floor().max(0.0).min(map_w as f32 - 1.0) as u32,
         pos.y.floor().max(0.0).min(map_h as f32 - 1.0) as u32)
    } else {
        return;
    };

    let goal = (
        target_x.floor().max(0.0).min(map_w as f32 - 1.0) as u32,
        target_y.floor().max(0.0).min(map_h as f32 - 1.0) as u32,
    );

    // Only re-pathfind if we don't already have a path or our current path is heading elsewhere
    let needs_repath = if let Some(ps) = world.get_component::<PathState>(entity) {
        if !ps.has_path() {
            true
        } else {
            // Check if path endpoint is near the target
            let end = ps.path[ps.path.len() - 1];
            let end_dx = end.0 as f32 - goal.0 as f32;
            let end_dy = end.1 as f32 - goal.1 as f32;
            (end_dx * end_dx + end_dy * end_dy) > 4.0 // Re-path if >2 tiles off
        }
    } else {
        return; // No PathState = can't move (building)
    };

    if needs_repath {
        // Look up unit kind for terrain-aware pathfinding
        let unit_kind = world.get_component::<UnitType>(entity)
            .map(|ut| ut.kind);

        let path = {
            let map = world.get_resource::<BattleMap>().unwrap();
            find_path(map, start, goal, unit_kind)
        };

        if let Some(path) = path {
            if let Some(ps) = world.get_component_mut::<PathState>(entity) {
                ps.path = path;
                ps.current_index = 1;
            }
        }
    }
}

/// Find the nearest enemy entity within vision range.
fn find_nearest_enemy(world: &World, entity: Entity) -> Option<Entity> {
    let (my_x, my_y) = {
        let pos = world.get_component::<Position>(entity)?;
        (pos.x, pos.y)
    };

    let my_owner = {
        let ut = world.get_component::<UnitType>(entity)?;
        ut.owner
    };

    let vision = {
        let vr = world.get_component::<VisionRange>(entity)?;
        vr.0
    };

    let vision_sq = vision * vision;

    let pos_storage = world.get_storage::<Position>()?;
    let ut_storage = world.get_storage::<UnitType>()?;
    let health_storage = world.get_storage::<Health>()?;

    let mut closest: Option<(Entity, f32)> = None;

    for (other, pos) in pos_storage.iter() {
        if other == entity {
            continue;
        }

        // Must be an enemy
        let other_owner = match ut_storage.get(other) {
            Some(ut) => ut.owner,
            None => continue,
        };
        if other_owner == my_owner {
            continue;
        }

        // Must be alive
        if let Some(h) = health_storage.get(other) {
            if h.is_dead() {
                continue;
            }
        }

        let dx = pos.x - my_x;
        let dy = pos.y - my_y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq <= vision_sq {
            if closest.is_none() || dist_sq < closest.unwrap().1 {
                closest = Some((other, dist_sq));
            }
        }
    }

    closest.map(|(e, _)| e)
}

/// Apply Hazard terrain damage to non-hover ground units each tick.
fn apply_hazard_damage(world: &mut World, delta_secs: f32) {
    // Collect entities standing on Hazard tiles (non-hover units only)
    let hazard_victims: Vec<Entity> = {
        let pos_storage = match world.get_storage::<Position>() {
            Some(s) => s,
            None => return,
        };
        let ut_storage = match world.get_storage::<UnitType>() {
            Some(s) => s,
            None => return,
        };
        let map = match world.get_resource::<BattleMap>() {
            Some(m) => m,
            None => return,
        };

        pos_storage.iter()
            .filter_map(|(entity, pos)| {
                let ut = ut_storage.get(entity)?;
                // HoverTank ignores hazard
                if ut.kind == SpriteId::HoverTank {
                    return None;
                }
                // Buildings don't take hazard damage
                if ut.kind == SpriteId::CommandPost || ut.kind == SpriteId::Node {
                    return None;
                }
                let tx = pos.x.floor().max(0.0) as u32;
                let ty = pos.y.floor().max(0.0) as u32;
                if tx < map.width && ty < map.height {
                    let terrain = map.get(tx, ty).terrain_type();
                    if terrain == TerrainType::Hazard {
                        return Some(entity);
                    }
                }
                None
            })
            .collect()
    };

    let damage = HAZARD_DPS * delta_secs;
    for entity in hazard_victims {
        if let Some(health) = world.get_component_mut::<Health>(entity) {
            health.current -= damage;
        }
    }
}

/// After attack animation completes (non-looping), revert to Idle if not still attacking.
fn revert_attack_anim(world: &mut World) {
    let entities_to_revert: Vec<Entity> = {
        let rs_storage = world.get_storage::<RenderState>();
        if rs_storage.is_none() {
            return;
        }
        let rs_s = rs_storage.unwrap();
        let cs_storage = world.get_storage::<CombatState>();

        rs_s.iter()
            .filter(|(entity, rs)| {
                // In Attack anim state, animation has completed (non-looping)
                if rs.anim_state != AnimState::Attack {
                    return false;
                }

                // Check if entity has no target (attack done)
                if let Some(cs_s) = &cs_storage {
                    if let Some(cs) = cs_s.get(*entity) {
                        return cs.target.is_none();
                    }
                }
                true
            })
            .map(|(entity, _)| entity)
            .collect()
    };

    for entity in entities_to_revert {
        // Check if unit is moving (read before mutable borrow)
        let is_moving = world.get_component::<PathState>(entity)
            .map(|ps| ps.has_path())
            .unwrap_or(false);

        if let Some(rs) = world.get_component_mut::<RenderState>(entity) {
            if is_moving {
                rs.anim_state = AnimState::Move;
            } else {
                rs.anim_state = AnimState::Idle;
            }
            rs.anim_timer = 0.0;
            rs.frame = 0;
        }
    }
}

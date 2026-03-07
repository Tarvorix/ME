use crate::ecs::World;
use crate::components::{Position, PreviousPosition, PathState, RenderState, UnitType};
use crate::map::{BattleMap, movement_cost};
use crate::targeting::is_entity_alive;
use crate::types::{AnimState, Direction, SpriteId};
use crate::game::TickDelta;

pub fn movement_system(world: &mut World) {
    let delta_secs = if let Some(td) = world.get_resource::<TickDelta>() {
        td.0
    } else {
        return;
    };

    // We need to iterate over entities that have Position, PreviousPosition, PathState, and RenderState.
    // Since our ECS doesn't support multi-component queries directly, we iterate PathState
    // and manually look up the other components.

    // Collect entities with paths to process
    let entities_with_paths: Vec<_> = if let Some(storage) = world.get_storage::<PathState>() {
        storage.iter()
            .filter(|(entity, ps)| ps.has_path() && is_entity_alive(world, *entity))
            .map(|(entity, _)| entity)
            .collect()
    } else {
        return;
    };

    for entity in entities_with_paths {
        // Save previous position
        if let (Some(pos), Some(prev)) = (
            world.get_component::<Position>(entity),
            world.get_component::<PreviousPosition>(entity),
        ) {
            let px = pos.x;
            let py = pos.y;
            // Need mutable access, re-borrow
            let _ = (pos, prev);
            if let Some(prev) = world.get_component_mut::<PreviousPosition>(entity) {
                prev.x = px;
                prev.y = py;
            }
        }

        // Get path data
        let (target_x, target_y, speed, current_index, path_len, waypoint_tx, waypoint_ty) = {
            if let Some(ps) = world.get_component::<PathState>(entity) {
                if !ps.has_path() {
                    continue;
                }
                let (tx, ty) = ps.path[ps.current_index];
                (tx as f32 + 0.5, ty as f32 + 0.5, ps.speed, ps.current_index, ps.path.len(), tx, ty)
            } else {
                continue;
            }
        };

        // Get current position
        let (cur_x, cur_y) = if let Some(pos) = world.get_component::<Position>(entity) {
            (pos.x, pos.y)
        } else {
            continue;
        };

        // Apply terrain speed modifier: speed * (1.0 / movement_cost)
        let terrain_speed_factor = {
            let unit_kind = world.get_component::<UnitType>(entity)
                .map(|ut| ut.kind)
                .unwrap_or(SpriteId::Thrall);
            if let Some(map) = world.get_resource::<BattleMap>() {
                let tx = waypoint_tx.min(map.width.saturating_sub(1));
                let ty = waypoint_ty.min(map.height.saturating_sub(1));
                let terrain = map.get(tx, ty).terrain_type();
                let cost = movement_cost(terrain, unit_kind);
                if cost.is_infinite() { 0.0 } else { 1.0 / cost }
            } else {
                1.0
            }
        };
        let effective_speed = speed * terrain_speed_factor;

        // Compute movement
        let dx = target_x - cur_x;
        let dy = target_y - cur_y;
        let dist = (dx * dx + dy * dy).sqrt();
        let move_dist = effective_speed * delta_secs;

        if dist <= move_dist {
            // Arrived at waypoint
            if let Some(pos) = world.get_component_mut::<Position>(entity) {
                pos.x = target_x;
                pos.y = target_y;
            }

            let next_index = current_index + 1;
            if next_index >= path_len {
                // Path complete
                if let Some(ps) = world.get_component_mut::<PathState>(entity) {
                    ps.clear();
                }
                if let Some(rs) = world.get_component_mut::<RenderState>(entity) {
                    rs.anim_state = AnimState::Idle;
                    rs.anim_timer = 0.0;
                    rs.frame = 0;
                }
            } else {
                // Advance to next waypoint
                if let Some(ps) = world.get_component_mut::<PathState>(entity) {
                    ps.current_index = next_index;
                }
            }
        } else {
            // Move toward waypoint
            let nx = dx / dist;
            let ny = dy / dist;

            if let Some(pos) = world.get_component_mut::<Position>(entity) {
                pos.x += nx * move_dist;
                pos.y += ny * move_dist;
            }
        }

        // Update facing and anim state
        if dx.abs() > 0.001 || dy.abs() > 0.001 {
            let dir = Direction::from_delta(dx, dy);
            if let Some(rs) = world.get_component_mut::<RenderState>(entity) {
                rs.facing = dir as u8;
                if rs.anim_state != AnimState::Move {
                    rs.anim_state = AnimState::Move;
                    rs.anim_timer = 0.0;
                    rs.frame = 0;
                }
            }
        }
    }

    // Set idle state for entities without paths
    let idle_entities: Vec<_> = if let Some(storage) = world.get_storage::<PathState>() {
        storage.iter()
            .filter(|(entity, ps)| !ps.has_path() && is_entity_alive(world, *entity))
            .map(|(entity, _)| entity)
            .collect()
    } else {
        return;
    };

    for entity in idle_entities {
        if let Some(rs) = world.get_component_mut::<RenderState>(entity) {
            if rs.anim_state == AnimState::Move {
                rs.anim_state = AnimState::Idle;
                rs.anim_timer = 0.0;
                rs.frame = 0;
            }
        }
    }
}

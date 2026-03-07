use crate::ecs::World;
use crate::ecs::entity::Entity;
use crate::components::{Health, RenderState, UnitType, Position, DeathTimer, PathState, CombatState, AttackMoveTarget};
use crate::game::{TickDelta, write_event};
use crate::types::{AnimState, EventType, SpriteId, get_frame_count};

pub fn death_cleanup_system(world: &mut World) {
    let delta_secs = if let Some(td) = world.get_resource::<TickDelta>() {
        td.0
    } else {
        return;
    };

    // Phase 1: Find entities that just died (health <= 0, not yet in Death anim)
    // and transition them to Death animation state + add DeathTimer.
    let newly_dead: Vec<(Entity, SpriteId, f32, f32)> = {
        let health_storage = world.get_storage::<Health>();
        let rs_storage = world.get_storage::<RenderState>();
        let ut_storage = world.get_storage::<UnitType>();
        let pos_storage = world.get_storage::<Position>();

        if health_storage.is_none() || rs_storage.is_none() {
            return;
        }

        let hs = health_storage.unwrap();
        let rss = rs_storage.unwrap();
        let uts = ut_storage.unwrap();
        let poss = pos_storage.unwrap();

        hs.iter()
            .filter(|(_, h)| h.is_dead())
            .filter_map(|(entity, _)| {
                let rs = rss.get(entity)?;
                // Skip if already in Death state
                if rs.anim_state == AnimState::Death {
                    return None;
                }
                let ut = uts.get(entity)?;
                let pos = poss.get(entity)?;
                Some((entity, ut.kind, pos.x, pos.y))
            })
            .collect()
    };

    for (entity, sprite_id, x, y) in &newly_dead {
        // Freeze the corpse immediately so later systems cannot keep advancing stale orders.
        if let Some(path_state) = world.get_component_mut::<PathState>(*entity) {
            path_state.clear();
        }
        if let Some(combat_state) = world.get_component_mut::<CombatState>(*entity) {
            combat_state.target = None;
            combat_state.in_range = false;
            combat_state.attack_cooldown = 0.0;
        }
        world.remove_component::<AttackMoveTarget>(*entity);

        // Set death animation
        if let Some(rs) = world.get_component_mut::<RenderState>(*entity) {
            rs.anim_state = AnimState::Death;
            rs.anim_timer = 0.0;
            rs.frame = 0;
        }

        // Calculate death animation duration
        let frame_count = get_frame_count(*sprite_id, AnimState::Death);
        let frame_duration = AnimState::Death.frame_duration();
        let death_duration = frame_count as f32 * frame_duration;

        // Add DeathTimer component
        world.add_component(*entity, DeathTimer(death_duration));

        // Write Death event
        let mut payload = [0u8; 16];
        payload[0..2].copy_from_slice(&(*sprite_id as u16).to_le_bytes());
        let owner = world.get_component::<UnitType>(*entity)
            .map(|ut| ut.owner)
            .unwrap_or(0);
        payload[2] = owner;
        write_event(world, EventType::Death, entity.raw(), *x, *y, &payload);
    }

    // Phase 2: Tick DeathTimers and despawn entities whose timer has expired.
    let expired: Vec<Entity> = {
        let dt_storage = world.get_storage::<DeathTimer>();
        if dt_storage.is_none() {
            return;
        }

        dt_storage.unwrap().iter()
            .filter(|(_, dt)| dt.0 <= 0.0)
            .map(|(entity, _)| entity)
            .collect()
    };

    // Decrement timers for entities still alive
    let active_timers: Vec<Entity> = {
        let dt_storage = world.get_storage::<DeathTimer>();
        if dt_storage.is_none() {
            return;
        }
        dt_storage.unwrap().iter()
            .filter(|(_, dt)| dt.0 > 0.0)
            .map(|(entity, _)| entity)
            .collect()
    };

    for entity in active_timers {
        if let Some(dt) = world.get_component_mut::<DeathTimer>(entity) {
            dt.0 -= delta_secs;
        }
    }

    // Despawn expired entities
    for entity in expired {
        world.despawn(entity);
    }
}

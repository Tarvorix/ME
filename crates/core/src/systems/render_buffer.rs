use crate::ecs::World;
use crate::components::{Position, RenderState, UnitType, Health};
use crate::game::{RenderBuffer, RenderCount, RENDER_ENTRY_SIZE};
use crate::systems::fog::{FogGrid, FOG_VISIBLE};

/// Resource: the local player index for fog-of-war filtering.
/// Enemy units on tiles not visible to this player are hidden from the render buffer.
pub struct LocalPlayer(pub u32);

pub fn render_buffer_system(world: &mut World) {
    // Get local player for fog filtering (default to player 0)
    let local_player = world.get_resource::<LocalPlayer>()
        .map(|lp| lp.0)
        .unwrap_or(0);

    // Collect all entities with Position + RenderState + UnitType
    let entries: Vec<_> = {
        let pos_storage = world.get_storage::<Position>();
        let rs_storage = world.get_storage::<RenderState>();
        let ut_storage = world.get_storage::<UnitType>();
        let health_storage = world.get_storage::<Health>();
        let fog = world.get_resource::<FogGrid>();

        if pos_storage.is_none() || rs_storage.is_none() || ut_storage.is_none() {
            if let Some(count) = world.get_resource_mut::<RenderCount>() {
                count.0 = 0;
            }
            return;
        }

        let pos_s = pos_storage.unwrap();
        let rs_s = rs_storage.unwrap();
        let ut_s = ut_storage.unwrap();

        pos_s.iter()
            .filter_map(|(entity, pos)| {
                let rs = rs_s.get(entity)?;
                let ut = ut_s.get(entity)?;

                // Fog-of-war filtering: enemy units are only rendered if on a Visible tile
                if ut.owner as u32 != local_player {
                    if let Some(fg) = &fog {
                        let tile_x = pos.x.floor() as u32;
                        let tile_y = pos.y.floor() as u32;
                        if fg.get(local_player, tile_x, tile_y) != FOG_VISIBLE {
                            return None; // Enemy not visible in fog
                        }
                    }
                }

                // Derive health_pct from Health component if present, fallback to RenderState
                let health_pct = health_storage.as_ref()
                    .and_then(|hs| hs.get(entity))
                    .map(|h| h.percent())
                    .unwrap_or(rs.health_pct);
                Some((entity, pos.x, pos.y, rs.sprite_id, rs.frame, health_pct,
                       rs.facing, ut.owner, rs.flags, rs.scale))
            })
            .collect()
    };

    let buffer = if let Some(rb) = world.get_resource_mut::<RenderBuffer>() {
        &mut rb.0
    } else {
        return;
    };

    let mut count: u32 = 0;

    for (entity, x, y, sprite_id, frame, health_pct, facing, owner, flags, scale) in &entries {
        let offset = count as usize * RENDER_ENTRY_SIZE;
        if offset + RENDER_ENTRY_SIZE > buffer.len() {
            break; // buffer full
        }

        // entity_id: u32 (bytes 0-3)
        buffer[offset..offset + 4].copy_from_slice(&entity.raw().to_le_bytes());
        // x: f32 (bytes 4-7)
        buffer[offset + 4..offset + 8].copy_from_slice(&x.to_le_bytes());
        // y: f32 (bytes 8-11)
        buffer[offset + 8..offset + 12].copy_from_slice(&y.to_le_bytes());
        // sprite_id: u16 (bytes 12-13)
        buffer[offset + 12..offset + 14].copy_from_slice(&sprite_id.to_le_bytes());
        // frame: u16 (bytes 14-15)
        buffer[offset + 14..offset + 16].copy_from_slice(&frame.to_le_bytes());
        // health_pct: u8 (byte 16)
        buffer[offset + 16] = *health_pct;
        // facing: u8 (byte 17)
        buffer[offset + 17] = *facing;
        // owner: u8 (byte 18)
        buffer[offset + 18] = *owner;
        // flags: u8 (byte 19)
        buffer[offset + 19] = *flags;
        // scale: f32 (bytes 20-23)
        buffer[offset + 20..offset + 24].copy_from_slice(&scale.to_le_bytes());
        // z_order: f32 (bytes 24-27) - isometric depth = x + y
        let z_order = x + y;
        buffer[offset + 24..offset + 28].copy_from_slice(&z_order.to_le_bytes());
        // reserved: u32 (bytes 28-31)
        buffer[offset + 28..offset + 32].copy_from_slice(&0u32.to_le_bytes());

        count += 1;
    }

    if let Some(rc) = world.get_resource_mut::<RenderCount>() {
        rc.0 = count;
    }
}

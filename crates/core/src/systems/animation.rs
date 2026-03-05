use crate::ecs::World;
use crate::components::RenderState;
use crate::types::{SpriteId, get_frame_count};
use crate::game::TickDelta;

pub fn animation_system(world: &mut World) {
    let delta_secs = if let Some(td) = world.get_resource::<TickDelta>() {
        td.0
    } else {
        return;
    };

    let entities: Vec<_> = if let Some(storage) = world.get_storage::<RenderState>() {
        storage.iter().map(|(e, _)| e).collect()
    } else {
        return;
    };

    for entity in entities {
        if let Some(rs) = world.get_component_mut::<RenderState>(entity) {
            let frame_duration = rs.anim_state.frame_duration();
            rs.anim_timer += delta_secs;

            if rs.anim_timer >= frame_duration {
                rs.anim_timer -= frame_duration;

                let sprite_id = match rs.sprite_id {
                    0 => SpriteId::Thrall,
                    1 => SpriteId::Sentinel,
                    2 => SpriteId::HoverTank,
                    3 => SpriteId::CommandPost,
                    4 => SpriteId::Forge,
                    _ => SpriteId::Thrall,
                };

                let max_frames = get_frame_count(sprite_id, rs.anim_state);

                if rs.anim_state.loops() {
                    rs.frame = (rs.frame + 1) % max_frames;
                } else {
                    if rs.frame < max_frames - 1 {
                        rs.frame += 1;
                    }
                    // Non-looping: stay on last frame
                }
            }

            // Pack anim_state into flags
            rs.pack_flags();
        }
    }
}

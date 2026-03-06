use crate::ecs::World;
use crate::ecs::entity::Entity;
use crate::components::{Position, UnitType, Health, CapturePointState};
use crate::game::{TickDelta, write_event};
use crate::types::{EventType, SpriteId};

/// Capture point decay rate when no units are nearby (progress per second).
const CAPTURE_DECAY_RATE: f32 = 2.0;

/// Capture system: handles proximity-based capture point mechanics.
///
/// Rules:
/// - Units within capture_radius contribute to capture.
/// - Single player with units nearby → progress increases (speed scales with sqrt(count)).
/// - Multiple players with units nearby → contested (progress paused).
/// - No units nearby → progress decays toward 0.
/// - At progress 100.0, the point flips to the capturing player's ownership.
/// - If a different player starts capturing an owned point, progress counts
///   down from 100 (uncapture) then up (recapture).
pub fn capture_system(world: &mut World) {
    let delta_secs = if let Some(td) = world.get_resource::<TickDelta>() {
        td.0
    } else {
        return;
    };

    // Collect capture point entities and their state
    let capture_points: Vec<(Entity, f32, f32, f32, f32, u8, f32, u8)> = {
        let cp_storage = match world.get_storage::<CapturePointState>() {
            Some(s) => s,
            None => return,
        };
        let pos_storage = match world.get_storage::<Position>() {
            Some(s) => s,
            None => return,
        };

        cp_storage.iter().filter_map(|(entity, cp)| {
            let pos = pos_storage.get(entity)?;
            Some((entity, pos.x, pos.y, cp.capture_radius, cp.capture_speed, cp.owner, cp.progress, cp.capturing_player))
        }).collect()
    };

    // Collect all alive combat units (non-buildings, alive)
    let units: Vec<(Entity, f32, f32, u8)> = {
        let pos_storage = match world.get_storage::<Position>() {
            Some(s) => s,
            None => return,
        };
        let ut_storage = match world.get_storage::<UnitType>() {
            Some(s) => s,
            None => return,
        };
        let health_storage = world.get_storage::<Health>();

        pos_storage.iter().filter_map(|(entity, pos)| {
            let ut = ut_storage.get(entity)?;
            // Skip buildings and capture points
            if ut.kind == SpriteId::CommandPost || ut.kind == SpriteId::Node || ut.kind == SpriteId::CapturePoint {
                return None;
            }
            // Check alive
            if let Some(hs) = &health_storage {
                if let Some(h) = hs.get(entity) {
                    if h.is_dead() {
                        return None;
                    }
                }
            }
            Some((entity, pos.x, pos.y, ut.owner))
        }).collect()
    };

    // Process each capture point
    for (cp_entity, cp_x, cp_y, radius, speed, current_owner, current_progress, current_capturing) in &capture_points {
        let radius_sq = radius * radius;

        // Count units per player within capture radius
        let mut player_counts: [u32; 8] = [0; 8]; // max 8 players
        for &(_unit_entity, ux, uy, owner) in &units {
            let dx = ux - cp_x;
            let dy = uy - cp_y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= radius_sq && (owner as usize) < 8 {
                player_counts[owner as usize] += 1;
            }
        }

        // Determine which players have presence
        let players_present: Vec<(u8, u32)> = player_counts.iter().enumerate()
            .filter(|(_, &count)| count > 0)
            .map(|(pid, &count)| (pid as u8, count))
            .collect();

        let contested = players_present.len() > 1;
        let mut new_progress = *current_progress;
        let mut new_capturing = *current_capturing;
        let mut new_owner = *current_owner;
        let mut emit_complete = false;

        if contested {
            // Contested: progress paused, no change
            // Update contested flag only
        } else if players_present.len() == 1 {
            let (capturing_player, unit_count) = players_present[0];
            // Capture speed scales with sqrt of unit count
            let effective_speed = speed * (unit_count as f32).sqrt();

            if *current_owner == 255 {
                // Neutral point — capture directly
                new_capturing = capturing_player;
                new_progress += effective_speed * delta_secs;
                if new_progress >= 100.0 {
                    new_progress = 100.0;
                    new_owner = capturing_player;
                    emit_complete = true;
                }
            } else if *current_owner == capturing_player {
                // Already owned by this player — reinforce (progress stays at 100)
                new_capturing = capturing_player;
                new_progress = 100.0;
            } else {
                // Enemy trying to capture — must first uncapture (reduce progress)
                if *current_capturing == capturing_player && *current_progress < 100.0 {
                    // Already in recapture phase — increase progress toward 100
                    new_progress += effective_speed * delta_secs;
                    if new_progress >= 100.0 {
                        new_progress = 100.0;
                        new_owner = capturing_player;
                        emit_complete = true;
                    }
                } else {
                    // First reduce current owner's control
                    new_progress -= effective_speed * delta_secs;
                    if new_progress <= 0.0 {
                        // Point is neutralized, start capturing for new player
                        new_progress = 0.0;
                        new_owner = 255; // neutral
                        new_capturing = capturing_player;
                    }
                }
            }
        } else {
            // No units nearby — decay progress toward 0
            if new_progress > 0.0 && *current_owner == 255 {
                new_progress = (new_progress - CAPTURE_DECAY_RATE * delta_secs).max(0.0);
                if new_progress <= 0.0 {
                    new_capturing = 255;
                }
            }
            // Owned points don't decay — they keep their 100% progress
        }

        // Apply changes
        if let Some(cp) = world.get_component_mut::<CapturePointState>(*cp_entity) {
            let progress_changed = (cp.progress - new_progress).abs() > 0.01;
            cp.progress = new_progress;
            cp.capturing_player = new_capturing;
            cp.contested = contested;
            cp.owner = new_owner;

            // Emit CaptureProgress event when progress changes significantly
            if progress_changed && !emit_complete {
                let mut payload = [0u8; 16];
                payload[0] = cp.point_index;
                payload[1] = new_capturing;
                payload[2..6].copy_from_slice(&new_progress.to_le_bytes());
                payload[6] = new_owner;
                payload[7] = if contested { 1 } else { 0 };
                write_event(world, EventType::CaptureProgress, cp_entity.raw(), *cp_x, *cp_y, &payload);
            }
        }

        // Emit CaptureComplete event
        if emit_complete {
            let mut payload = [0u8; 16];
            if let Some(cp) = world.get_component::<CapturePointState>(*cp_entity) {
                payload[0] = cp.point_index;
            }
            payload[1] = new_owner;
            write_event(world, EventType::CaptureComplete, cp_entity.raw(), *cp_x, *cp_y, &payload);
        }
    }
}

/// Spawn capture points on the battle map using deterministic placement.
/// Places an odd number of points (3 or 5) spread across the map,
/// avoiding spawn corners.
pub fn spawn_capture_points(world: &mut World, count: u8, map_width: u32, map_height: u32, seed: u64) {
    use crate::blueprints::get_blueprint;
    use crate::components::{PreviousPosition, RenderState, VisionRange, Deployed};

    let count = if count % 2 == 0 { count + 1 } else { count }; // ensure odd
    let count = count.max(3).min(7); // 3-7 capture points

    // Deterministic placement using simple seeded positions
    // Center point always exists, others are distributed symmetrically
    let cx = map_width as f32 / 2.0;
    let cy = map_height as f32 / 2.0;

    let mut positions: Vec<(f32, f32)> = Vec::new();

    // Always place center point
    positions.push((cx, cy));

    if count >= 3 {
        // Two points at 1/4 and 3/4 along diagonal
        let offset_x = map_width as f32 * 0.25;
        let offset_y = map_height as f32 * 0.25;
        // Use seed to add slight variation
        let hash = (seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) as f32;
        let jitter_x = (hash % 3.0) - 1.0;
        let jitter_y = ((hash / 3.0) % 3.0) - 1.0;
        positions.push((cx - offset_x + jitter_x, cy - offset_y + jitter_y));
        positions.push((cx + offset_x - jitter_x, cy + offset_y - jitter_y));
    }

    if count >= 5 {
        // Two more points on the other diagonal
        let offset_x = map_width as f32 * 0.25;
        let offset_y = map_height as f32 * 0.25;
        let hash2 = (seed.wrapping_mul(1103515245).wrapping_add(12345)) as f32;
        let jitter_x = (hash2 % 3.0) - 1.0;
        let jitter_y = ((hash2 / 3.0) % 3.0) - 1.0;
        positions.push((cx + offset_x + jitter_x, cy - offset_y + jitter_y));
        positions.push((cx - offset_x - jitter_x, cy + offset_y - jitter_y));
    }

    if count >= 7 {
        // Two more at top/bottom center
        let offset_y = map_height as f32 * 0.35;
        positions.push((cx, cy - offset_y));
        positions.push((cx, cy + offset_y));
    }

    // Clamp positions to map bounds with margin
    let margin = 3.0;
    for pos in positions.iter_mut() {
        pos.0 = pos.0.clamp(margin, map_width as f32 - margin);
        pos.1 = pos.1.clamp(margin, map_height as f32 - margin);
    }

    // Spawn capture point entities
    let bp = get_blueprint(SpriteId::CapturePoint);
    for (i, &(x, y)) in positions.iter().enumerate().take(count as usize) {
        let entity = world.spawn();
        world.add_component(entity, Position { x: x + 0.5, y: y + 0.5 });
        world.add_component(entity, PreviousPosition { x: x + 0.5, y: y + 0.5 });
        world.add_component(entity, UnitType { kind: SpriteId::CapturePoint, owner: 255 });
        world.add_component(entity, Health::new(bp.max_hp));
        world.add_component(entity, VisionRange(bp.vision_range));
        world.add_component(entity, Deployed(true));
        world.add_component(entity, RenderState::new(SpriteId::CapturePoint, bp.scale));
        world.add_component(entity, CapturePointState::new(i as u8));
    }
}

/// Resource: per-player capture point count for win condition checking.
pub struct CapturePointCounts {
    /// Total number of capture points on the map.
    pub total: u8,
    /// Number of capture points owned per player (indexed by player_id).
    pub per_player: [u8; 8],
}

impl CapturePointCounts {
    pub fn new() -> Self {
        CapturePointCounts {
            total: 0,
            per_player: [0; 8],
        }
    }

    /// Update counts from the world's capture point components.
    pub fn update(&mut self, world: &World) {
        self.total = 0;
        self.per_player = [0; 8];

        let cp_storage = match world.get_storage::<CapturePointState>() {
            Some(s) => s,
            None => return,
        };

        for (_entity, cp) in cp_storage.iter() {
            self.total += 1;
            if cp.owner != 255 && (cp.owner as usize) < 8 {
                self.per_player[cp.owner as usize] += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, GameConfig};

    fn test_game_with_capture_points(cp_count: u8) -> Game {
        let mut game = Game::new(GameConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
        });

        spawn_capture_points(&mut game.world, cp_count, 64, 64, 42);
        game
    }

    #[test]
    fn test_capture_points_spawn_correct_count() {
        let game = test_game_with_capture_points(3);

        let cp_storage = game.world.get_storage::<CapturePointState>().unwrap();
        let count = cp_storage.iter().count();
        assert_eq!(count, 3, "Should spawn 3 capture points");
    }

    #[test]
    fn test_capture_points_spawn_odd_count() {
        // Even number should be bumped to odd
        let game = test_game_with_capture_points(4);

        let cp_storage = game.world.get_storage::<CapturePointState>().unwrap();
        let count = cp_storage.iter().count();
        assert_eq!(count, 5, "Even count 4 should become 5 (odd)");
    }

    #[test]
    fn test_capture_points_start_neutral() {
        let game = test_game_with_capture_points(3);

        let cp_storage = game.world.get_storage::<CapturePointState>().unwrap();
        for (_entity, cp) in cp_storage.iter() {
            assert_eq!(cp.owner, 255, "Capture points should start neutral");
            assert_eq!(cp.progress, 0.0);
            assert!(!cp.contested);
        }
    }

    #[test]
    fn test_capture_points_have_unique_indices() {
        let game = test_game_with_capture_points(5);

        let cp_storage = game.world.get_storage::<CapturePointState>().unwrap();
        let indices: Vec<u8> = cp_storage.iter().map(|(_, cp)| cp.point_index).collect();
        let mut sorted = indices.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(indices.len(), sorted.len(), "All capture point indices should be unique");
    }

    #[test]
    fn test_capture_points_spread_across_map() {
        let game = test_game_with_capture_points(3);

        let pos_storage = game.world.get_storage::<Position>().unwrap();
        let cp_storage = game.world.get_storage::<CapturePointState>().unwrap();

        let positions: Vec<(f32, f32)> = cp_storage.iter().filter_map(|(entity, _)| {
            let pos = pos_storage.get(entity)?;
            Some((pos.x, pos.y))
        }).collect();

        assert_eq!(positions.len(), 3);

        // Check that points are spread out (not all in same area)
        let mut max_dist = 0.0f32;
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let dx = positions[i].0 - positions[j].0;
                let dy = positions[i].1 - positions[j].1;
                max_dist = max_dist.max((dx * dx + dy * dy).sqrt());
            }
        }
        assert!(max_dist > 10.0, "Capture points should be spread across map, max_dist={}", max_dist);
    }

    #[test]
    fn test_single_player_captures_point() {
        let mut game = test_game_with_capture_points(3);

        // Find the center capture point position
        let (cp_entity, cp_x, cp_y) = {
            let cp_storage = game.world.get_storage::<CapturePointState>().unwrap();
            let pos_storage = game.world.get_storage::<Position>().unwrap();
            let (entity, _cp) = cp_storage.iter().find(|(_, cp)| cp.point_index == 0).unwrap();
            let pos = pos_storage.get(entity).unwrap();
            (entity, pos.x, pos.y)
        };

        // Place player 0 unit next to the capture point
        game.spawn_thrall(cp_x + 0.5, cp_y + 0.5, 0);

        // Tick for enough time to capture (100 progress / 5 speed = 20 seconds = 400 ticks)
        for _ in 0..420 {
            game.tick(50.0);
        }

        let cp = game.world.get_component::<CapturePointState>(cp_entity).unwrap();
        assert_eq!(cp.owner, 0, "Player 0 should own the capture point");
        assert_eq!(cp.progress, 100.0);
    }

    #[test]
    fn test_contested_pauses_capture() {
        let mut game = test_game_with_capture_points(3);

        // Find center capture point
        let (cp_entity, cp_x, cp_y) = {
            let cp_storage = game.world.get_storage::<CapturePointState>().unwrap();
            let pos_storage = game.world.get_storage::<Position>().unwrap();
            let (entity, _) = cp_storage.iter().find(|(_, cp)| cp.point_index == 0).unwrap();
            let pos = pos_storage.get(entity).unwrap();
            (entity, pos.x, pos.y)
        };

        // Both players have units near the point
        game.spawn_thrall(cp_x + 0.5, cp_y + 0.5, 0);
        game.spawn_thrall(cp_x - 0.5, cp_y - 0.5, 1);

        // Tick for a while
        for _ in 0..100 {
            game.tick(50.0);
        }

        let cp = game.world.get_component::<CapturePointState>(cp_entity).unwrap();
        assert!(cp.contested, "Point should be contested");
        assert_eq!(cp.progress, 0.0, "Progress should not advance when contested");
        assert_eq!(cp.owner, 255, "Point should remain neutral when contested");
    }

    #[test]
    fn test_capture_speed_scales_with_sqrt_count() {
        let mut game1 = test_game_with_capture_points(3);
        let mut game2 = test_game_with_capture_points(3);

        // Find center point positions for both games
        let (cp1_entity, cp1_x, cp1_y) = {
            let cp_storage = game1.world.get_storage::<CapturePointState>().unwrap();
            let pos_storage = game1.world.get_storage::<Position>().unwrap();
            let (entity, _) = cp_storage.iter().find(|(_, cp)| cp.point_index == 0).unwrap();
            let pos = pos_storage.get(entity).unwrap();
            (entity, pos.x, pos.y)
        };
        let (cp2_entity, cp2_x, cp2_y) = {
            let cp_storage = game2.world.get_storage::<CapturePointState>().unwrap();
            let pos_storage = game2.world.get_storage::<Position>().unwrap();
            let (entity, _) = cp_storage.iter().find(|(_, cp)| cp.point_index == 0).unwrap();
            let pos = pos_storage.get(entity).unwrap();
            (entity, pos.x, pos.y)
        };

        // Game 1: 1 unit
        game1.spawn_thrall(cp1_x + 0.5, cp1_y + 0.5, 0);

        // Game 2: 4 units (should be 2x speed due to sqrt(4) = 2)
        for i in 0..4 {
            game2.spawn_thrall(cp2_x + (i as f32) * 0.3, cp2_y + 0.5, 0);
        }

        // Tick both for same duration
        for _ in 0..100 {
            game1.tick(50.0);
            game2.tick(50.0);
        }

        let progress1 = game1.world.get_component::<CapturePointState>(cp1_entity).unwrap().progress;
        let progress2 = game2.world.get_component::<CapturePointState>(cp2_entity).unwrap().progress;

        // 4 units should have ~2x the progress of 1 unit
        assert!(progress2 > progress1 * 1.8,
            "4 units should capture ~2x faster than 1, got progress1={}, progress2={}", progress1, progress2);
    }

    #[test]
    fn test_owner_flips_at_100() {
        let mut game = test_game_with_capture_points(3);

        let (cp_entity, cp_x, cp_y) = {
            let cp_storage = game.world.get_storage::<CapturePointState>().unwrap();
            let pos_storage = game.world.get_storage::<Position>().unwrap();
            let (entity, _) = cp_storage.iter().find(|(_, cp)| cp.point_index == 0).unwrap();
            let pos = pos_storage.get(entity).unwrap();
            (entity, pos.x, pos.y)
        };

        // Many units to capture quickly
        for i in 0..6 {
            game.spawn_thrall(cp_x + (i as f32) * 0.3, cp_y + 0.5, 0);
        }

        // Before capture
        let cp = game.world.get_component::<CapturePointState>(cp_entity).unwrap();
        assert_eq!(cp.owner, 255);

        // Tick until captured
        for _ in 0..200 {
            game.tick(50.0);
        }

        let cp = game.world.get_component::<CapturePointState>(cp_entity).unwrap();
        assert_eq!(cp.owner, 0, "Owner should be player 0");
        assert_eq!(cp.progress, 100.0);
    }

    #[test]
    fn test_recapture_requires_neutralize_first() {
        let mut game = test_game_with_capture_points(3);

        let (cp_entity, cp_x, cp_y) = {
            let cp_storage = game.world.get_storage::<CapturePointState>().unwrap();
            let pos_storage = game.world.get_storage::<Position>().unwrap();
            let (entity, _) = cp_storage.iter().find(|(_, cp)| cp.point_index == 0).unwrap();
            let pos = pos_storage.get(entity).unwrap();
            (entity, pos.x, pos.y)
        };

        // Player 0 captures the point first
        let u0 = game.spawn_thrall(cp_x + 0.5, cp_y + 0.5, 0);
        for _ in 0..500 {
            game.tick(50.0);
        }
        assert_eq!(game.world.get_component::<CapturePointState>(cp_entity).unwrap().owner, 0);

        // Remove player 0's unit (kill it)
        if let Some(h) = game.world.get_component_mut::<Health>(u0) {
            h.current = 0.0;
        }
        game.tick(50.0); // death cleanup

        // Player 1 starts recapturing — must first neutralize
        game.spawn_thrall(cp_x + 0.5, cp_y + 0.5, 1);

        // Tick some — progress should decrease first (neutralize)
        for _ in 0..100 {
            game.tick(50.0);
        }

        let cp = game.world.get_component::<CapturePointState>(cp_entity).unwrap();
        // Progress should have decreased from 100
        assert!(cp.progress < 100.0, "Progress should decrease during recapture");
    }

    #[test]
    fn test_decay_without_units() {
        let mut game = test_game_with_capture_points(3);

        let (cp_entity, cp_x, cp_y) = {
            let cp_storage = game.world.get_storage::<CapturePointState>().unwrap();
            let pos_storage = game.world.get_storage::<Position>().unwrap();
            let (entity, _) = cp_storage.iter().find(|(_, cp)| cp.point_index == 0).unwrap();
            let pos = pos_storage.get(entity).unwrap();
            (entity, pos.x, pos.y)
        };

        // Start capturing
        let unit = game.spawn_thrall(cp_x + 0.5, cp_y + 0.5, 0);
        for _ in 0..50 {
            game.tick(50.0);
        }

        let progress_with_unit = game.world.get_component::<CapturePointState>(cp_entity).unwrap().progress;
        assert!(progress_with_unit > 0.0, "Should have some capture progress");

        // Kill the unit — progress should start decaying (point is still neutral)
        if let Some(h) = game.world.get_component_mut::<Health>(unit) {
            h.current = 0.0;
        }

        // Tick many times for decay
        for _ in 0..200 {
            game.tick(50.0);
        }

        let progress_after_decay = game.world.get_component::<CapturePointState>(cp_entity).unwrap().progress;
        assert!(progress_after_decay < progress_with_unit,
            "Progress should decay without units, before={}, after={}", progress_with_unit, progress_after_decay);
    }

    #[test]
    fn test_capture_complete_event_emitted() {
        let mut game = test_game_with_capture_points(3);

        let (_, cp_x, cp_y) = {
            let cp_storage = game.world.get_storage::<CapturePointState>().unwrap();
            let pos_storage = game.world.get_storage::<Position>().unwrap();
            let (entity, _) = cp_storage.iter().find(|(_, cp)| cp.point_index == 0).unwrap();
            let pos = pos_storage.get(entity).unwrap();
            (entity, pos.x, pos.y)
        };

        // Many units to capture quickly
        for i in 0..6 {
            game.spawn_thrall(cp_x + (i as f32) * 0.3, cp_y + 0.5, 0);
        }

        // Run until capture completes
        let mut found_complete = false;
        for _ in 0..500 {
            game.tick(50.0);

            // Check events this tick
            let ec = game.world.get_resource::<crate::game::EventCount>().unwrap().0;
            let eb = &game.world.get_resource::<crate::game::EventBuffer>().unwrap().0;
            for i in 0..ec as usize {
                let off = i * crate::game::EVENT_ENTRY_SIZE;
                let event_type = u16::from_le_bytes([eb[off], eb[off + 1]]);
                if event_type == EventType::CaptureComplete as u16 {
                    found_complete = true;
                    break;
                }
            }
            if found_complete {
                break;
            }
        }

        assert!(found_complete, "Should emit CaptureComplete event when point is captured");
    }

    #[test]
    fn test_capture_point_counts_resource() {
        let mut game = test_game_with_capture_points(3);

        let mut counts = CapturePointCounts::new();
        counts.update(&game.world);
        assert_eq!(counts.total, 3);
        assert_eq!(counts.per_player[0], 0);
        assert_eq!(counts.per_player[1], 0);

        // Capture one point
        let (_, cp_x, cp_y) = {
            let cp_storage = game.world.get_storage::<CapturePointState>().unwrap();
            let pos_storage = game.world.get_storage::<Position>().unwrap();
            let (entity, _) = cp_storage.iter().find(|(_, cp)| cp.point_index == 0).unwrap();
            let pos = pos_storage.get(entity).unwrap();
            (entity, pos.x, pos.y)
        };

        for i in 0..6 {
            game.spawn_thrall(cp_x + (i as f32) * 0.3, cp_y + 0.5, 0);
        }
        for _ in 0..500 {
            game.tick(50.0);
        }

        counts.update(&game.world);
        assert_eq!(counts.total, 3);
        assert_eq!(counts.per_player[0], 1, "Player 0 should own 1 capture point");
    }
}

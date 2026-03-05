use crate::ecs::World;
use crate::components::{Position, UnitType, VisionRange};
use crate::game::TickDelta;

/// Fog of war visibility states per tile.
pub const FOG_UNEXPLORED: u8 = 0;
pub const FOG_EXPLORED: u8 = 1;
pub const FOG_VISIBLE: u8 = 2;

/// Per-player fog of war grid. Each byte is a tile's visibility state.
pub struct FogGrid {
    /// Flat array: grids[player_index][y * width + x]
    pub grids: Vec<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub player_count: u32,
}

impl FogGrid {
    pub fn new(width: u32, height: u32, player_count: u32) -> Self {
        let size = (width * height) as usize;
        let grids = (0..player_count)
            .map(|_| vec![FOG_UNEXPLORED; size])
            .collect();
        FogGrid { grids, width, height, player_count }
    }

    /// Get visibility state for a tile for a specific player.
    #[inline]
    pub fn get(&self, player: u32, x: u32, y: u32) -> u8 {
        if player >= self.player_count || x >= self.width || y >= self.height {
            return FOG_UNEXPLORED;
        }
        self.grids[player as usize][(y * self.width + x) as usize]
    }

    /// Set visibility state for a tile for a specific player.
    #[inline]
    pub fn set(&mut self, player: u32, x: u32, y: u32, state: u8) {
        if player >= self.player_count || x >= self.width || y >= self.height {
            return;
        }
        self.grids[player as usize][(y * self.width + x) as usize] = state;
    }

    /// Get a pointer to a player's fog grid for WASM export.
    pub fn grid_ptr(&self, player: u32) -> *const u8 {
        if player >= self.player_count {
            return std::ptr::null();
        }
        self.grids[player as usize].as_ptr()
    }

    /// Get the byte length of one player's fog grid.
    pub fn grid_len(&self) -> u32 {
        self.width * self.height
    }
}

pub fn fog_system(world: &mut World) {
    let _delta = if let Some(td) = world.get_resource::<TickDelta>() {
        td.0
    } else {
        return;
    };

    // Collect all entities with vision (Position + UnitType + VisionRange)
    let vision_entities: Vec<(f32, f32, u8, f32)> = {
        let pos_storage = world.get_storage::<Position>();
        let ut_storage = world.get_storage::<UnitType>();
        let vr_storage = world.get_storage::<VisionRange>();

        if pos_storage.is_none() || ut_storage.is_none() || vr_storage.is_none() {
            return;
        }

        let pos_s = pos_storage.unwrap();
        let ut_s = ut_storage.unwrap();
        let vr_s = vr_storage.unwrap();

        pos_s.iter()
            .filter_map(|(entity, pos)| {
                let ut = ut_s.get(entity)?;
                let vr = vr_s.get(entity)?;
                if vr.0 <= 0.0 {
                    return None; // No vision
                }
                Some((pos.x, pos.y, ut.owner, vr.0))
            })
            .collect()
    };

    let fog = if let Some(fg) = world.get_resource_mut::<FogGrid>() {
        fg
    } else {
        return;
    };

    let width = fog.width;
    let height = fog.height;
    let player_count = fog.player_count;

    // Phase 1: Downgrade all Visible tiles to Explored for each player
    for p in 0..player_count as usize {
        for cell in fog.grids[p].iter_mut() {
            if *cell == FOG_VISIBLE {
                *cell = FOG_EXPLORED;
            }
        }
    }

    // Phase 2: For each unit, mark tiles within vision radius as Visible
    for (ux, uy, owner, vision_range) in &vision_entities {
        let player = *owner as u32;
        if player >= player_count {
            continue;
        }

        let cx = *ux;
        let cy = *uy;
        let r = *vision_range;
        let r_sq = r * r;

        // Bounded iteration: only check tiles within the vision radius bounding box
        let min_x = ((cx - r).floor().max(0.0)) as u32;
        let max_x = ((cx + r).ceil().min(width as f32 - 1.0)) as u32;
        let min_y = ((cy - r).floor().max(0.0)) as u32;
        let max_y = ((cy + r).ceil().min(height as f32 - 1.0)) as u32;

        let grid = &mut fog.grids[player as usize];

        for ty in min_y..=max_y {
            for tx in min_x..=max_x {
                // Use tile center (tx + 0.5, ty + 0.5) for distance check
                let dx = (tx as f32 + 0.5) - cx;
                let dy = (ty as f32 + 0.5) - cy;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq <= r_sq {
                    let idx = (ty * width + tx) as usize;
                    grid[idx] = FOG_VISIBLE;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, GameConfig};

    fn test_game() -> Game {
        Game::new(GameConfig {
            map_width: 16,
            map_height: 16,
            player_count: 2,
            seed: 42,
        })
    }

    #[test]
    fn test_fog_starts_unexplored() {
        let grid = FogGrid::new(16, 16, 2);
        for p in 0..2u32 {
            for y in 0..16u32 {
                for x in 0..16u32 {
                    assert_eq!(grid.get(p, x, y), FOG_UNEXPLORED);
                }
            }
        }
    }

    #[test]
    fn test_unit_reveals_fog() {
        let mut game = test_game();
        // Add FogGrid resource
        game.world.insert_resource(FogGrid::new(16, 16, 2));
        // Register fog system
        game.systems.add_system("fog", fog_system);

        // Spawn a Thrall at (8.5, 8.5) with vision range 8
        game.spawn_thrall(8.5, 8.5, 0);
        game.tick(50.0);

        let fog = game.world.get_resource::<FogGrid>().unwrap();
        // The tile at (8, 8) should be visible
        assert_eq!(fog.get(0, 8, 8), FOG_VISIBLE);
        // A tile near the unit should be visible
        assert_eq!(fog.get(0, 9, 8), FOG_VISIBLE);
        // A tile far away should still be unexplored
        assert_eq!(fog.get(0, 0, 0), FOG_UNEXPLORED);
    }

    #[test]
    fn test_fog_explored_persists() {
        let mut game = test_game();
        game.world.insert_resource(FogGrid::new(16, 16, 2));
        game.systems.add_system("fog", fog_system);

        let entity = game.spawn_thrall(8.5, 8.5, 0);
        game.tick(50.0);

        let fog = game.world.get_resource::<FogGrid>().unwrap();
        assert_eq!(fog.get(0, 8, 8), FOG_VISIBLE);

        // Move the unit far away by directly setting position
        let _ = fog;
        if let Some(pos) = game.world.get_component_mut::<Position>(entity) {
            pos.x = 2.5;
            pos.y = 2.5;
        }

        game.tick(50.0);

        let fog = game.world.get_resource::<FogGrid>().unwrap();
        // Old position should now be Explored (not Unexplored)
        assert_eq!(fog.get(0, 8, 8), FOG_EXPLORED);
        // New position should be Visible
        assert_eq!(fog.get(0, 2, 2), FOG_VISIBLE);
    }

    #[test]
    fn test_fog_circle_shape() {
        let mut game = test_game();
        game.world.insert_resource(FogGrid::new(16, 16, 2));
        game.systems.add_system("fog", fog_system);

        // Spawn unit with known vision range
        game.spawn_thrall(8.5, 8.5, 0);
        game.tick(50.0);

        let fog = game.world.get_resource::<FogGrid>().unwrap();
        // Vision range = 8. Tile at corner (0.5, 0.5) is sqrt((8)^2 + (8)^2) = 11.3 away > 8
        assert_eq!(fog.get(0, 0, 0), FOG_UNEXPLORED, "Corner should be outside vision circle");
        // Tile at (1, 8) is 7 tiles away horizontally, should be visible
        assert_eq!(fog.get(0, 1, 8), FOG_VISIBLE, "Tile 7 tiles away should be visible");
    }

    #[test]
    fn test_enemy_player_fog_separate() {
        let mut game = test_game();
        game.world.insert_resource(FogGrid::new(16, 16, 2));
        game.systems.add_system("fog", fog_system);

        // Player 0 unit at (4, 4)
        game.spawn_thrall(4.5, 4.5, 0);
        // Player 1 unit at (12, 12)
        game.spawn_thrall(12.5, 12.5, 1);

        game.tick(50.0);

        let fog = game.world.get_resource::<FogGrid>().unwrap();
        // Player 0 sees around (4,4) but not (12,12)
        assert_eq!(fog.get(0, 4, 4), FOG_VISIBLE);
        assert_eq!(fog.get(0, 12, 12), FOG_UNEXPLORED);
        // Player 1 sees around (12,12) but not (4,4)
        assert_eq!(fog.get(1, 12, 12), FOG_VISIBLE);
        assert_eq!(fog.get(1, 4, 4), FOG_UNEXPLORED);
    }
}

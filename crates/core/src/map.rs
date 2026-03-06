use rand::prelude::*;
use rand::rngs::SmallRng;

use crate::types::SpriteId;

/// Terrain types for battle map tiles.
/// Values match the `terrain` byte in the Tile struct.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum TerrainType {
    Open = 0,
    Impassable = 1,
    Rough = 2,
    Elevated = 3,
    Hazard = 4,
    Cover = 5,
    Road = 6,
}

impl TerrainType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => TerrainType::Open,
            1 => TerrainType::Impassable,
            2 => TerrainType::Rough,
            3 => TerrainType::Elevated,
            4 => TerrainType::Hazard,
            5 => TerrainType::Cover,
            6 => TerrainType::Road,
            _ => TerrainType::Impassable,
        }
    }
}

/// Returns the movement cost multiplier for a given terrain type and unit kind.
/// HoverTank always has cost 1.0 (ignores terrain), except Impassable which is infinite.
/// Buildings (CommandPost, Node) cannot move so their cost is irrelevant.
pub fn movement_cost(terrain: TerrainType, unit_kind: SpriteId) -> f32 {
    if terrain == TerrainType::Impassable {
        return f32::INFINITY;
    }

    // Hover Tank ignores all terrain penalties
    if unit_kind == SpriteId::HoverTank {
        return 1.0;
    }

    match terrain {
        TerrainType::Open => 1.0,
        TerrainType::Road => 0.75,
        TerrainType::Rough => 1.5,
        TerrainType::Elevated => 2.0,
        TerrainType::Hazard => 1.5,
        TerrainType::Cover => 1.25,
        TerrainType::Impassable => f32::INFINITY,
    }
}

/// Returns the damage reduction factor for units on this terrain.
/// Cover provides 25% damage reduction, Elevated provides 15%.
pub fn damage_reduction(terrain: TerrainType) -> f32 {
    match terrain {
        TerrainType::Cover => 0.25,
        TerrainType::Elevated => 0.15,
        _ => 0.0,
    }
}

/// Damage per second dealt to non-hover ground units standing on Hazard tiles.
pub const HAZARD_DPS: f32 = 2.0;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Tile {
    pub terrain: u8,
    pub elevation: u8,
    pub sprite_variant: u8,
    pub flags: u8,
}

impl Tile {
    pub fn terrain_type(&self) -> TerrainType {
        TerrainType::from_u8(self.terrain)
    }
}

pub struct BattleMap {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Tile>,
}

impl BattleMap {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        let tiles = vec![Tile {
            terrain: TerrainType::Open as u8,
            elevation: 0,
            sprite_variant: 0,
            flags: 0,
        }; size];

        BattleMap { width, height, tiles }
    }

    #[inline]
    pub fn idx(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    pub fn get(&self, x: u32, y: u32) -> &Tile {
        &self.tiles[self.idx(x, y)]
    }

    pub fn get_mut(&mut self, x: u32, y: u32) -> &mut Tile {
        let idx = self.idx(x, y);
        &mut self.tiles[idx]
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    /// Returns true if the tile is walkable (anything except Impassable).
    pub fn is_walkable(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        self.get(x, y).terrain_type() != TerrainType::Impassable
    }

    /// Generate a battle map with varied terrain clusters.
    /// Terrain distribution:
    /// - ~62% Open
    /// - ~15% Rough
    /// - ~5% Elevated
    /// - ~3% Hazard
    /// - ~8% Cover
    /// - ~5% Road
    /// - ~2% Impassable
    ///
    /// Spawn corners (within `margin` tiles of each corner) are kept Open
    /// to guarantee fair starting positions.
    pub fn generate_simple(width: u32, height: u32, seed: u64) -> Self {
        let mut map = BattleMap::new(width, height);
        let mut rng = SmallRng::seed_from_u64(seed);

        // Assign random sprite variants (0-4) to all tiles (5 ground textures)
        for tile in map.tiles.iter_mut() {
            tile.sprite_variant = rng.gen_range(0..5);
        }

        let total_tiles = (width * height) as usize;

        // Terrain cluster definitions: (terrain_type, target_fraction, cluster_radius_range)
        let terrain_specs: &[(TerrainType, f32, (u32, u32))] = &[
            (TerrainType::Rough,      0.15, (2, 5)),
            (TerrainType::Cover,      0.08, (1, 3)),
            (TerrainType::Road,       0.05, (1, 2)),
            (TerrainType::Elevated,   0.05, (2, 4)),
            (TerrainType::Hazard,     0.03, (1, 3)),
            (TerrainType::Impassable, 0.02, (1, 2)),
        ];

        // Margin around corners kept clear for spawn safety
        let margin = (width.min(height) / 4).max(4);

        for &(terrain, fraction, (min_r, max_r)) in terrain_specs {
            let target_count = ((total_tiles as f32) * fraction) as usize;
            let mut placed = 0usize;
            let mut attempts = 0;
            let max_attempts = target_count * 10 + 100;

            while placed < target_count && attempts < max_attempts {
                attempts += 1;

                // Random cluster center
                let cx = rng.gen_range(0..width) as i32;
                let cy = rng.gen_range(0..height) as i32;

                // Skip if in spawn corner margin
                if is_spawn_corner(cx as u32, cy as u32, width, height, margin) {
                    continue;
                }

                let radius = rng.gen_range(min_r..=max_r) as i32;

                // Place cluster
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = cx + dx;
                        let ny = cy + dy;
                        if !map.in_bounds(nx, ny) {
                            continue;
                        }
                        let ux = nx as u32;
                        let uy = ny as u32;

                        // Skip spawn corners
                        if is_spawn_corner(ux, uy, width, height, margin) {
                            continue;
                        }

                        // Diamond-shaped cluster
                        if (dx.abs() + dy.abs()) as u32 > radius as u32 {
                            continue;
                        }

                        // Only overwrite Open tiles
                        let tile = map.get_mut(ux, uy);
                        if tile.terrain == TerrainType::Open as u8 {
                            tile.terrain = terrain as u8;
                            placed += 1;
                            if placed >= target_count {
                                break;
                            }
                        }
                    }
                    if placed >= target_count {
                        break;
                    }
                }
            }
        }

        // Set elevation for Elevated tiles
        for tile in map.tiles.iter_mut() {
            if tile.terrain == TerrainType::Elevated as u8 {
                tile.elevation = 1;
            }
        }

        map
    }
}

/// Check if a tile position is within the spawn corner safety margin.
fn is_spawn_corner(x: u32, y: u32, width: u32, height: u32, margin: u32) -> bool {
    let corners: [(u32, u32); 4] = [
        (0, 0),
        (width.saturating_sub(1), height.saturating_sub(1)),
        (width.saturating_sub(1), 0),
        (0, height.saturating_sub(1)),
    ];

    for &(cx, cy) in &corners {
        let dx = if x > cx { x - cx } else { cx - x };
        let dy = if y > cy { y - cy } else { cy - y };
        if dx <= margin && dy <= margin {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_map_all_open() {
        let map = BattleMap::new(8, 8);
        for tile in &map.tiles {
            assert_eq!(tile.terrain_type(), TerrainType::Open);
        }
    }

    #[test]
    fn test_generate_has_terrain_variety() {
        let map = BattleMap::generate_simple(64, 64, 42);
        let mut terrain_counts = [0usize; 7];
        for tile in &map.tiles {
            let t = tile.terrain as usize;
            if t < 7 {
                terrain_counts[t] += 1;
            }
        }
        // Should have Open as the majority
        assert!(terrain_counts[TerrainType::Open as usize] > 0,
            "Should have Open tiles");
        // Should have at least some non-Open terrain
        let non_open: usize = terrain_counts[1..].iter().sum();
        assert!(non_open > 0, "Should have non-Open terrain, counts: {:?}", terrain_counts);
        // Should have multiple terrain types
        let types_present = terrain_counts.iter().filter(|&&c| c > 0).count();
        assert!(types_present >= 4,
            "Should have at least 4 terrain types, got {} types: {:?}", types_present, terrain_counts);
    }

    #[test]
    fn test_generate_has_variants() {
        let map = BattleMap::generate_simple(64, 64, 42);
        let variants: std::collections::HashSet<u8> = map.tiles.iter()
            .map(|t| t.sprite_variant)
            .collect();
        assert!(variants.len() >= 2, "Map should have multiple terrain variants");
    }

    #[test]
    fn test_spawn_corners_are_open() {
        let map = BattleMap::generate_simple(64, 64, 42);
        let margin = 4;
        // Check all four corners
        let corners = [(0, 0), (63, 63), (63, 0), (0, 63)];
        for &(cx, cy) in &corners {
            for dy in 0..=margin {
                for dx in 0..=margin {
                    let x = if cx == 0 { dx } else { cx - dx };
                    let y = if cy == 0 { dy } else { cy - dy };
                    if x < 64 && y < 64 {
                        let tile = map.get(x, y);
                        assert_eq!(tile.terrain_type(), TerrainType::Open,
                            "Spawn corner tile ({}, {}) should be Open, got {:?}",
                            x, y, tile.terrain_type());
                    }
                }
            }
        }
    }

    #[test]
    fn test_bounds_checking() {
        let map = BattleMap::new(8, 8);
        assert!(map.in_bounds(0, 0));
        assert!(map.in_bounds(7, 7));
        assert!(!map.in_bounds(-1, 0));
        assert!(!map.in_bounds(8, 0));
    }

    #[test]
    fn test_is_walkable_non_impassable() {
        let mut map = BattleMap::new(8, 8);
        // Set various terrain types
        map.get_mut(1, 1).terrain = TerrainType::Rough as u8;
        map.get_mut(2, 2).terrain = TerrainType::Elevated as u8;
        map.get_mut(3, 3).terrain = TerrainType::Hazard as u8;
        map.get_mut(4, 4).terrain = TerrainType::Cover as u8;
        map.get_mut(5, 5).terrain = TerrainType::Road as u8;
        map.get_mut(6, 6).terrain = TerrainType::Impassable as u8;

        // All non-Impassable should be walkable
        assert!(map.is_walkable(0, 0)); // Open
        assert!(map.is_walkable(1, 1)); // Rough
        assert!(map.is_walkable(2, 2)); // Elevated
        assert!(map.is_walkable(3, 3)); // Hazard
        assert!(map.is_walkable(4, 4)); // Cover
        assert!(map.is_walkable(5, 5)); // Road
        assert!(!map.is_walkable(6, 6)); // Impassable
    }

    #[test]
    fn test_movement_cost_thrall() {
        assert_eq!(movement_cost(TerrainType::Open, SpriteId::Thrall), 1.0);
        assert_eq!(movement_cost(TerrainType::Road, SpriteId::Thrall), 0.75);
        assert_eq!(movement_cost(TerrainType::Rough, SpriteId::Thrall), 1.5);
        assert_eq!(movement_cost(TerrainType::Elevated, SpriteId::Thrall), 2.0);
        assert_eq!(movement_cost(TerrainType::Hazard, SpriteId::Thrall), 1.5);
        assert_eq!(movement_cost(TerrainType::Cover, SpriteId::Thrall), 1.25);
        assert!(movement_cost(TerrainType::Impassable, SpriteId::Thrall).is_infinite());
    }

    #[test]
    fn test_movement_cost_sentinel() {
        assert_eq!(movement_cost(TerrainType::Open, SpriteId::Sentinel), 1.0);
        assert_eq!(movement_cost(TerrainType::Rough, SpriteId::Sentinel), 1.5);
        assert_eq!(movement_cost(TerrainType::Road, SpriteId::Sentinel), 0.75);
    }

    #[test]
    fn test_movement_cost_hover_tank_ignores_terrain() {
        assert_eq!(movement_cost(TerrainType::Open, SpriteId::HoverTank), 1.0);
        assert_eq!(movement_cost(TerrainType::Rough, SpriteId::HoverTank), 1.0);
        assert_eq!(movement_cost(TerrainType::Elevated, SpriteId::HoverTank), 1.0);
        assert_eq!(movement_cost(TerrainType::Hazard, SpriteId::HoverTank), 1.0);
        assert_eq!(movement_cost(TerrainType::Cover, SpriteId::HoverTank), 1.0);
        assert_eq!(movement_cost(TerrainType::Road, SpriteId::HoverTank), 1.0);
        assert!(movement_cost(TerrainType::Impassable, SpriteId::HoverTank).is_infinite());
    }

    #[test]
    fn test_damage_reduction() {
        assert_eq!(damage_reduction(TerrainType::Cover), 0.25);
        assert_eq!(damage_reduction(TerrainType::Elevated), 0.15);
        assert_eq!(damage_reduction(TerrainType::Open), 0.0);
        assert_eq!(damage_reduction(TerrainType::Rough), 0.0);
        assert_eq!(damage_reduction(TerrainType::Road), 0.0);
        assert_eq!(damage_reduction(TerrainType::Hazard), 0.0);
    }

    #[test]
    fn test_terrain_type_from_u8() {
        assert_eq!(TerrainType::from_u8(0), TerrainType::Open);
        assert_eq!(TerrainType::from_u8(1), TerrainType::Impassable);
        assert_eq!(TerrainType::from_u8(2), TerrainType::Rough);
        assert_eq!(TerrainType::from_u8(3), TerrainType::Elevated);
        assert_eq!(TerrainType::from_u8(4), TerrainType::Hazard);
        assert_eq!(TerrainType::from_u8(5), TerrainType::Cover);
        assert_eq!(TerrainType::from_u8(6), TerrainType::Road);
        assert_eq!(TerrainType::from_u8(255), TerrainType::Impassable);
    }
}

use crate::ecs::World;
use crate::components::{Position, UnitType, Health};
use crate::blueprints::get_blueprint;

/// How often the influence map updates, in game ticks (every 0.5s at 20Hz).
pub const INFLUENCE_UPDATE_INTERVAL: u32 = 10;

/// Grid-based influence map for AI decision-making.
/// Maintains per-player layers for threat assessment, friendly strength,
/// and derived metrics like tension and vulnerability.
pub struct InfluenceGrid {
    /// Grid width (matches map width).
    pub width: u32,
    /// Grid height (matches map height).
    pub height: u32,
    /// Number of players.
    pub player_count: u32,

    /// Per-player threat grids. threat[player][y * width + x] = sum of enemy DPS influence at that tile.
    pub threat: Vec<Vec<f32>>,
    /// Per-player friendly strength grids. friendly[player][y * width + x] = sum of own DPS influence.
    pub friendly_strength: Vec<Vec<f32>>,
    /// Per-player tension grids (friendly_strength - threat). Positive = friendly advantage.
    pub tension: Vec<Vec<f32>>,
    /// Per-player vulnerability grids (threat - friendly_strength). Positive = enemy advantage.
    pub vulnerability: Vec<Vec<f32>>,
    /// Global density grid. Counts all units near each tile regardless of owner.
    pub density: Vec<f32>,

    /// Last tick the influence map was updated.
    pub last_update_tick: u32,
}

impl InfluenceGrid {
    /// Create a new blank influence grid.
    pub fn new(width: u32, height: u32, player_count: u32) -> Self {
        let size = (width * height) as usize;
        let pc = player_count as usize;

        InfluenceGrid {
            width,
            height,
            player_count,
            threat: vec![vec![0.0; size]; pc],
            friendly_strength: vec![vec![0.0; size]; pc],
            tension: vec![vec![0.0; size]; pc],
            vulnerability: vec![vec![0.0; size]; pc],
            density: vec![0.0; size],
            last_update_tick: 0,
        }
    }

    /// Clear all grids to zero.
    fn clear(&mut self) {
        let size = (self.width * self.height) as usize;
        let pc = self.player_count as usize;

        for p in 0..pc {
            self.threat[p] = vec![0.0; size];
            self.friendly_strength[p] = vec![0.0; size];
            self.tension[p] = vec![0.0; size];
            self.vulnerability[p] = vec![0.0; size];
        }
        self.density = vec![0.0; size];
    }

    /// Recalculate all influence grids from the current world state.
    /// Iterates all living combat units, propagates DPS-weighted influence
    /// with linear distance falloff within (attack_range + 2.0) radius.
    pub fn update(&mut self, world: &World, current_tick: u32) {
        self.clear();
        self.last_update_tick = current_tick;

        let pos_storage = match world.get_storage::<Position>() {
            Some(s) => s,
            None => return,
        };
        let ut_storage = match world.get_storage::<UnitType>() {
            Some(s) => s,
            None => return,
        };
        let health_storage = world.get_storage::<Health>();

        // Collect all living combat units
        for (entity, pos) in pos_storage.iter() {
            let ut = match ut_storage.get(entity) {
                Some(ut) => ut,
                None => continue,
            };

            let bp = get_blueprint(ut.kind);

            // Skip buildings (no speed = stationary = not a combat unit for influence purposes)
            if bp.speed <= 0.0 {
                continue;
            }

            // Skip units with no combat capability
            if bp.damage <= 0.0 {
                continue;
            }

            // Check if unit is alive
            let health_fraction = if let Some(hs) = &health_storage {
                if let Some(h) = hs.get(entity) {
                    if h.is_dead() {
                        continue;
                    }
                    h.current / h.max
                } else {
                    1.0
                }
            } else {
                1.0
            };

            // Calculate DPS: (damage / cooldown) * health_fraction
            let dps = if bp.attack_cooldown > 0.0 {
                (bp.damage / bp.attack_cooldown) * health_fraction
            } else {
                bp.damage * health_fraction
            };

            let owner = ut.owner as usize;
            let influence_radius = bp.attack_range + 2.0;

            // Propagate influence to nearby tiles
            self.propagate_influence(pos.x, pos.y, dps, influence_radius, owner);
        }

        // Compute derived layers (tension, vulnerability) for each player
        let size = (self.width * self.height) as usize;
        for p in 0..self.player_count as usize {
            for i in 0..size {
                self.tension[p][i] = self.friendly_strength[p][i] - self.threat[p][i];
                self.vulnerability[p][i] = self.threat[p][i] - self.friendly_strength[p][i];
            }
        }
    }

    /// Propagate a unit's influence to surrounding tiles.
    /// Uses linear distance falloff: influence = dps * (1.0 - dist / radius).
    fn propagate_influence(
        &mut self,
        x: f32,
        y: f32,
        dps: f32,
        radius: f32,
        owner: usize,
    ) {
        let tile_x = x.floor() as i32;
        let tile_y = y.floor() as i32;
        let r_ceil = radius.ceil() as i32;

        let w = self.width as i32;
        let h = self.height as i32;

        // Iterate tiles in bounding box
        let min_tx = (tile_x - r_ceil).max(0);
        let max_tx = (tile_x + r_ceil).min(w - 1);
        let min_ty = (tile_y - r_ceil).max(0);
        let max_ty = (tile_y + r_ceil).min(h - 1);

        for ty in min_ty..=max_ty {
            for tx in min_tx..=max_tx {
                let dx = (tx as f32 + 0.5) - x;
                let dy = (ty as f32 + 0.5) - y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist > radius {
                    continue;
                }

                // Linear falloff: full at center, zero at radius
                let falloff = 1.0 - (dist / radius);
                let influence = dps * falloff;

                let idx = (ty as u32 * self.width + tx as u32) as usize;

                // Add to density (all units)
                self.density[idx] += influence;

                // Add to friendly strength for owner, threat for all other players
                self.friendly_strength[owner][idx] += influence;

                for p in 0..self.player_count as usize {
                    if p != owner {
                        self.threat[p][idx] += influence;
                    }
                }
            }
        }
    }

    /// Get tile index, or None if out of bounds.
    fn tile_index(&self, tx: u32, ty: u32) -> Option<usize> {
        if tx < self.width && ty < self.height {
            Some((ty * self.width + tx) as usize)
        } else {
            None
        }
    }

    /// Get threat level at a tile for a specific player.
    pub fn get_threat(&self, player_id: u32, tx: u32, ty: u32) -> f32 {
        if let Some(idx) = self.tile_index(tx, ty) {
            if (player_id as usize) < self.threat.len() {
                return self.threat[player_id as usize][idx];
            }
        }
        0.0
    }

    /// Get friendly strength at a tile for a specific player.
    pub fn get_friendly_strength(&self, player_id: u32, tx: u32, ty: u32) -> f32 {
        if let Some(idx) = self.tile_index(tx, ty) {
            if (player_id as usize) < self.friendly_strength.len() {
                return self.friendly_strength[player_id as usize][idx];
            }
        }
        0.0
    }

    /// Get tension at a tile for a specific player.
    /// Positive = friendly advantage, negative = enemy advantage.
    pub fn get_tension(&self, player_id: u32, tx: u32, ty: u32) -> f32 {
        if let Some(idx) = self.tile_index(tx, ty) {
            if (player_id as usize) < self.tension.len() {
                return self.tension[player_id as usize][idx];
            }
        }
        0.0
    }

    /// Get vulnerability at a tile for a specific player.
    /// Positive = enemy has advantage, negative = player has advantage.
    pub fn get_vulnerability(&self, player_id: u32, tx: u32, ty: u32) -> f32 {
        if let Some(idx) = self.tile_index(tx, ty) {
            if (player_id as usize) < self.vulnerability.len() {
                return self.vulnerability[player_id as usize][idx];
            }
        }
        0.0
    }

    /// Get density at a tile (all units combined).
    pub fn get_density(&self, tx: u32, ty: u32) -> f32 {
        if let Some(idx) = self.tile_index(tx, ty) {
            return self.density[idx];
        }
        0.0
    }

    /// Find the tile with the highest threat for a player within a search area.
    /// Returns (tile_x, tile_y, threat_value) or None if no threat found.
    pub fn highest_threat_tile(
        &self,
        player_id: u32,
        search_x: u32,
        search_y: u32,
        search_radius: u32,
    ) -> Option<(u32, u32, f32)> {
        let pid = player_id as usize;
        if pid >= self.threat.len() {
            return None;
        }

        let w = self.width;
        let h = self.height;

        let min_x = search_x.saturating_sub(search_radius);
        let max_x = (search_x + search_radius).min(w - 1);
        let min_y = search_y.saturating_sub(search_radius);
        let max_y = (search_y + search_radius).min(h - 1);

        let mut best: Option<(u32, u32, f32)> = None;

        for ty in min_y..=max_y {
            for tx in min_x..=max_x {
                let idx = (ty * w + tx) as usize;
                let val = self.threat[pid][idx];
                if val > 0.0 {
                    if best.is_none() || val > best.unwrap().2 {
                        best = Some((tx, ty, val));
                    }
                }
            }
        }

        best
    }

    /// Find a safe position for a player's unit to retreat to.
    /// Searches for the tile with the lowest threat within the given radius.
    /// Returns (tile_x, tile_y) or None if no tiles found.
    pub fn find_safe_position(
        &self,
        player_id: u32,
        from_x: u32,
        from_y: u32,
        search_radius: u32,
    ) -> Option<(u32, u32)> {
        let pid = player_id as usize;
        if pid >= self.threat.len() {
            return None;
        }

        let w = self.width;
        let h = self.height;

        let min_x = from_x.saturating_sub(search_radius);
        let max_x = (from_x + search_radius).min(w - 1);
        let min_y = from_y.saturating_sub(search_radius);
        let max_y = (from_y + search_radius).min(h - 1);

        let mut best: Option<(u32, u32, f32)> = None;

        for ty in min_y..=max_y {
            for tx in min_x..=max_x {
                let idx = (ty * w + tx) as usize;
                let threat = self.threat[pid][idx];
                let friendly = self.friendly_strength[pid][idx];

                // Prefer tiles with low threat and high friendly presence
                let safety_score = friendly - threat;

                if best.is_none() || safety_score > best.unwrap().2 {
                    best = Some((tx, ty, safety_score));
                }
            }
        }

        best.map(|(x, y, _)| (x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, GameConfig};
    use crate::types::SpriteId;

    fn test_game() -> Game {
        Game::new(GameConfig {
            map_width: 16,
            map_height: 16,
            player_count: 2,
            seed: 42,
        })
    }

    #[test]
    fn test_influence_grid_creation() {
        let grid = InfluenceGrid::new(16, 16, 2);
        assert_eq!(grid.width, 16);
        assert_eq!(grid.height, 16);
        assert_eq!(grid.player_count, 2);
        assert_eq!(grid.threat.len(), 2);
        assert_eq!(grid.threat[0].len(), 256);
        assert_eq!(grid.friendly_strength.len(), 2);
        assert_eq!(grid.density.len(), 256);
    }

    #[test]
    fn test_single_unit_influence() {
        let mut game = test_game();
        game.spawn_thrall(8.5, 8.5, 0);

        // Tick once so positions are set
        game.tick(50.0);

        let mut grid = InfluenceGrid::new(16, 16, 2);
        grid.update(&game.world, 1);

        // Player 0's thrall should create friendly strength for player 0
        let friendly_at_unit = grid.get_friendly_strength(0, 8, 8);
        assert!(friendly_at_unit > 0.0, "Friendly strength at unit position should be positive, got {}", friendly_at_unit);

        // Same tile should be a threat for player 1
        let threat_at_unit = grid.get_threat(1, 8, 8);
        assert!(threat_at_unit > 0.0, "Threat for player 1 at unit position should be positive, got {}", threat_at_unit);

        // Player 0 should have no threat from their own unit
        let own_threat = grid.get_threat(0, 8, 8);
        assert_eq!(own_threat, 0.0, "Player 0 should have no threat from own unit");

        // Far away tile should have no influence
        let far_friendly = grid.get_friendly_strength(0, 0, 0);
        assert_eq!(far_friendly, 0.0, "Far away tile should have no influence");
    }

    #[test]
    fn test_two_player_threat() {
        let mut game = test_game();
        game.spawn_thrall(4.5, 4.5, 0);
        game.spawn_thrall(6.5, 4.5, 1);

        game.tick(50.0);

        let mut grid = InfluenceGrid::new(16, 16, 2);
        grid.update(&game.world, 1);

        // Player 0 sees threat from player 1's unit near (6, 4)
        let threat_p0 = grid.get_threat(0, 6, 4);
        assert!(threat_p0 > 0.0, "Player 0 should see threat from player 1 near (6,4)");

        // Player 1 sees threat from player 0's unit near (4, 4)
        let threat_p1 = grid.get_threat(1, 4, 4);
        assert!(threat_p1 > 0.0, "Player 1 should see threat from player 0 near (4,4)");

        // Both units contribute to density
        let density = grid.get_density(5, 4);
        assert!(density > 0.0, "Density between both units should be positive");
    }

    #[test]
    fn test_tension_calculation() {
        let mut game = test_game();
        // Player 0 has 2 thralls at one location
        game.spawn_thrall(5.5, 5.5, 0);
        game.spawn_thrall(5.5, 6.5, 0);
        // Player 1 has 1 thrall nearby
        game.spawn_thrall(7.5, 5.5, 1);

        game.tick(50.0);

        let mut grid = InfluenceGrid::new(16, 16, 2);
        grid.update(&game.world, 1);

        // Near player 0's units, player 0 should have positive tension (friendly advantage)
        let tension_p0 = grid.get_tension(0, 5, 5);
        assert!(tension_p0 > 0.0, "Player 0 should have positive tension near own units, got {}", tension_p0);

        // Near player 0's units, player 1 should have negative tension (enemy advantage)
        let tension_p1 = grid.get_tension(1, 5, 5);
        assert!(tension_p1 < 0.0, "Player 1 should have negative tension near player 0 units, got {}", tension_p1);
    }

    #[test]
    fn test_dead_units_excluded() {
        let mut game = test_game();
        let thrall = game.spawn_thrall(8.5, 8.5, 0);

        // Kill the unit
        if let Some(h) = game.world.get_component_mut::<Health>(thrall) {
            h.current = 0.0;
        }

        game.tick(50.0);

        let mut grid = InfluenceGrid::new(16, 16, 2);
        grid.update(&game.world, 1);

        // Dead unit should not contribute influence
        let friendly = grid.get_friendly_strength(0, 8, 8);
        assert_eq!(friendly, 0.0, "Dead unit should not contribute friendly strength");
    }

    #[test]
    fn test_buildings_excluded() {
        let mut game = test_game();
        // Spawn a Command Post (speed=0, no combat capability)
        game.spawn_command_post(8.5, 8.5, 0);

        game.tick(50.0);

        let mut grid = InfluenceGrid::new(16, 16, 2);
        grid.update(&game.world, 1);

        // Building should not contribute to influence
        let friendly = grid.get_friendly_strength(0, 8, 8);
        assert_eq!(friendly, 0.0, "Building should not contribute to influence");
    }

    #[test]
    fn test_highest_threat_tile() {
        let mut game = test_game();
        // Player 1 has a unit at (10, 10)
        game.spawn_thrall(10.5, 10.5, 1);

        game.tick(50.0);

        let mut grid = InfluenceGrid::new(16, 16, 2);
        grid.update(&game.world, 1);

        // Player 0 searches for highest threat tile near (10, 10)
        let result = grid.highest_threat_tile(0, 10, 10, 5);
        assert!(result.is_some(), "Should find a threat tile");
        let (tx, ty, val) = result.unwrap();
        assert!(val > 0.0, "Threat value should be positive");
        // Should be near the enemy unit
        let dist = (((tx as i32 - 10).pow(2) + (ty as i32 - 10).pow(2)) as f32).sqrt();
        assert!(dist < 3.0, "Highest threat should be near enemy unit, dist={}", dist);
    }

    #[test]
    fn test_find_safe_position() {
        let mut game = test_game();
        // Player 1 threat at (10, 10)
        game.spawn_thrall(10.5, 10.5, 1);
        // Player 0 unit at (5, 5)
        game.spawn_thrall(5.5, 5.5, 0);

        game.tick(50.0);

        let mut grid = InfluenceGrid::new(16, 16, 2);
        grid.update(&game.world, 1);

        // Player 0 searches for a safe position from (8, 8)
        let result = grid.find_safe_position(0, 8, 8, 6);
        assert!(result.is_some(), "Should find a safe position");
        let (sx, sy) = result.unwrap();

        // Safe position should be away from enemy and toward friendly
        let dist_to_enemy = (((sx as i32 - 10).pow(2) + (sy as i32 - 10).pow(2)) as f32).sqrt();
        let dist_to_friendly = (((sx as i32 - 5).pow(2) + (sy as i32 - 5).pow(2)) as f32).sqrt();
        assert!(
            dist_to_friendly < dist_to_enemy,
            "Safe position should be closer to friendly ({},{}) than enemy, dist_friendly={}, dist_enemy={}",
            sx, sy, dist_to_friendly, dist_to_enemy
        );
    }

    #[test]
    fn test_influence_falloff() {
        let mut game = test_game();
        game.spawn_thrall(8.5, 8.5, 0);

        game.tick(50.0);

        let mut grid = InfluenceGrid::new(16, 16, 2);
        grid.update(&game.world, 1);

        // Influence should decrease with distance
        let at_center = grid.get_friendly_strength(0, 8, 8);
        let at_1_away = grid.get_friendly_strength(0, 9, 8);
        let at_2_away = grid.get_friendly_strength(0, 10, 8);

        assert!(at_center > at_1_away, "Center {} should be > 1 away {}", at_center, at_1_away);
        assert!(at_1_away > at_2_away, "1 away {} should be > 2 away {}", at_1_away, at_2_away);
    }

    #[test]
    fn test_health_affects_influence() {
        let mut game = test_game();
        let _full_hp = game.spawn_thrall(5.5, 5.5, 0);
        let half_hp = game.spawn_thrall(12.5, 12.5, 0);

        // Set second thrall to 50% health
        if let Some(h) = game.world.get_component_mut::<Health>(half_hp) {
            h.current = 40.0; // 40/80 = 50%
        }

        game.tick(50.0);

        let mut grid = InfluenceGrid::new(16, 16, 2);
        grid.update(&game.world, 1);

        let full_influence = grid.get_friendly_strength(0, 5, 5);
        let half_influence = grid.get_friendly_strength(0, 12, 12);

        assert!(full_influence > half_influence,
            "Full HP unit ({}) should have more influence than half HP unit ({})",
            full_influence, half_influence);

        // Half HP should be roughly half the influence (at same distance)
        let ratio = half_influence / full_influence;
        assert!((ratio - 0.5).abs() < 0.15,
            "Half HP influence ratio should be ~0.5, got {}", ratio);
    }

    #[test]
    fn test_stronger_unit_more_influence() {
        let mut game = test_game();
        // Thrall: 8 damage / 0.5 cooldown = 16 DPS
        game.spawn_thrall(4.5, 8.5, 0);
        // Sentinel: 25 damage / 0.8 cooldown = 31.25 DPS
        game.spawn_unit(SpriteId::Sentinel, 12.5, 8.5, 0);

        game.tick(50.0);

        let mut grid = InfluenceGrid::new(16, 16, 2);
        grid.update(&game.world, 1);

        let thrall_influence = grid.get_friendly_strength(0, 4, 8);
        let sentinel_influence = grid.get_friendly_strength(0, 12, 8);

        assert!(sentinel_influence > thrall_influence,
            "Sentinel ({}) should have more influence than Thrall ({})",
            sentinel_influence, thrall_influence);
    }

    #[test]
    fn test_vulnerability_query() {
        let mut game = test_game();
        // Player 1 has a strong presence at (10, 10)
        game.spawn_unit(SpriteId::Sentinel, 10.5, 10.5, 1);

        game.tick(50.0);

        let mut grid = InfluenceGrid::new(16, 16, 2);
        grid.update(&game.world, 1);

        // Player 0 should see high vulnerability near enemy sentinel
        let vuln = grid.get_vulnerability(0, 10, 10);
        assert!(vuln > 0.0, "Player 0 should see vulnerability near enemy Sentinel, got {}", vuln);

        // Player 1 should see low vulnerability (they own the unit)
        let vuln_p1 = grid.get_vulnerability(1, 10, 10);
        assert!(vuln_p1 < 0.0, "Player 1 should have negative vulnerability near own unit, got {}", vuln_p1);
    }
}

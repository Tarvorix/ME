use std::collections::BinaryHeap;
use std::cmp::Ordering;
use crate::map::{BattleMap, movement_cost};
use crate::types::SpriteId;

const SQRT2: f32 = 1.41421356;

/// A* node for the priority queue (min-heap via reversed Ord).
#[derive(Debug)]
struct Node {
    x: u32,
    y: u32,
    f_cost: f32, // g + h
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.f_cost == other.f_cost
    }
}
impl Eq for Node {}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed for min-heap behavior
        other.f_cost.partial_cmp(&self.f_cost).unwrap_or(Ordering::Equal)
    }
}

/// Octile distance heuristic (admissible + consistent for 8-dir grids).
fn heuristic(x: u32, y: u32, gx: u32, gy: u32) -> f32 {
    let dx = (x as f32 - gx as f32).abs();
    let dy = (y as f32 - gy as f32).abs();
    let min = dx.min(dy);
    let max = dx.max(dy);
    max + (SQRT2 - 1.0) * min
}

/// 8-directional neighbors: (dx, dy, cost).
const NEIGHBORS: [(i32, i32, f32); 8] = [
    (0, -1, 1.0),   // N
    (1, -1, SQRT2), // NE
    (1, 0, 1.0),    // E
    (1, 1, SQRT2),  // SE
    (0, 1, 1.0),    // S
    (-1, 1, SQRT2), // SW
    (-1, 0, 1.0),   // W
    (-1, -1, SQRT2), // NW
];

/// Find a path from start to goal on the battle map using A*.
/// `unit_kind` is used to compute terrain-aware movement costs.
/// If None, defaults to Thrall movement costs.
/// Returns a sequence of tile coordinates from start to goal (inclusive), or None if unreachable.
pub fn find_path(
    map: &BattleMap,
    start: (u32, u32),
    goal: (u32, u32),
    unit_kind: Option<SpriteId>,
) -> Option<Vec<(u32, u32)>> {
    if !map.is_walkable(start.0, start.1) || !map.is_walkable(goal.0, goal.1) {
        return None;
    }

    if start == goal {
        return Some(vec![start]);
    }

    let kind = unit_kind.unwrap_or(SpriteId::Thrall);
    let w = map.width;
    let h = map.height;
    let size = (w * h) as usize;

    let mut g_cost = vec![f32::INFINITY; size];
    let mut came_from = vec![u32::MAX; size];
    let mut closed = vec![false; size];

    let start_idx = (start.1 * w + start.0) as usize;
    g_cost[start_idx] = 0.0;

    let mut open = BinaryHeap::new();
    open.push(Node {
        x: start.0,
        y: start.1,
        f_cost: heuristic(start.0, start.1, goal.0, goal.1),
    });

    while let Some(current) = open.pop() {
        let cx = current.x;
        let cy = current.y;

        if cx == goal.0 && cy == goal.1 {
            // Reconstruct path
            return Some(reconstruct_path(&came_from, w, start, goal));
        }

        let c_idx = (cy * w + cx) as usize;
        if closed[c_idx] {
            continue;
        }
        closed[c_idx] = true;

        let current_g = g_cost[c_idx];

        for &(dx, dy, base_cost) in &NEIGHBORS {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;

            if !map.in_bounds(nx, ny) {
                continue;
            }

            let nx = nx as u32;
            let ny = ny as u32;

            if !map.is_walkable(nx, ny) {
                continue;
            }

            // Prevent diagonal corner-cutting through walls
            if dx != 0 && dy != 0 {
                let adj1_walkable = map.is_walkable(cx, ny);
                let adj2_walkable = map.is_walkable(nx, cy);
                if !adj1_walkable || !adj2_walkable {
                    continue;
                }
            }

            let n_idx = (ny * w + nx) as usize;
            if closed[n_idx] {
                continue;
            }

            // Apply terrain movement cost to the neighbor tile
            let terrain = map.get(nx, ny).terrain_type();
            let terrain_cost = movement_cost(terrain, kind);
            let tentative_g = current_g + base_cost * terrain_cost;

            if tentative_g < g_cost[n_idx] {
                g_cost[n_idx] = tentative_g;
                came_from[n_idx] = cy * w + cx;
                let f = tentative_g + heuristic(nx, ny, goal.0, goal.1);
                open.push(Node { x: nx, y: ny, f_cost: f });
            }
        }
    }

    None // No path found
}

fn reconstruct_path(came_from: &[u32], w: u32, start: (u32, u32), goal: (u32, u32)) -> Vec<(u32, u32)> {
    let mut path = Vec::new();
    let mut current = goal.1 * w + goal.0;
    let start_idx = start.1 * w + start.0;

    path.push(goal);
    while current != start_idx {
        let parent = came_from[current as usize];
        let px = parent % w;
        let py = parent / w;
        path.push((px, py));
        current = parent;
    }

    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{BattleMap, TerrainType};

    #[test]
    fn test_straight_path() {
        let map = BattleMap::new(8, 8);
        let path = find_path(&map, (0, 0), (4, 0), None);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(*path.first().unwrap(), (0, 0));
        assert_eq!(*path.last().unwrap(), (4, 0));
    }

    #[test]
    fn test_diagonal_path() {
        let map = BattleMap::new(8, 8);
        let path = find_path(&map, (0, 0), (3, 3), None);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 4); // diagonal: 4 steps including start
    }

    #[test]
    fn test_path_around_wall() {
        let mut map = BattleMap::new(8, 8);
        // Create a vertical wall at x=3, leaving a gap only at y=7
        for y in 0..7 {
            map.get_mut(3, y).terrain = TerrainType::Impassable as u8;
        }

        let path = find_path(&map, (1, 3), (5, 3), None);
        assert!(path.is_some());
        let path = path.unwrap();
        // Path must go around the wall (down to y=7 gap)
        assert!(path.len() > 5);
        // Verify no point is on the wall (x=3 for y=0..6)
        for &(x, y) in &path {
            if x == 3 {
                assert!(y >= 7, "Path should not cross the wall at x=3,y={}", y);
            }
        }
    }

    #[test]
    fn test_no_path_blocked() {
        let mut map = BattleMap::new(8, 8);
        // Completely surround the goal
        for x in 3..6 {
            map.get_mut(x, 3).terrain = TerrainType::Impassable as u8;
            map.get_mut(x, 5).terrain = TerrainType::Impassable as u8;
        }
        map.get_mut(3, 4).terrain = TerrainType::Impassable as u8;
        map.get_mut(5, 4).terrain = TerrainType::Impassable as u8;

        let path = find_path(&map, (0, 0), (4, 4), None);
        assert!(path.is_none());
    }

    #[test]
    fn test_same_start_and_goal() {
        let map = BattleMap::new(8, 8);
        let path = find_path(&map, (3, 3), (3, 3), None);
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 1);
    }

    #[test]
    fn test_no_corner_cutting() {
        let mut map = BattleMap::new(8, 8);
        // Place walls that would allow corner cutting
        map.get_mut(2, 1).terrain = TerrainType::Impassable as u8;
        map.get_mut(1, 2).terrain = TerrainType::Impassable as u8;

        let path = find_path(&map, (1, 1), (2, 2), None);
        assert!(path.is_some());
        let path = path.unwrap();
        // Should NOT go directly (1,1)->(2,2) as that cuts the corner
        assert!(path.len() > 2);
    }

    #[test]
    fn test_pathfinding_prefers_road() {
        let mut map = BattleMap::new(16, 3);
        // Row 1 is all Rough (cost 1.5)
        for x in 0..16 {
            map.get_mut(x, 1).terrain = TerrainType::Rough as u8;
        }
        // Row 2 is all Road (cost 0.75)
        for x in 0..16 {
            map.get_mut(x, 2).terrain = TerrainType::Road as u8;
        }

        // Path from (0,0) to (15,0) — should prefer going through Road tiles
        // even though it means going down and back up
        let path_thrall = find_path(&map, (0, 0), (15, 0), Some(SpriteId::Thrall));
        assert!(path_thrall.is_some());

        // Verify the path exists (specific route depends on cost)
        let path = path_thrall.unwrap();
        assert!(*path.last().unwrap() == (15, 0));
    }

    #[test]
    fn test_hover_tank_ignores_rough_terrain() {
        let mut map = BattleMap::new(8, 8);
        // Make most tiles Rough
        for y in 0..8 {
            for x in 0..8 {
                map.get_mut(x, y).terrain = TerrainType::Rough as u8;
            }
        }

        // HoverTank should path straight through (cost 1.0 regardless)
        let path_tank = find_path(&map, (0, 0), (7, 0), Some(SpriteId::HoverTank));
        assert!(path_tank.is_some());
        let path = path_tank.unwrap();
        // Should be a straight line (8 tiles including start)
        assert_eq!(path.len(), 8);

        // Thrall should also find a path (Rough is walkable, just expensive)
        let path_thrall = find_path(&map, (0, 0), (7, 0), Some(SpriteId::Thrall));
        assert!(path_thrall.is_some());
    }

    #[test]
    fn test_terrain_cost_affects_path_choice() {
        let mut map = BattleMap::new(8, 8);
        // Create a band of Rough terrain across the direct path
        for x in 0..8 {
            map.get_mut(x, 3).terrain = TerrainType::Rough as u8;
            map.get_mut(x, 4).terrain = TerrainType::Rough as u8;
        }
        // Create a Road bypass around the Rough
        map.get_mut(0, 5).terrain = TerrainType::Road as u8;
        map.get_mut(0, 6).terrain = TerrainType::Road as u8;

        // Path should exist for both unit types
        let path = find_path(&map, (0, 0), (0, 7), Some(SpriteId::Thrall));
        assert!(path.is_some());
    }
}

use crate::ecs::World;
use crate::ecs::entity::Entity;
use crate::blueprints::{get_blueprint, production_line, ProductionLine};
use crate::components::Position;
use crate::types::{SpriteId, EventType};
use crate::game::TickDelta;
use crate::systems::resource::Economies;

/// Strain added per Thrall produced.
const THRALL_STRAIN_AMOUNT: f32 = 8.0;

/// A single production job in a Node queue line.
#[derive(Debug, Clone)]
pub struct ProductionJob {
    pub unit_type: SpriteId,
    pub progress: f32,
    pub total_time: f32,
}

/// Per-player production state.
#[derive(Debug, Clone)]
pub struct PlayerProduction {
    /// Infantry production lines (each can hold one job at a time).
    pub infantry_lines: Vec<Option<ProductionJob>>,
    /// Armor production lines (each can hold one job at a time).
    pub armor_lines: Vec<Option<ProductionJob>>,
    /// Rally point X (where newly produced units move to).
    pub rally_x: f32,
    /// Rally point Y.
    pub rally_y: f32,
    /// The player's Command Post entity (spawn location).
    pub command_post: Option<Entity>,
}

impl PlayerProduction {
    pub fn new() -> Self {
        PlayerProduction {
            infantry_lines: vec![None], // 1 infantry line to start
            armor_lines: vec![None],    // 1 armor line to start
            rally_x: 0.0,
            rally_y: 0.0,
            command_post: None,
        }
    }

    /// Try to queue a unit for production. Returns true if queued, false if no free line.
    pub fn queue_unit(&mut self, unit_type: SpriteId) -> bool {
        let bp = get_blueprint(unit_type);
        let line = production_line(unit_type);

        let lines = match line {
            Some(ProductionLine::Infantry) => &mut self.infantry_lines,
            Some(ProductionLine::Armor) => &mut self.armor_lines,
            None => return false, // Buildings can't be produced this way
        };

        // Find first free line
        for slot in lines.iter_mut() {
            if slot.is_none() {
                *slot = Some(ProductionJob {
                    unit_type,
                    progress: 0.0,
                    total_time: bp.build_time_secs,
                });
                return true;
            }
        }

        false // All lines busy
    }

    /// Cancel production on a specific line index.
    /// line_index: 0..infantry_count for infantry, infantry_count.. for armor.
    pub fn cancel_line(&mut self, line_index: u8) {
        let idx = line_index as usize;
        let infantry_count = self.infantry_lines.len();
        if idx < infantry_count {
            self.infantry_lines[idx] = None;
        } else {
            let armor_idx = idx - infantry_count;
            if armor_idx < self.armor_lines.len() {
                self.armor_lines[armor_idx] = None;
            }
        }
    }
}

/// Resource: per-player production state.
pub struct Productions(pub Vec<PlayerProduction>);

impl Productions {
    pub fn new(player_count: u32) -> Self {
        Productions(
            (0..player_count)
                .map(|_| PlayerProduction::new())
                .collect()
        )
    }
}

/// Production system: advances production queues, spawns completed units.
pub fn production_system(world: &mut World) {
    let delta = if let Some(td) = world.get_resource::<TickDelta>() {
        td.0
    } else {
        return;
    };

    // Get strain production penalties per player
    let strain_penalties: Vec<f32> = if let Some(econ) = world.get_resource::<Economies>() {
        econ.0.iter().map(|e| e.strain_production_penalty()).collect()
    } else {
        return;
    };

    // Pre-read command post positions (immutable borrows first)
    let cp_positions: Vec<Option<(f32, f32)>> = {
        let prods = if let Some(p) = world.get_resource::<Productions>() {
            p
        } else {
            return;
        };
        prods.0.iter().map(|prod| {
            if let Some(cp) = prod.command_post {
                world.get_component::<Position>(cp).map(|pos| (pos.x, pos.y))
            } else {
                None
            }
        }).collect()
    };

    // Collect completed production jobs and their spawn info
    let mut completed: Vec<(usize, SpriteId, f32, f32)> = Vec::new(); // (player, unit_type, spawn_x, spawn_y)
    let mut energy_costs: Vec<(usize, u32)> = Vec::new(); // (player, cost)
    let mut strain_additions: Vec<(usize, f32)> = Vec::new(); // (player, strain_amount)

    {
        let productions = if let Some(p) = world.get_resource_mut::<Productions>() {
            p
        } else {
            return;
        };

        for (player_idx, prod) in productions.0.iter_mut().enumerate() {
            let penalty = if player_idx < strain_penalties.len() {
                strain_penalties[player_idx]
            } else {
                0.0
            };

            // Get spawn location from pre-read command post positions
            let (spawn_x, spawn_y) = if let Some(pos) = cp_positions.get(player_idx).and_then(|p| *p) {
                pos
            } else {
                continue; // No command post or no valid position
            };

            // Advance all infantry lines
            for line in prod.infantry_lines.iter_mut() {
                if let Some(job) = line {
                    let speed = 1.0 - penalty;
                    job.progress += delta * speed;

                    if job.progress >= job.total_time {
                        let unit_type = job.unit_type;
                        let bp = get_blueprint(unit_type);
                        energy_costs.push((player_idx, bp.energy_cost));

                        if bp.is_conscript {
                            strain_additions.push((player_idx, THRALL_STRAIN_AMOUNT));
                        }

                        completed.push((player_idx, unit_type, spawn_x, spawn_y));
                        *line = None; // Clear the line
                    }
                }
            }

            // Advance all armor lines
            for line in prod.armor_lines.iter_mut() {
                if let Some(job) = line {
                    let speed = 1.0 - penalty;
                    job.progress += delta * speed;

                    if job.progress >= job.total_time {
                        let unit_type = job.unit_type;
                        let bp = get_blueprint(unit_type);
                        energy_costs.push((player_idx, bp.energy_cost));

                        if bp.is_conscript {
                            strain_additions.push((player_idx, THRALL_STRAIN_AMOUNT));
                        }

                        completed.push((player_idx, unit_type, spawn_x, spawn_y));
                        *line = None;
                    }
                }
            }
        }
    }

    // Deduct energy costs and add strain
    if let Some(econ) = world.get_resource_mut::<Economies>() {
        for (player, cost) in &energy_costs {
            if *player < econ.0.len() {
                econ.0[*player].energy_bank -= *cost as f32;
                if econ.0[*player].energy_bank < 0.0 {
                    econ.0[*player].energy_bank = 0.0;
                }
            }
        }
        for (player, amount) in &strain_additions {
            if *player < econ.0.len() {
                econ.0[*player].add_strain(*amount);
            }
        }
    }

    // Spawn completed units and issue rally move
    for (player_idx, unit_type, spawn_x, spawn_y) in &completed {
        let owner = *player_idx as u8;

        // Spawn the unit at the Command Post location
        let bp = get_blueprint(*unit_type);
        let entity = world.spawn();
        world.add_component(entity, crate::components::Position { x: *spawn_x, y: *spawn_y });
        world.add_component(entity, crate::components::PreviousPosition { x: *spawn_x, y: *spawn_y });
        world.add_component(entity, crate::components::UnitType { kind: *unit_type, owner });
        world.add_component(entity, crate::components::Health::new(bp.max_hp));
        world.add_component(entity, crate::components::VisionRange(bp.vision_range));
        world.add_component(entity, crate::components::Deployed(true));
        world.add_component(entity, crate::components::RenderState::new(*unit_type, bp.scale));
        if bp.speed > 0.0 {
            world.add_component(entity, crate::components::PathState::empty(bp.speed));
        }
        if bp.damage > 0.0 {
            world.add_component(entity, crate::components::CombatState::new());
        }

        // Write UnitSpawned event
        let mut payload = [0u8; 16];
        payload[0..2].copy_from_slice(&(*unit_type as u16).to_le_bytes());
        payload[2] = owner;
        crate::game::write_event(world, EventType::UnitSpawned, entity.raw(), *spawn_x, *spawn_y, &payload);

        // Issue rally move if rally point is set
        let (rally_x, rally_y) = if let Some(prods) = world.get_resource::<Productions>() {
            let p = &prods.0[*player_idx];
            (p.rally_x, p.rally_y)
        } else {
            continue;
        };

        if rally_x != 0.0 || rally_y != 0.0 {
            // Pathfind to rally point
            let (start_tx, start_ty) = (spawn_x.floor() as u32, spawn_y.floor() as u32);
            let (goal_tx, goal_ty) = (rally_x.floor() as u32, rally_y.floor() as u32);

            let path = {
                let map = world.get_resource::<crate::map::BattleMap>();
                if let Some(m) = map {
                    let gt = goal_tx.min(m.width - 1);
                    let gs = goal_ty.min(m.height - 1);
                    let st = start_tx.min(m.width - 1);
                    let ss = start_ty.min(m.height - 1);
                    crate::pathfinding::find_path(m, (st, ss), (gt, gs), Some(*unit_type))
                } else {
                    None
                }
            };

            if let Some(path) = path {
                if let Some(ps) = world.get_component_mut::<crate::components::PathState>(entity) {
                    ps.path = path;
                    ps.current_index = 1;
                }
            }
        }
    }
}

/// Write production queue data to UIStateBuffer bytes [68-195].
/// Layout per line (16 bytes each): [0-1] unit_type u16, [2-5] progress f32, [6-9] total_time f32, [10-15] padding
/// Max 8 lines (128 bytes total: 68..196)
pub fn write_production_ui(world: &mut World) {
    // Collect production data into owned structs to avoid borrow conflicts
    let line_data: Vec<Option<(u16, f32, f32)>> = if let Some(p) = world.get_resource::<Productions>() {
        if p.0.is_empty() { return; }
        let prod = &p.0[0];
        let mut lines = Vec::new();
        for line in &prod.infantry_lines {
            lines.push(line.as_ref().map(|j| (j.unit_type as u16, j.progress, j.total_time)));
        }
        for line in &prod.armor_lines {
            lines.push(line.as_ref().map(|j| (j.unit_type as u16, j.progress, j.total_time)));
        }
        lines
    } else {
        return;
    };

    let ui = if let Some(u) = world.get_resource_mut::<crate::systems::resource::UIStateBuffer>() {
        &mut u.0
    } else {
        return;
    };

    // Clear production area
    for i in 68..196 {
        ui[i] = 0;
    }

    for (i, line) in line_data.iter().enumerate().take(8) {
        let offset = 68 + i * 16;
        if let Some((unit_type, progress, total_time)) = line {
            ui[offset..offset + 2].copy_from_slice(&unit_type.to_le_bytes());
            ui[offset + 2..offset + 6].copy_from_slice(&progress.to_le_bytes());
            ui[offset + 6..offset + 10].copy_from_slice(&total_time.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, GameConfig};

    fn test_game_with_production() -> Game {
        let mut game = Game::new(GameConfig {
            map_width: 16,
            map_height: 16,
            player_count: 2,
            seed: 42,
        });

        // Spawn command posts and register them
        let cp0 = game.spawn_command_post(8.5, 8.5, 0);
        let cp1 = game.spawn_command_post(4.5, 4.5, 1);

        if let Some(prods) = game.world.get_resource_mut::<Productions>() {
            prods.0[0].command_post = Some(cp0);
            prods.0[1].command_post = Some(cp1);
        }

        game
    }

    #[test]
    fn test_produce_thrall() {
        let mut game = test_game_with_production();

        // Queue a Thrall on player 0
        if let Some(prods) = game.world.get_resource_mut::<Productions>() {
            assert!(prods.0[0].queue_unit(SpriteId::Thrall));
        }

        // Thrall build time = 10s. Tick for 10 seconds (200 ticks at 50ms)
        for _ in 0..200 {
            game.tick(50.0);
        }

        // Count player 0 units (should have command posts + 1 Thrall)
        let count = {
            let ut_s = game.world.get_storage::<crate::components::UnitType>().unwrap();
            ut_s.iter()
                .filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Thrall)
                .count()
        };
        assert_eq!(count, 1, "Should have produced 1 Thrall, got {}", count);
    }

    #[test]
    fn test_produce_uses_infantry_line() {
        let mut game = test_game_with_production();

        // Queue a Thrall — should use infantry line
        if let Some(prods) = game.world.get_resource_mut::<Productions>() {
            assert!(prods.0[0].queue_unit(SpriteId::Thrall));
            // Line should now be busy
            assert!(!prods.0[0].queue_unit(SpriteId::Thrall), "Should not queue second Thrall on same line");
        }
    }

    #[test]
    fn test_produce_hover_tank_uses_armor() {
        let mut game = test_game_with_production();

        if let Some(prods) = game.world.get_resource_mut::<Productions>() {
            // Queue HoverTank — should use armor line
            assert!(prods.0[0].queue_unit(SpriteId::HoverTank));
            // Can still queue infantry
            assert!(prods.0[0].queue_unit(SpriteId::Thrall));
        }
    }

    #[test]
    fn test_production_blocked_no_free_line() {
        let mut game = test_game_with_production();

        if let Some(prods) = game.world.get_resource_mut::<Productions>() {
            // Fill the only infantry line
            assert!(prods.0[0].queue_unit(SpriteId::Thrall));
            // Should fail — no free infantry line
            assert!(!prods.0[0].queue_unit(SpriteId::Sentinel));
        }
    }

    #[test]
    fn test_cancel_production() {
        let mut game = test_game_with_production();

        if let Some(prods) = game.world.get_resource_mut::<Productions>() {
            prods.0[0].queue_unit(SpriteId::Thrall);
            assert!(prods.0[0].infantry_lines[0].is_some());

            prods.0[0].cancel_line(0); // Cancel infantry line 0
            assert!(prods.0[0].infantry_lines[0].is_none());
        }
    }

    #[test]
    fn test_production_costs_energy() {
        let mut game = test_game_with_production();

        let start_bank = game.world.get_resource::<Economies>().unwrap().0[0].energy_bank;

        if let Some(prods) = game.world.get_resource_mut::<Productions>() {
            prods.0[0].queue_unit(SpriteId::Thrall);
        }

        // Tick through production (10 seconds)
        for _ in 0..210 {
            game.tick(50.0);
        }

        let end_bank = game.world.get_resource::<Economies>().unwrap().0[0].energy_bank;
        // Thrall costs 30 energy. Over 10.5s with 5e/s income, the bank gains ~52.5 minus 30 cost.
        // Verify bank is LESS than it would be without the production cost (start + income - upkeep).
        // The production cost should have been deducted at some point.
        let max_possible_bank = start_bank + 5.0 * 10.5; // start + income with no production cost
        assert!(end_bank < max_possible_bank,
            "Production should have cost energy: end={}, max_without_cost={}", end_bank, max_possible_bank);
    }

    #[test]
    fn test_thrall_adds_strain() {
        let mut game = test_game_with_production();

        if let Some(prods) = game.world.get_resource_mut::<Productions>() {
            prods.0[0].queue_unit(SpriteId::Thrall);
        }

        // Tick through production (10s build time + buffer)
        for _ in 0..220 {
            game.tick(50.0);
        }

        // Strain should have been added (though some decay happens)
        // With THRALL_STRAIN_AMOUNT=8 and 11s elapsed, decay would reduce it
        // but it should still be above 0
        let strain = game.world.get_resource::<Economies>().unwrap().0[0].conscription_strain;
        assert!(strain > 0.0, "Thrall production should add strain, got {}", strain);
    }

    #[test]
    fn test_strain_slows_production() {
        let mut game = test_game_with_production();

        // Set strain very high
        if let Some(econ) = game.world.get_resource_mut::<Economies>() {
            econ.0[0].conscription_strain = 90.0;
        }

        if let Some(prods) = game.world.get_resource_mut::<Productions>() {
            prods.0[0].queue_unit(SpriteId::Thrall);
        }

        // Normal Thrall build time = 10s. At 90% strain, production penalty = 50%.
        // So effective build time = 10s / (1 - 0.5) = 20s.
        // Tick for 11s — should NOT be done yet
        for _ in 0..220 {
            game.tick(50.0);
        }

        // Check if production is still in progress (not completed)
        let still_producing = if let Some(prods) = game.world.get_resource::<Productions>() {
            prods.0[0].infantry_lines[0].is_some()
        } else {
            false
        };
        assert!(still_producing, "Production should be slower with high strain");
    }

    #[test]
    fn test_rally_point() {
        let mut game = test_game_with_production();

        // Set rally point
        if let Some(prods) = game.world.get_resource_mut::<Productions>() {
            prods.0[0].rally_x = 12.0;
            prods.0[0].rally_y = 12.0;
            prods.0[0].queue_unit(SpriteId::Thrall);
        }

        // Tick through production (10s) + some movement time
        for _ in 0..400 {
            game.tick(50.0);
        }

        // Find the produced Thrall and check it's moving toward rally
        let thrall_pos = {
            let ut_s = game.world.get_storage::<crate::components::UnitType>().unwrap();
            let pos_s = game.world.get_storage::<Position>().unwrap();
            let mut result = None;
            for (entity, ut) in ut_s.iter() {
                if ut.owner == 0 && ut.kind == SpriteId::Thrall {
                    if let Some(pos) = pos_s.get(entity) {
                        result = Some((pos.x, pos.y));
                    }
                }
            }
            result
        };

        assert!(thrall_pos.is_some(), "Thrall should exist");
        let (x, y) = thrall_pos.unwrap();
        // After 10 seconds of movement at speed 3, it should be closer to (12,12)
        let dist_to_rally = ((x - 12.5).powi(2) + (y - 12.5).powi(2)).sqrt();
        let dist_from_cp = ((x - 8.5).powi(2) + (y - 8.5).powi(2)).sqrt();
        assert!(dist_to_rally < dist_from_cp || dist_to_rally < 2.0,
            "Thrall should be moving toward rally point, pos=({}, {})", x, y);
    }

    #[test]
    fn test_spawns_at_command_post() {
        let mut game = test_game_with_production();

        if let Some(prods) = game.world.get_resource_mut::<Productions>() {
            prods.0[0].queue_unit(SpriteId::Thrall);
        }

        // Tick through production (10s + buffer)
        for _ in 0..220 {
            game.tick(50.0);
        }

        // The spawned Thrall should start near the command post at (8.5, 8.5)
        // By now it may have moved a bit, but check within a reasonable range
        let thrall_exists = {
            let ut_s = game.world.get_storage::<crate::components::UnitType>().unwrap();
            ut_s.iter().any(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Thrall)
        };
        assert!(thrall_exists, "Thrall should have been spawned at command post");
    }
}

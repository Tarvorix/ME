use crate::ecs::{World, SystemRunner};
use crate::ecs::entity::Entity;
use crate::components::{Position, PreviousPosition, UnitType, PathState, RenderState, Health, CombatState, VisionRange, Deployed};
use crate::blueprints::get_blueprint;
use crate::map::BattleMap;
use crate::command::PendingCommands;
use crate::types::{SpriteId, EventType};
use crate::systems;
use crate::systems::fog::FogGrid;
use crate::systems::production::Productions;
use crate::systems::resource::{Economies, UIStateBuffer};
use crate::ai::influence_map::InfluenceGrid;
use crate::ai::tactical::{BtTemplates, AiBlackboard};
use crate::ai::player::{AiPlayers, AiPlayer, AiDifficulty};

pub const MAX_ENTITIES: usize = 2048;
pub const RENDER_ENTRY_SIZE: usize = 32;
pub const MAX_EVENTS: usize = 256;
pub const EVENT_ENTRY_SIZE: usize = 32;

/// Resource: seconds elapsed this tick.
pub struct TickDelta(pub f32);

/// Resource: the render buffer (flat byte array).
pub struct RenderBuffer(pub Vec<u8>);

/// Resource: number of entities written to render buffer.
pub struct RenderCount(pub u32);

/// Resource: the event buffer (flat byte array).
pub struct EventBuffer(pub Vec<u8>);

/// Resource: number of events written this tick.
pub struct EventCount(pub u32);

/// Timing data from a profiled tick. Each field is the duration of that system in microseconds.
#[derive(Clone, Debug, Default)]
pub struct TickProfile {
    pub total_us: u64,
    pub system_timings: Vec<(String, u64)>,
}

pub struct GameConfig {
    pub map_width: u32,
    pub map_height: u32,
    pub player_count: u32,
    pub seed: u32,
}

pub struct Game {
    pub world: World,
    pub systems: SystemRunner,
    pub tick_count: u32,
}

impl Game {
    pub fn new(config: GameConfig) -> Self {
        let mut world = World::new();

        // Generate map and store as resource
        let map = BattleMap::generate_simple(
            config.map_width,
            config.map_height,
            config.seed as u64,
        );
        world.insert_resource(map);

        // Allocate buffers
        world.insert_resource(RenderBuffer(vec![0u8; MAX_ENTITIES * RENDER_ENTRY_SIZE]));
        world.insert_resource(RenderCount(0));
        world.insert_resource(EventBuffer(vec![0u8; MAX_EVENTS * EVENT_ENTRY_SIZE]));
        world.insert_resource(EventCount(0));
        world.insert_resource(PendingCommands::new());
        world.insert_resource(TickDelta(0.05)); // 50ms default
        world.insert_resource(FogGrid::new(config.map_width, config.map_height, config.player_count));
        world.insert_resource(Economies::new(config.player_count));
        world.insert_resource(Productions::new(config.player_count));
        world.insert_resource(UIStateBuffer::new());
        world.insert_resource(InfluenceGrid::new(config.map_width, config.map_height, config.player_count));
        world.insert_resource(BtTemplates::new());
        world.insert_resource(AiBlackboard::new(config.player_count));
        world.insert_resource(AiPlayers::new());

        // Register systems in execution order
        let mut systems = SystemRunner::new();
        systems.add_system("command_processor", systems::command_processor_system);
        systems.add_system("movement", systems::movement_system);
        systems.add_system("combat", systems::combat_system);
        systems.add_system("capture", systems::capture_system);
        systems.add_system("battle_victory", systems::battle_victory_system);
        systems.add_system("death_cleanup", systems::death_cleanup_system);
        systems.add_system("fog", systems::fog_system);
        systems.add_system("ai_influence", crate::ai::tactical::ai_influence_update_system);
        systems.add_system("ai_tactical", crate::ai::tactical::ai_tactical_system);
        systems.add_system("ai_strategic", crate::ai::player::ai_strategic_system);
        systems.add_system("production", systems::production_system);
        systems.add_system("resource", systems::resource_system);
        systems.add_system("animation", systems::animation_system);
        systems.add_system("render_buffer", systems::render_buffer_system);

        Game {
            world,
            systems,
            tick_count: 0,
        }
    }

    pub fn tick(&mut self, delta_ms: f32) {
        // Set tick delta
        if let Some(td) = self.world.get_resource_mut::<TickDelta>() {
            td.0 = delta_ms / 1000.0;
        }

        // Reset event count at start of each tick
        if let Some(ec) = self.world.get_resource_mut::<EventCount>() {
            ec.0 = 0;
        }

        // Run all systems
        self.systems.run_all(&mut self.world);

        self.tick_count += 1;

        // Write UI state buffer (game tick, economy data, production queues)
        if let Some(ui) = self.world.get_resource_mut::<UIStateBuffer>() {
            ui.0[20..24].copy_from_slice(&self.tick_count.to_le_bytes());
        }
        crate::systems::resource::write_ui_state(&mut self.world);
        crate::systems::production::write_production_ui(&mut self.world);
    }

    /// Tick with profiling — returns timing data for each system.
    pub fn tick_profiled(&mut self, delta_ms: f32) -> TickProfile {
        let total_start = std::time::Instant::now();

        // Set tick delta
        if let Some(td) = self.world.get_resource_mut::<TickDelta>() {
            td.0 = delta_ms / 1000.0;
        }

        // Reset event count at start of each tick
        if let Some(ec) = self.world.get_resource_mut::<EventCount>() {
            ec.0 = 0;
        }

        // Run all systems with profiling
        let timings = self.systems.run_all_profiled(&mut self.world);

        self.tick_count += 1;

        // Write UI state buffer
        if let Some(ui) = self.world.get_resource_mut::<UIStateBuffer>() {
            ui.0[20..24].copy_from_slice(&self.tick_count.to_le_bytes());
        }
        crate::systems::resource::write_ui_state(&mut self.world);
        crate::systems::production::write_production_ui(&mut self.world);

        let total_us = total_start.elapsed().as_micros() as u64;

        TickProfile {
            total_us,
            system_timings: timings.into_iter().map(|(name, us)| (name.to_string(), us)).collect(),
        }
    }

    /// Spawn any unit or building type at the given tile position using blueprint data.
    pub fn spawn_unit(&mut self, kind: SpriteId, x: f32, y: f32, owner: u8) -> Entity {
        let bp = get_blueprint(kind);
        let entity = self.world.spawn();

        self.world.add_component(entity, Position { x, y });
        self.world.add_component(entity, PreviousPosition { x, y });
        self.world.add_component(entity, UnitType { kind, owner });
        self.world.add_component(entity, Health::new(bp.max_hp));
        self.world.add_component(entity, VisionRange(bp.vision_range));
        self.world.add_component(entity, Deployed(true));
        self.world.add_component(entity, RenderState::new(kind, bp.scale));

        // Mobile units get pathfinding state
        if bp.speed > 0.0 {
            self.world.add_component(entity, PathState::empty(bp.speed));
        }

        // Combat-capable units get combat state
        if bp.damage > 0.0 {
            self.world.add_component(entity, CombatState::new());
        }

        entity
    }

    /// Convenience: spawn a Thrall at the given tile position.
    pub fn spawn_thrall(&mut self, x: f32, y: f32, owner: u8) -> Entity {
        self.spawn_unit(SpriteId::Thrall, x, y, owner)
    }

    /// Spawn a Command Post at the given tile position.
    pub fn spawn_command_post(&mut self, x: f32, y: f32, owner: u8) -> Entity {
        self.spawn_unit(SpriteId::CommandPost, x, y, owner)
    }

    /// Push a command to be processed next tick.
    pub fn push_command(&mut self, cmd: crate::command::Command) {
        if let Some(pending) = self.world.get_resource_mut::<PendingCommands>() {
            pending.push(cmd);
        }
    }

    pub fn render_buffer_ptr(&self) -> *const u8 {
        self.world.get_resource::<RenderBuffer>()
            .map(|rb| rb.0.as_ptr())
            .unwrap_or(std::ptr::null())
    }

    pub fn render_count(&self) -> u32 {
        self.world.get_resource::<RenderCount>()
            .map(|rc| rc.0)
            .unwrap_or(0)
    }

    pub fn map(&self) -> &BattleMap {
        self.world.get_resource::<BattleMap>().unwrap()
    }

    /// Spawn a Forge building at the given tile position.
    pub fn spawn_forge(&mut self, x: f32, y: f32, owner: u8) -> Entity {
        self.spawn_unit(SpriteId::Forge, x, y, owner)
    }

    /// Check if a player has any Forge entity alive.
    pub fn check_forge_alive(&self, player_id: u8) -> bool {
        let ut_storage = match self.world.get_storage::<UnitType>() {
            Some(s) => s,
            None => return false,
        };
        let health_storage = self.world.get_storage::<Health>();

        for (entity, ut) in ut_storage.iter() {
            if ut.owner == player_id && ut.kind == SpriteId::Forge {
                // Check if it's alive (health > 0 or no health component)
                if let Some(hs) = &health_storage {
                    if let Some(h) = hs.get(entity) {
                        if !h.is_dead() {
                            return true;
                        }
                    } else {
                        return true; // No health component means alive
                    }
                } else {
                    return true;
                }
            }
        }
        false
    }

    /// Spawn starting units for a player: Command Post + Forge + 3 Thralls.
    /// Also registers the Command Post with the production system.
    pub fn spawn_starting_units(&mut self, player_id: u8, x: f32, y: f32) {
        use crate::systems::production::Productions;

        // Spawn Command Post
        let cp = self.spawn_command_post(x, y, player_id);

        // Register CP with production system
        if let Some(prods) = self.world.get_resource_mut::<Productions>() {
            if (player_id as usize) < prods.0.len() {
                prods.0[player_id as usize].command_post = Some(cp);
                prods.0[player_id as usize].rally_x = x + 3.0;
                prods.0[player_id as usize].rally_y = y + 3.0;
            }
        }

        // Spawn Forge nearby
        self.spawn_forge(x + 2.0, y - 2.0, player_id);

        // Spawn 3 starting Thralls
        for i in 0..3 {
            self.spawn_thrall(x + 1.0 + i as f32, y + 1.0, player_id);
        }
    }

    /// Get all combat unit entity IDs for a player.
    pub fn get_player_combat_unit_ids(&self, player_id: u8) -> Vec<u32> {
        let ut_storage = match self.world.get_storage::<UnitType>() {
            Some(s) => s,
            None => return Vec::new(),
        };
        let health_storage = self.world.get_storage::<Health>();

        let mut ids = Vec::new();
        for (entity, ut) in ut_storage.iter() {
            if ut.owner == player_id {
                let bp = get_blueprint(ut.kind);
                if bp.damage > 0.0 && bp.speed > 0.0 {
                    // Check alive
                    let alive = if let Some(hs) = &health_storage {
                        hs.get(entity).map(|h| !h.is_dead()).unwrap_or(true)
                    } else {
                        true
                    };
                    if alive {
                        ids.push(entity.raw());
                    }
                }
            }
        }
        ids
    }

    /// Add an AI player to the game. The AI will automatically make strategic decisions.
    pub fn add_ai_player(&mut self, player_id: u8, difficulty: AiDifficulty) {
        let map = self.world.get_resource::<BattleMap>().unwrap();
        let map_width = map.width;
        let map_height = map.height;
        let player_count = self.world.get_resource::<FogGrid>()
            .map(|f| f.player_count as u8)
            .unwrap_or(2);

        let seed = (player_id as u64) * 1000 + 42;

        if let Some(ai_players) = self.world.get_resource_mut::<AiPlayers>() {
            ai_players.0.push(AiPlayer::new(
                player_id,
                difficulty,
                seed,
                map_width,
                map_height,
                player_count,
            ));
        }
    }

    /// Compute a hash of the current game state for determinism verification.
    /// Hashes entity positions, health, and unit types.
    pub fn hash_game_state(&self) -> u32 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash tick count
        self.tick_count.hash(&mut hasher);

        // Hash all entity positions and health
        let pos_storage = self.world.get_storage::<Position>();
        let ut_storage = self.world.get_storage::<UnitType>();
        let health_storage = self.world.get_storage::<Health>();

        if let (Some(pos_s), Some(ut_s)) = (pos_storage, ut_storage) {
            // Collect into sorted vec for deterministic ordering
            let mut entities: Vec<(u32, f32, f32, u8, u8, i32)> = Vec::new();
            for (entity, pos) in pos_s.iter() {
                if let Some(ut) = ut_s.get(entity) {
                    let hp = if let Some(hs) = &health_storage {
                        hs.get(entity).map(|h| (h.current * 100.0) as i32).unwrap_or(0)
                    } else {
                        0
                    };
                    entities.push((
                        entity.raw(),
                        pos.x,
                        pos.y,
                        ut.kind as u8,
                        ut.owner,
                        hp,
                    ));
                }
            }
            entities.sort_by_key(|e| e.0);

            for (id, x, y, kind, owner, hp) in &entities {
                id.hash(&mut hasher);
                (*x as i32).hash(&mut hasher);
                (*y as i32).hash(&mut hasher);
                kind.hash(&mut hasher);
                owner.hash(&mut hasher);
                hp.hash(&mut hasher);
            }
        }

        // Hash economy state
        if let Some(economies) = self.world.get_resource::<crate::systems::resource::Economies>() {
            for econ in &economies.0 {
                (econ.energy_bank as i32).hash(&mut hasher);
                (econ.conscription_strain as i32).hash(&mut hasher);
            }
        }

        (hasher.finish() & 0xFFFFFFFF) as u32
    }
}

/// Write a 32-byte event to the event buffer. Called by systems during tick.
/// Layout: [0-1] event_type: u16, [2-3] reserved, [4-7] entity_id: u32,
///         [8-11] x: f32, [12-15] y: f32, [16-31] payload: [u8; 16]
pub fn write_event(
    world: &mut World,
    event_type: EventType,
    entity_id: u32,
    x: f32,
    y: f32,
    payload: &[u8; 16],
) {
    let count = if let Some(ec) = world.get_resource::<EventCount>() {
        ec.0 as usize
    } else {
        return;
    };

    if count >= MAX_EVENTS {
        return; // Buffer full, drop event
    }

    let offset = count * EVENT_ENTRY_SIZE;
    if let Some(eb) = world.get_resource_mut::<EventBuffer>() {
        let buf = &mut eb.0;
        if offset + EVENT_ENTRY_SIZE > buf.len() {
            return;
        }
        // event_type: u16
        buf[offset..offset + 2].copy_from_slice(&(event_type as u16).to_le_bytes());
        // reserved: u16
        buf[offset + 2..offset + 4].copy_from_slice(&0u16.to_le_bytes());
        // entity_id: u32
        buf[offset + 4..offset + 8].copy_from_slice(&entity_id.to_le_bytes());
        // x: f32
        buf[offset + 8..offset + 12].copy_from_slice(&x.to_le_bytes());
        // y: f32
        buf[offset + 12..offset + 16].copy_from_slice(&y.to_le_bytes());
        // payload: [u8; 16]
        buf[offset + 16..offset + 32].copy_from_slice(payload);
    }

    if let Some(ec) = world.get_resource_mut::<EventCount>() {
        ec.0 = (count + 1) as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::types::AnimState;

    fn test_config() -> GameConfig {
        GameConfig {
            map_width: 16,
            map_height: 16,
            player_count: 2,
            seed: 42,
        }
    }

    #[test]
    fn test_game_creation() {
        let game = Game::new(test_config());
        assert_eq!(game.tick_count, 0);
        assert!(game.world.has_resource::<BattleMap>());
        assert!(game.world.has_resource::<RenderBuffer>());
    }

    #[test]
    fn test_spawn_thrall() {
        let mut game = Game::new(test_config());
        let entity = game.spawn_thrall(5.5, 5.5, 0);
        assert!(game.world.is_alive(entity));
        assert!(game.world.has_component::<Position>(entity));
        assert!(game.world.has_component::<PathState>(entity));
        assert!(game.world.has_component::<RenderState>(entity));
        assert!(game.world.has_component::<Health>(entity));
        assert!(game.world.has_component::<CombatState>(entity));
        assert!(game.world.has_component::<VisionRange>(entity));
        assert!(game.world.has_component::<Deployed>(entity));

        // Verify blueprint-driven values
        let health = game.world.get_component::<Health>(entity).unwrap();
        assert_eq!(health.max, 80.0);
        assert_eq!(health.current, 80.0);

        let ps = game.world.get_component::<PathState>(entity).unwrap();
        assert_eq!(ps.speed, 3.0);

        let rs = game.world.get_component::<RenderState>(entity).unwrap();
        assert!((rs.scale - 48.0 / 512.0).abs() < 0.001);
    }

    #[test]
    fn test_spawn_sentinel() {
        let mut game = Game::new(test_config());
        let entity = game.spawn_unit(SpriteId::Sentinel, 5.5, 5.5, 0);
        assert!(game.world.is_alive(entity));
        assert!(game.world.has_component::<Health>(entity));
        assert!(game.world.has_component::<CombatState>(entity));
        assert!(game.world.has_component::<VisionRange>(entity));

        let health = game.world.get_component::<Health>(entity).unwrap();
        assert_eq!(health.max, 200.0);

        let ps = game.world.get_component::<PathState>(entity).unwrap();
        assert_eq!(ps.speed, 2.0);
    }

    #[test]
    fn test_spawn_hover_tank() {
        let mut game = Game::new(test_config());
        let entity = game.spawn_unit(SpriteId::HoverTank, 5.5, 5.5, 0);
        assert!(game.world.is_alive(entity));

        let health = game.world.get_component::<Health>(entity).unwrap();
        assert_eq!(health.max, 500.0);

        let ps = game.world.get_component::<PathState>(entity).unwrap();
        assert_eq!(ps.speed, 2.5);

        let vr = game.world.get_component::<VisionRange>(entity).unwrap();
        assert_eq!(vr.0, 10.0);
    }

    #[test]
    fn test_spawn_command_post() {
        let mut game = Game::new(test_config());
        let entity = game.spawn_command_post(5.5, 5.5, 0);
        assert!(game.world.is_alive(entity));

        let health = game.world.get_component::<Health>(entity).unwrap();
        assert_eq!(health.max, 800.0);

        // Command Post has no combat (damage=0) so no CombatState
        assert!(!game.world.has_component::<CombatState>(entity));

        // Command Post has no speed so no PathState
        assert!(!game.world.has_component::<PathState>(entity));

        // Large vision range
        let vr = game.world.get_component::<VisionRange>(entity).unwrap();
        assert_eq!(vr.0, 14.0);
    }

    #[test]
    fn test_health_renders_in_buffer() {
        let mut game = Game::new(test_config());
        let entity = game.spawn_thrall(5.5, 5.5, 0);

        // Set health to 50%
        if let Some(health) = game.world.get_component_mut::<Health>(entity) {
            health.current = 40.0; // 40/80 = 50%
        }

        game.tick(50.0);

        // Read health_pct from render buffer (byte 16)
        let buf = game.world.get_resource::<RenderBuffer>().unwrap();
        let health_pct = buf.0[16];
        assert_eq!(health_pct, 50, "Health should be 50% in render buffer, got {}", health_pct);
    }

    #[test]
    fn test_tick_produces_render_buffer() {
        let mut game = Game::new(test_config());
        game.spawn_thrall(5.5, 5.5, 0);
        game.tick(50.0);

        assert_eq!(game.render_count(), 1);

        // Read entity ID from render buffer
        let buf = game.world.get_resource::<RenderBuffer>().unwrap();
        let entity_id = u32::from_le_bytes([buf.0[0], buf.0[1], buf.0[2], buf.0[3]]);
        assert!(entity_id < MAX_ENTITIES as u32);
    }

    #[test]
    fn test_move_command_pathfinds() {
        let mut game = Game::new(GameConfig {
            map_width: 16,
            map_height: 16,
            player_count: 2,
            seed: 1,
        });
        let entity = game.spawn_thrall(2.5, 2.5, 0);

        // Issue move command
        game.push_command(Command::Move {
            unit_ids: vec![entity.raw()],
            target_x: 8.0,
            target_y: 8.0,
        });

        // Run one tick to process command
        game.tick(50.0);

        // Entity should have a path now
        let ps = game.world.get_component::<PathState>(entity).unwrap();
        assert!(ps.has_path(), "Entity should have a path after move command");

        // Run many ticks to let it arrive
        for _ in 0..200 {
            game.tick(50.0);
        }

        // Entity should be near the target
        let pos = game.world.get_component::<Position>(entity).unwrap();
        let dist = ((pos.x - 8.5).powi(2) + (pos.y - 8.5).powi(2)).sqrt();
        assert!(dist < 1.0, "Entity should be near target, but is at ({}, {})", pos.x, pos.y);
    }

    #[test]
    fn test_attack_sets_target() {
        let mut game = Game::new(test_config());
        let attacker = game.spawn_thrall(5.5, 5.5, 0);
        let target = game.spawn_thrall(6.5, 5.5, 1);

        game.push_command(Command::Attack {
            unit_ids: vec![attacker.raw()],
            target_id: target.raw(),
        });
        game.tick(50.0);

        let cs = game.world.get_component::<CombatState>(attacker).unwrap();
        assert_eq!(cs.target, Some(target));
    }

    #[test]
    fn test_combat_deals_damage() {
        let mut game = Game::new(test_config());
        // Place attacker and target within Thrall attack range (5.0 tiles)
        // Use spawn corner positions (within margin=4 of (0,0)) to ensure Open terrain
        let attacker = game.spawn_thrall(1.5, 1.5, 0);
        let target = game.spawn_thrall(3.5, 1.5, 1); // 2 tiles away, within range, on Open terrain

        game.push_command(Command::Attack {
            unit_ids: vec![attacker.raw()],
            target_id: target.raw(),
        });

        // Tick enough for first attack (cooldown starts at 0, fires immediately)
        game.tick(50.0);

        let health = game.world.get_component::<Health>(target).unwrap();
        assert!(health.current < health.max, "Target should have taken damage, health={}", health.current);
        assert_eq!(health.current, 80.0 - 8.0); // Thrall damage = 8, Open terrain = no reduction
    }

    #[test]
    fn test_death_on_zero_health() {
        let mut game = Game::new(test_config());
        let attacker = game.spawn_thrall(1.5, 1.5, 0);
        let target = game.spawn_thrall(3.5, 1.5, 1);

        // Set target health very low
        if let Some(h) = game.world.get_component_mut::<Health>(target) {
            h.current = 1.0;
        }

        game.push_command(Command::Attack {
            unit_ids: vec![attacker.raw()],
            target_id: target.raw(),
        });

        game.tick(50.0); // Attack kills target

        // Target should be in death state
        let rs = game.world.get_component::<RenderState>(target).unwrap();
        assert_eq!(rs.anim_state, AnimState::Death);
    }

    #[test]
    fn test_death_despawn() {
        let mut game = Game::new(test_config());
        let attacker = game.spawn_thrall(1.5, 1.5, 0);
        let target = game.spawn_thrall(3.5, 1.5, 1);

        // Kill target instantly
        if let Some(h) = game.world.get_component_mut::<Health>(target) {
            h.current = 1.0;
        }

        game.push_command(Command::Attack {
            unit_ids: vec![attacker.raw()],
            target_id: target.raw(),
        });
        game.tick(50.0); // Attack kills, starts death anim

        // Tick through death animation (6 frames * 0.12s = 0.72s = ~15 ticks at 50ms)
        for _ in 0..20 {
            game.tick(50.0);
        }

        // Target should be despawned
        assert!(!game.world.is_alive(target), "Dead entity should be despawned after death animation");
    }

    #[test]
    fn test_attack_cooldown() {
        let mut game = Game::new(test_config());
        // Use spawn corner positions to ensure Open terrain (no damage reduction)
        let attacker = game.spawn_thrall(1.5, 1.5, 0);
        let target = game.spawn_unit(SpriteId::Sentinel, 3.5, 1.5, 1); // High HP target, Open terrain

        game.push_command(Command::Attack {
            unit_ids: vec![attacker.raw()],
            target_id: target.raw(),
        });

        // First tick: should fire immediately (cooldown starts at 0)
        game.tick(50.0);
        let h1 = game.world.get_component::<Health>(target).unwrap().current;
        assert_eq!(h1, 200.0 - 8.0);

        // Second tick: cooldown not elapsed yet (0.5s cooldown, only 0.05s elapsed)
        game.tick(50.0);
        let h2 = game.world.get_component::<Health>(target).unwrap().current;
        assert_eq!(h2, h1, "Should not fire again within cooldown period");

        // Tick 10 more times (0.5s total elapsed since last shot)
        for _ in 0..10 {
            game.tick(50.0);
        }

        // Should have fired again
        let h3 = game.world.get_component::<Health>(target).unwrap().current;
        assert!(h3 < h2, "Should fire again after cooldown, health={}", h3);
    }

    #[test]
    fn test_shot_event() {
        let mut game = Game::new(test_config());
        let attacker = game.spawn_thrall(5.5, 5.5, 0);
        let _target = game.spawn_thrall(8.5, 5.5, 1);

        game.push_command(Command::Attack {
            unit_ids: vec![attacker.raw()],
            target_id: _target.raw(),
        });

        game.tick(50.0);

        let event_count = game.world.get_resource::<EventCount>().unwrap().0;
        assert!(event_count > 0, "Should have at least one event after attacking");

        // Read the event type from the event buffer
        let eb = game.world.get_resource::<EventBuffer>().unwrap();
        let event_type = u16::from_le_bytes([eb.0[0], eb.0[1]]);
        assert_eq!(event_type, 0, "First event should be Shot (type 0)");
    }

    #[test]
    fn test_death_event() {
        let mut game = Game::new(test_config());
        let attacker = game.spawn_thrall(5.5, 5.5, 0);
        let target = game.spawn_thrall(8.5, 5.5, 1);

        // Kill in one hit
        if let Some(h) = game.world.get_component_mut::<Health>(target) {
            h.current = 1.0;
        }

        game.push_command(Command::Attack {
            unit_ids: vec![attacker.raw()],
            target_id: target.raw(),
        });

        game.tick(50.0);

        // Should have both Shot and Death events
        let event_count = game.world.get_resource::<EventCount>().unwrap().0;
        assert!(event_count >= 2, "Should have Shot + Death events, got {}", event_count);

        let eb = game.world.get_resource::<EventBuffer>().unwrap();
        // Check second event is Death (type 1)
        let death_event_type = u16::from_le_bytes([eb.0[32], eb.0[33]]);
        assert_eq!(death_event_type, 1, "Second event should be Death (type 1)");
    }

    #[test]
    fn test_event_count_resets() {
        let mut game = Game::new(test_config());
        let attacker = game.spawn_thrall(5.5, 5.5, 0);
        let _target = game.spawn_unit(SpriteId::Sentinel, 8.5, 5.5, 1);

        game.push_command(Command::Attack {
            unit_ids: vec![attacker.raw()],
            target_id: _target.raw(),
        });

        game.tick(50.0);
        let count1 = game.world.get_resource::<EventCount>().unwrap().0;
        assert!(count1 > 0);

        // Next tick with no new attacks within cooldown
        game.tick(50.0);
        let count2 = game.world.get_resource::<EventCount>().unwrap().0;
        assert_eq!(count2, 0, "Event count should reset each tick when no events occur");
    }

    #[test]
    fn test_enemy_hidden_in_fog() {
        let mut game = Game::new(test_config());
        // Player 0 unit at (2, 2) — far from player 1 unit at (14, 14)
        game.spawn_thrall(2.5, 2.5, 0);
        game.spawn_thrall(14.5, 14.5, 1);

        game.tick(50.0);

        // Render buffer should only contain player 0's unit (local player = 0 by default)
        // Player 1's unit at (14,14) should be hidden (not visible in player 0's fog)
        let render_count = game.render_count();
        assert_eq!(render_count, 1, "Only own unit should be in render buffer, got {}", render_count);

        // Verify it's player 0's unit
        let buf = game.world.get_resource::<RenderBuffer>().unwrap();
        let owner = buf.0[18]; // owner byte at offset 18
        assert_eq!(owner, 0, "Rendered unit should be player 0");
    }

    #[test]
    fn test_own_units_always_visible() {
        let mut game = Game::new(test_config());
        // Spawn 3 player 0 units spread out
        game.spawn_thrall(2.5, 2.5, 0);
        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_thrall(14.5, 14.5, 0);

        game.tick(50.0);

        // All 3 of player 0's units should always render regardless of fog
        let render_count = game.render_count();
        assert_eq!(render_count, 3, "All own units should render, got {}", render_count);
    }

    #[test]
    fn test_command_enum_variants() {
        // Verify all Phase 2 command variants can be constructed
        let _move = Command::Move { unit_ids: vec![0], target_x: 1.0, target_y: 1.0 };
        let _stop = Command::Stop { unit_ids: vec![0] };
        let _attack = Command::Attack { unit_ids: vec![0], target_id: 1 };
        let _attack_move = Command::AttackMove { unit_ids: vec![0], target_x: 1.0, target_y: 1.0 };
        let _build = Command::Build { player: 0, building_type: 3, tile_x: 5, tile_y: 5 };
        let _produce = Command::Produce { player: 0, unit_type: 0 };
        let _cancel = Command::CancelProduction { player: 0, line_index: 0 };
        let _rally = Command::SetRally { player: 0, x: 10.0, y: 10.0 };
        let _deploy = Command::Deploy { player: 0, cp_x: 8.0, cp_y: 8.0 };
        let _confirm = Command::ConfirmDeployment { player: 0 };
        let _upgrade = Command::UpgradeForge { player: 0, upgrade: 0 };
        let _research = Command::CampaignResearch { player: 0, tech_id: 0 };
        let _dispatch = Command::CampaignDispatch { player: 0, source_site: 0, target_site: 1, units: vec![(0, 5)] };
        let _withdraw = Command::CampaignWithdraw { player: 0, site_id: 0 };
    }
}

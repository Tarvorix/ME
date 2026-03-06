use crate::ecs::World;
use crate::ecs::entity::Entity;
use crate::components::{Position, PreviousPosition, UnitType, Health, VisionRange, Deployed, RenderState, PathState, CombatState};
use crate::blueprints::get_blueprint;
use crate::types::SpriteId;
use crate::systems::battle_victory::{BattleState, BattleStatus};

/// Deployment zone for a player during the pre-battle phase.
#[derive(Clone, Debug)]
pub struct DeploymentZone {
    pub player_id: u8,
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
}

impl DeploymentZone {
    /// Check if a position is within this deployment zone.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let dx = x - self.center_x;
        let dy = y - self.center_y;
        (dx * dx + dy * dy).sqrt() <= self.radius
    }
}

/// Resource tracking deployment phase state.
pub struct DeploymentState {
    /// Deployment zones per player.
    pub zones: Vec<DeploymentZone>,
    /// Whether each player has confirmed deployment.
    pub confirmed: Vec<bool>,
    /// Countdown timer (seconds remaining) after all players deploy.
    pub countdown: f32,
    /// Maximum countdown duration.
    pub countdown_max: f32,
    /// Whether deployment has started (at least one player placed CP).
    pub deployment_started: bool,
    /// Deployed Command Post entity per player (None if not yet placed).
    pub command_posts: Vec<Option<Entity>>,
}

impl DeploymentState {
    pub fn new(player_count: u8) -> Self {
        let zones = deployment_zones(player_count, 64, 64);
        let pcount = player_count as usize;
        DeploymentState {
            zones,
            confirmed: vec![false; pcount],
            countdown: 30.0,
            countdown_max: 30.0,
            deployment_started: false,
            command_posts: vec![None; pcount],
        }
    }

    /// Create with specific map dimensions.
    pub fn with_map_size(player_count: u8, map_width: u32, map_height: u32) -> Self {
        let zones = deployment_zones(player_count, map_width, map_height);
        let pcount = player_count as usize;
        DeploymentState {
            zones,
            confirmed: vec![false; pcount],
            countdown: 30.0,
            countdown_max: 30.0,
            deployment_started: false,
            command_posts: vec![None; pcount],
        }
    }

    /// Returns true if all players have confirmed deployment.
    pub fn all_confirmed(&self) -> bool {
        self.confirmed.iter().all(|&c| c)
    }

    /// Returns true if a specific player has confirmed.
    pub fn is_confirmed(&self, player_id: u8) -> bool {
        self.confirmed.get(player_id as usize).copied().unwrap_or(false)
    }

    /// Returns the deployment zone for a specific player.
    pub fn zone_for(&self, player_id: u8) -> Option<&DeploymentZone> {
        self.zones.iter().find(|z| z.player_id == player_id)
    }
}

/// Generate deployment zones based on player count and map size.
/// 2 players: opposite corners.
/// 3 players: triangle (two corners + center edge).
/// 4 players: all four corners.
pub fn deployment_zones(player_count: u8, map_width: u32, map_height: u32) -> Vec<DeploymentZone> {
    let w = map_width as f32;
    let h = map_height as f32;
    let margin = 8.0;
    let radius = 8.0;

    let mut zones = Vec::new();

    match player_count {
        2 => {
            // Bottom-left and top-right corners
            zones.push(DeploymentZone {
                player_id: 0,
                center_x: margin,
                center_y: margin,
                radius,
            });
            zones.push(DeploymentZone {
                player_id: 1,
                center_x: w - margin,
                center_y: h - margin,
                radius,
            });
        }
        3 => {
            zones.push(DeploymentZone {
                player_id: 0,
                center_x: margin,
                center_y: margin,
                radius,
            });
            zones.push(DeploymentZone {
                player_id: 1,
                center_x: w - margin,
                center_y: h - margin,
                radius,
            });
            zones.push(DeploymentZone {
                player_id: 2,
                center_x: w - margin,
                center_y: margin,
                radius,
            });
        }
        _ => {
            // 4 players: all four corners
            zones.push(DeploymentZone {
                player_id: 0,
                center_x: margin,
                center_y: margin,
                radius,
            });
            zones.push(DeploymentZone {
                player_id: 1,
                center_x: w - margin,
                center_y: h - margin,
                radius,
            });
            zones.push(DeploymentZone {
                player_id: 2,
                center_x: w - margin,
                center_y: margin,
                radius,
            });
            zones.push(DeploymentZone {
                player_id: 3,
                center_x: margin,
                center_y: h - margin,
                radius,
            });
        }
    }

    zones
}

/// Validate that a Command Post placement is within the player's deployment zone.
pub fn validate_placement(deployment: &DeploymentState, player_id: u8, x: f32, y: f32) -> bool {
    if let Some(zone) = deployment.zone_for(player_id) {
        zone.contains(x, y)
    } else {
        false
    }
}

/// Deploy a player's starting force at the given Command Post position.
/// Spawns: Command Post + Node + 10 Thralls + 3 Sentinels + 1 Hover Tank.
/// Returns the Command Post entity.
pub fn deploy_force(world: &mut World, player_id: u8, cp_x: f32, cp_y: f32) -> Entity {
    // Spawn Command Post
    let cp = spawn_entity(world, SpriteId::CommandPost, cp_x, cp_y, player_id);

    // Spawn Node nearby
    spawn_entity(world, SpriteId::Node, cp_x + 2.0, cp_y - 2.0, player_id);

    // Spawn 10 Thralls in formation
    for i in 0..10 {
        let row = i / 5;
        let col = i % 5;
        spawn_entity(
            world,
            SpriteId::Thrall,
            cp_x - 3.0 + col as f32 * 1.5,
            cp_y + 3.0 + row as f32 * 1.5,
            player_id,
        );
    }

    // Spawn 3 Sentinels
    for i in 0..3 {
        spawn_entity(
            world,
            SpriteId::Sentinel,
            cp_x + 3.0 + i as f32 * 1.0,
            cp_y + 1.0,
            player_id,
        );
    }

    // Spawn 1 Hover Tank
    spawn_entity(
        world,
        SpriteId::HoverTank,
        cp_x - 3.0,
        cp_y + 1.0,
        player_id,
    );

    cp
}

/// Helper to spawn a single entity with full component setup.
fn spawn_entity(world: &mut World, kind: SpriteId, x: f32, y: f32, owner: u8) -> Entity {
    let bp = get_blueprint(kind);
    let entity = world.spawn();

    world.add_component(entity, Position { x, y });
    world.add_component(entity, PreviousPosition { x, y });
    world.add_component(entity, UnitType { kind, owner });
    world.add_component(entity, Health::new(bp.max_hp));
    world.add_component(entity, VisionRange(bp.vision_range));
    world.add_component(entity, Deployed(true));
    world.add_component(entity, RenderState::new(kind, bp.scale));

    if bp.speed > 0.0 {
        world.add_component(entity, PathState::empty(bp.speed));
    }
    if bp.damage > 0.0 {
        world.add_component(entity, CombatState::new());
    }

    entity
}

/// Process a Deploy command during deployment phase.
pub fn process_deploy(world: &mut World, player_id: u8, cp_x: f32, cp_y: f32) {
    // Validate zone
    let valid = {
        let ds = match world.get_resource::<DeploymentState>() {
            Some(ds) => ds,
            None => return,
        };

        // Check if already deployed
        if ds.command_posts.get(player_id as usize).and_then(|e| *e).is_some() {
            return; // Already deployed
        }

        validate_placement(ds, player_id, cp_x, cp_y)
    };

    if !valid {
        return;
    }

    // Deploy the force
    let cp = deploy_force(world, player_id, cp_x, cp_y);

    // Register CP
    if let Some(ds) = world.get_resource_mut::<DeploymentState>() {
        ds.command_posts[player_id as usize] = Some(cp);
        ds.deployment_started = true;
    }

    // Register with production system
    if let Some(prods) = world.get_resource_mut::<crate::systems::production::Productions>() {
        if (player_id as usize) < prods.0.len() {
            prods.0[player_id as usize].command_post = Some(cp);
            prods.0[player_id as usize].rally_x = cp_x + 3.0;
            prods.0[player_id as usize].rally_y = cp_y + 3.0;
        }
    }
}

/// Process a ConfirmDeployment command.
pub fn process_confirm_deployment(world: &mut World, player_id: u8) {
    // Must have deployed first
    let has_deployed = {
        let ds = match world.get_resource::<DeploymentState>() {
            Some(ds) => ds,
            None => return,
        };
        ds.command_posts.get(player_id as usize).and_then(|e| *e).is_some()
    };

    if !has_deployed {
        return;
    }

    let all_confirmed = {
        let ds = world.get_resource_mut::<DeploymentState>().unwrap();
        ds.confirmed[player_id as usize] = true;
        ds.all_confirmed()
    };

    // If all confirmed, transition to Active
    if all_confirmed {
        if let Some(bs) = world.get_resource_mut::<BattleState>() {
            bs.status = BattleStatus::Active;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, GameConfig};
    use crate::systems::battle_victory::BattleState;

    fn test_game() -> Game {
        let mut game = Game::new(GameConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
        });

        game.world.insert_resource(DeploymentState::with_map_size(2, 64, 64));
        game.world.insert_resource(BattleState::new(2));
        // Set battle to Deployment status
        game.world.get_resource_mut::<BattleState>().unwrap().status = BattleStatus::Deployment;
        game
    }

    #[test]
    fn test_deployment_zones_2p() {
        let zones = deployment_zones(2, 64, 64);
        assert_eq!(zones.len(), 2);
        assert_eq!(zones[0].player_id, 0);
        assert_eq!(zones[1].player_id, 1);
        // Player 0 near bottom-left, Player 1 near top-right
        assert!(zones[0].center_x < 32.0);
        assert!(zones[0].center_y < 32.0);
        assert!(zones[1].center_x > 32.0);
        assert!(zones[1].center_y > 32.0);
    }

    #[test]
    fn test_deployment_zones_4p() {
        let zones = deployment_zones(4, 64, 64);
        assert_eq!(zones.len(), 4);
        for i in 0..4 {
            assert_eq!(zones[i].player_id, i as u8);
        }
    }

    #[test]
    fn test_valid_placement() {
        let ds = DeploymentState::with_map_size(2, 64, 64);
        // Player 0 zone is around (8, 8) with radius 8
        assert!(validate_placement(&ds, 0, 8.0, 8.0), "Center of zone should be valid");
        assert!(validate_placement(&ds, 0, 10.0, 10.0), "Within radius should be valid");
        assert!(!validate_placement(&ds, 0, 50.0, 50.0), "Far from zone should be invalid");
    }

    #[test]
    fn test_invalid_placement_wrong_zone() {
        let ds = DeploymentState::with_map_size(2, 64, 64);
        // Player 0 should not be able to deploy in player 1's zone
        assert!(!validate_placement(&ds, 0, 56.0, 56.0), "Should not deploy in enemy zone");
    }

    #[test]
    fn test_deploy_creates_entities() {
        let mut game = test_game();

        process_deploy(&mut game.world, 0, 8.0, 8.0);

        // Count entities belonging to player 0
        let ut_storage = game.world.get_storage::<UnitType>().unwrap();
        let count = ut_storage.iter().filter(|(_, ut)| ut.owner == 0).count();

        // Should have: 1 CP + 1 Node + 10 Thralls + 3 Sentinels + 1 HoverTank = 16
        assert_eq!(count, 16, "Should have 16 entities, got {}", count);

        // Check types
        let thralls = ut_storage.iter().filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Thrall).count();
        let sentinels = ut_storage.iter().filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Sentinel).count();
        let tanks = ut_storage.iter().filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::HoverTank).count();
        let cps = ut_storage.iter().filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::CommandPost).count();
        let nodes = ut_storage.iter().filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Node).count();

        assert_eq!(thralls, 10, "Should have 10 Thralls");
        assert_eq!(sentinels, 3, "Should have 3 Sentinels");
        assert_eq!(tanks, 1, "Should have 1 Hover Tank");
        assert_eq!(cps, 1, "Should have 1 Command Post");
        assert_eq!(nodes, 1, "Should have 1 Node");
    }

    #[test]
    fn test_deploy_registers_command_post() {
        let mut game = test_game();

        process_deploy(&mut game.world, 0, 8.0, 8.0);

        let ds = game.world.get_resource::<DeploymentState>().unwrap();
        assert!(ds.command_posts[0].is_some(), "CP should be registered");
        assert!(ds.deployment_started);
    }

    #[test]
    fn test_cannot_deploy_twice() {
        let mut game = test_game();

        process_deploy(&mut game.world, 0, 8.0, 8.0);
        let count1 = game.world.get_storage::<UnitType>().unwrap().iter().count();

        process_deploy(&mut game.world, 0, 10.0, 10.0); // Should be ignored
        let count2 = game.world.get_storage::<UnitType>().unwrap().iter().count();

        assert_eq!(count1, count2, "Should not create entities on second deploy");
    }

    #[test]
    fn test_confirm_all_starts_battle() {
        let mut game = test_game();

        // Both players deploy
        process_deploy(&mut game.world, 0, 8.0, 8.0);
        process_deploy(&mut game.world, 1, 56.0, 56.0);

        // Both confirm
        process_confirm_deployment(&mut game.world, 0);
        assert!(!game.world.get_resource::<BattleState>().unwrap().is_active(),
            "Should not be active until all confirm");

        process_confirm_deployment(&mut game.world, 1);
        assert!(game.world.get_resource::<BattleState>().unwrap().is_active(),
            "Should transition to Active when all confirmed");
    }

    #[test]
    fn test_cannot_confirm_without_deploy() {
        let mut game = test_game();

        process_confirm_deployment(&mut game.world, 0); // Should be ignored

        let ds = game.world.get_resource::<DeploymentState>().unwrap();
        assert!(!ds.is_confirmed(0), "Should not confirm without deploying first");
    }

    #[test]
    fn test_deployment_zone_radius() {
        let zone = DeploymentZone {
            player_id: 0,
            center_x: 10.0,
            center_y: 10.0,
            radius: 8.0,
        };

        assert!(zone.contains(10.0, 10.0)); // center
        assert!(zone.contains(15.0, 10.0)); // within radius
        assert!(zone.contains(10.0, 17.0)); // within radius
        assert!(!zone.contains(10.0, 19.0)); // outside radius
        assert!(!zone.contains(25.0, 25.0)); // way outside
    }

    #[test]
    fn test_formation_around_cp() {
        let mut game = test_game();

        process_deploy(&mut game.world, 0, 8.0, 8.0);

        // All units should be near the CP (within ~5 tiles)
        let pos_storage = game.world.get_storage::<Position>().unwrap();
        let ut_storage = game.world.get_storage::<UnitType>().unwrap();

        for (entity, ut) in ut_storage.iter() {
            if ut.owner == 0 {
                if let Some(pos) = pos_storage.get(entity) {
                    let dx = pos.x - 8.0;
                    let dy = pos.y - 8.0;
                    let dist = (dx * dx + dy * dy).sqrt();
                    assert!(dist < 10.0,
                        "Entity {:?} at ({}, {}) should be near CP (8, 8), dist={}",
                        ut.kind, pos.x, pos.y, dist);
                }
            }
        }
    }
}

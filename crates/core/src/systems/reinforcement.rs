use crate::ecs::World;
use crate::components::{Position, PreviousPosition, UnitType, Health, VisionRange, Deployed, RenderState, PathState, CombatState};
use crate::blueprints::get_blueprint;
use crate::types::{SpriteId, EventType};
use crate::deployment::DeploymentState;
use crate::systems::resource::UIStateBuffer;

/// Minimum ticks between reinforcement requests per player (3 seconds at 20Hz).
const REINFORCEMENT_COOLDOWN_TICKS: u32 = 60;

/// A single pending reinforcement request waiting to arrive.
#[derive(Clone, Debug)]
pub struct ReinforcementRequest {
    pub player: u8,
    pub unit_type: SpriteId,
    pub count: u32,
    pub health_pct: f32,
    pub ticks_remaining: u32,
}

/// Resource: pending reinforcement requests for this battle.
pub struct PendingReinforcements {
    pub requests: Vec<ReinforcementRequest>,
    /// Per-player cooldown (ticks remaining before next request allowed).
    pub cooldowns: Vec<u32>,
}

impl PendingReinforcements {
    pub fn new(player_count: u32) -> Self {
        PendingReinforcements {
            requests: Vec::new(),
            cooldowns: vec![0; player_count as usize],
        }
    }

    /// Check if a player is off cooldown.
    pub fn can_request(&self, player: u8) -> bool {
        self.cooldowns.get(player as usize).copied().unwrap_or(0) == 0
    }

    /// Start cooldown for a player.
    pub fn start_cooldown(&mut self, player: u8) {
        if let Some(cd) = self.cooldowns.get_mut(player as usize) {
            *cd = REINFORCEMENT_COOLDOWN_TICKS;
        }
    }
}

/// Resource: what reinforcements are available from the campaign Node garrison.
/// Updated by CampaignGame each tick for each active battle.
#[derive(Clone, Debug)]
pub struct ReinforcementAvailability {
    /// Per-player availability: (unit_type as u16, count available).
    pub available: Vec<Vec<(u16, u32)>>,
    /// Whether each player's CP is alive in this battle.
    pub cp_alive: Vec<bool>,
}

impl ReinforcementAvailability {
    pub fn new(player_count: u32) -> Self {
        ReinforcementAvailability {
            available: (0..player_count).map(|_| Vec::new()).collect(),
            cp_alive: vec![false; player_count as usize],
        }
    }
}

/// Reinforcement system: counts down arrival timers, spawns units at CP when ready.
pub fn reinforcement_system(world: &mut World) {
    // Tick cooldowns
    {
        if let Some(pending) = world.get_resource_mut::<PendingReinforcements>() {
            for cd in pending.cooldowns.iter_mut() {
                if *cd > 0 {
                    *cd -= 1;
                }
            }
        }
    }

    // Collect requests that have arrived
    let mut arrived: Vec<ReinforcementRequest> = Vec::new();
    {
        let pending = match world.get_resource_mut::<PendingReinforcements>() {
            Some(p) => p,
            None => return,
        };

        let mut i = 0;
        while i < pending.requests.len() {
            pending.requests[i].ticks_remaining = pending.requests[i].ticks_remaining.saturating_sub(1);
            if pending.requests[i].ticks_remaining == 0 {
                arrived.push(pending.requests.remove(i));
            } else {
                i += 1;
            }
        }
    }

    // Spawn arrived reinforcements at CP position
    for req in &arrived {
        // Find CP position for this player
        let cp_pos = {
            let ds = match world.get_resource::<DeploymentState>() {
                Some(ds) => ds,
                None => continue,
            };

            let cp_entity = match ds.command_posts.get(req.player as usize) {
                Some(Some(e)) => *e,
                _ => continue,
            };

            // Check CP is alive
            let health_storage = world.get_storage::<Health>();
            if let Some(hs) = health_storage {
                if let Some(h) = hs.get(cp_entity) {
                    if h.is_dead() {
                        continue; // CP destroyed, reinforcements lost
                    }
                }
            }

            match world.get_component::<Position>(cp_entity) {
                Some(pos) => (pos.x, pos.y),
                None => continue,
            }
        };

        // Spawn units near CP
        let bp = get_blueprint(req.unit_type);
        for i in 0..req.count {
            let row = i / 5;
            let col = i % 5;
            let x = cp_pos.0 - 2.0 + col as f32 * 1.0;
            let y = cp_pos.1 + 2.0 + row as f32 * 1.0;

            let entity = world.spawn();
            world.add_component(entity, Position { x, y });
            world.add_component(entity, PreviousPosition { x, y });
            world.add_component(entity, UnitType { kind: req.unit_type, owner: req.player });
            world.add_component(entity, Health {
                current: bp.max_hp * req.health_pct,
                max: bp.max_hp,
            });
            world.add_component(entity, VisionRange(bp.vision_range));
            world.add_component(entity, Deployed(true));
            world.add_component(entity, RenderState::new(req.unit_type, bp.scale));

            if bp.speed > 0.0 {
                world.add_component(entity, PathState::empty(bp.speed));
            }
            if bp.damage > 0.0 {
                world.add_component(entity, CombatState::new());
            }

            // Emit UnitSpawned event for visual/audio feedback
            crate::game::write_event(
                world,
                EventType::UnitSpawned,
                entity.raw(),
                x, y,
                &[0u8; 16],
            );
        }
    }
}

/// Write reinforcement UI data to UIStateBuffer bytes [196-255].
///
/// Layout (per player 0, local player):
/// [196] cp_alive: u8 (0 or 1)
/// [197] cooldown_remaining: u8 (ticks remaining)
/// [198] pending_count: u8 (number of pending requests)
/// [199] reserved
/// [200-203] available_thrall_count: u32
/// [204-207] available_sentinel_count: u32
/// [208-211] available_tank_count: u32
/// [212-215] pending_request_0: unit_type u16 + count u16
/// [216-219] pending_request_0_ticks: u32
/// [220-223] pending_request_1: unit_type u16 + count u16
/// [224-227] pending_request_1_ticks: u32
/// [228-231] pending_request_2: unit_type u16 + count u16
/// [232-235] pending_request_2_ticks: u32
pub fn write_reinforcement_ui(world: &mut World) {
    let (cp_alive, cooldown, pending_count, pending_requests) = {
        let avail = world.get_resource::<ReinforcementAvailability>();
        let pending = world.get_resource::<PendingReinforcements>();

        let cp_alive = avail.as_ref()
            .and_then(|a| a.cp_alive.get(0))
            .copied()
            .unwrap_or(false);

        let cooldown = pending.as_ref()
            .and_then(|p| p.cooldowns.get(0))
            .copied()
            .unwrap_or(0);

        let player_requests: Vec<(u16, u32, u32)> = pending.as_ref()
            .map(|p| {
                p.requests.iter()
                    .filter(|r| r.player == 0)
                    .take(3)
                    .map(|r| (r.unit_type as u16, r.count, r.ticks_remaining))
                    .collect()
            })
            .unwrap_or_default();

        let pending_count = player_requests.len() as u8;

        (cp_alive, cooldown, pending_count, player_requests)
    };

    let (avail_thralls, avail_sentinels, avail_tanks) = {
        let avail = world.get_resource::<ReinforcementAvailability>();
        if let Some(a) = avail {
            if let Some(player_avail) = a.available.get(0) {
                let thralls = player_avail.iter().find(|(t, _)| *t == 0).map(|(_, c)| *c).unwrap_or(0);
                let sentinels = player_avail.iter().find(|(t, _)| *t == 1).map(|(_, c)| *c).unwrap_or(0);
                let tanks = player_avail.iter().find(|(t, _)| *t == 2).map(|(_, c)| *c).unwrap_or(0);
                (thralls, sentinels, tanks)
            } else {
                (0, 0, 0)
            }
        } else {
            (0, 0, 0)
        }
    };

    let buf = if let Some(ui) = world.get_resource_mut::<UIStateBuffer>() {
        &mut ui.0
    } else {
        return;
    };

    // [196] cp_alive
    buf[196] = if cp_alive { 1 } else { 0 };
    // [197] cooldown_remaining
    buf[197] = cooldown.min(255) as u8;
    // [198] pending_count
    buf[198] = pending_count;
    // [199] reserved
    buf[199] = 0;
    // [200-203] available_thrall_count
    buf[200..204].copy_from_slice(&avail_thralls.to_le_bytes());
    // [204-207] available_sentinel_count
    buf[204..208].copy_from_slice(&avail_sentinels.to_le_bytes());
    // [208-211] available_tank_count
    buf[208..212].copy_from_slice(&avail_tanks.to_le_bytes());

    // Write up to 3 pending requests
    for i in 0..3 {
        let base = 212 + i * 8;
        if i < pending_requests.len() {
            let (unit_type, count, ticks) = pending_requests[i];
            buf[base..base + 2].copy_from_slice(&unit_type.to_le_bytes());
            buf[base + 2..base + 4].copy_from_slice(&(count as u16).to_le_bytes());
            buf[base + 4..base + 8].copy_from_slice(&ticks.to_le_bytes());
        } else {
            buf[base..base + 8].fill(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, GameConfig};
    use crate::deployment::DeploymentState;
    use crate::systems::battle_victory::{BattleState, BattleStatus};
    use crate::systems::resource::UIStateBuffer;

    fn test_game_with_reinforcements() -> Game {
        let mut game = Game::new(GameConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
        });

        // Set up deployment state
        let mut deployment = DeploymentState::with_map_size(2, 64, 64);

        // Spawn CPs for both players
        let cp0 = game.spawn_command_post(8.0, 8.0, 0);
        let cp1 = game.spawn_command_post(56.0, 56.0, 1);
        deployment.command_posts = vec![Some(cp0), Some(cp1)];
        deployment.confirmed = vec![true, true];

        game.world.insert_resource(deployment);

        // Set up battle state
        let mut battle_state = BattleState::new(2);
        battle_state.status = BattleStatus::Active;
        game.world.insert_resource(battle_state);

        // Set up reinforcement resources
        game.world.insert_resource(PendingReinforcements::new(2));
        let mut avail = ReinforcementAvailability::new(2);
        avail.cp_alive = vec![true, true];
        avail.available = vec![
            vec![(0, 10), (1, 5), (2, 2)], // Player 0: 10 Thralls, 5 Sentinels, 2 Tanks
            vec![(0, 8), (1, 3)],            // Player 1: 8 Thralls, 3 Sentinels
        ];
        game.world.insert_resource(avail);

        game
    }

    #[test]
    fn test_pending_reinforcements_cooldown() {
        let mut pr = PendingReinforcements::new(2);
        assert!(pr.can_request(0));
        assert!(pr.can_request(1));

        pr.start_cooldown(0);
        assert!(!pr.can_request(0));
        assert!(pr.can_request(1));

        // Tick down cooldown
        for cd in pr.cooldowns.iter_mut() {
            if *cd > 0 { *cd -= 1; }
        }
        assert!(!pr.can_request(0)); // Still 59 ticks remaining
    }

    #[test]
    fn test_reinforcement_request_added() {
        let mut pr = PendingReinforcements::new(2);
        pr.requests.push(ReinforcementRequest {
            player: 0,
            unit_type: SpriteId::Thrall,
            count: 5,
            health_pct: 1.0,
            ticks_remaining: 100,
        });
        assert_eq!(pr.requests.len(), 1);
        assert_eq!(pr.requests[0].ticks_remaining, 100);
    }

    #[test]
    fn test_reinforcement_spawns_units() {
        let mut game = test_game_with_reinforcements();

        // Add a reinforcement request about to arrive
        if let Some(pr) = game.world.get_resource_mut::<PendingReinforcements>() {
            pr.requests.push(ReinforcementRequest {
                player: 0,
                unit_type: SpriteId::Thrall,
                count: 3,
                health_pct: 1.0,
                ticks_remaining: 1, // Arrives next tick
            });
        }

        // Count entities before
        let count_before = game.world.get_storage::<UnitType>().unwrap()
            .iter().filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Thrall).count();

        game.tick(50.0);

        // Count entities after
        let count_after = game.world.get_storage::<UnitType>().unwrap()
            .iter().filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Thrall).count();

        assert_eq!(count_after, count_before + 3, "Should have spawned 3 Thralls, got {} -> {}", count_before, count_after);

        // Request should be consumed
        let pr = game.world.get_resource::<PendingReinforcements>().unwrap();
        assert!(pr.requests.is_empty(), "Request should be consumed after spawning");
    }

    #[test]
    fn test_reinforcement_with_reduced_health() {
        let mut game = test_game_with_reinforcements();

        if let Some(pr) = game.world.get_resource_mut::<PendingReinforcements>() {
            pr.requests.push(ReinforcementRequest {
                player: 0,
                unit_type: SpriteId::Sentinel,
                count: 2,
                health_pct: 0.5,
                ticks_remaining: 1,
            });
        }

        game.tick(50.0);

        // Check spawned sentinels have 50% health
        let ut_s = game.world.get_storage::<UnitType>().unwrap();
        let h_s = game.world.get_storage::<Health>().unwrap();

        for (e, ut) in ut_s.iter() {
            if ut.owner == 0 && ut.kind == SpriteId::Sentinel {
                let h = h_s.get(e).unwrap();
                assert!((h.current - 100.0).abs() < 0.01,
                    "Sentinel should have 100 HP (50% of 200), got {}", h.current);
            }
        }
    }

    #[test]
    fn test_reinforcement_blocked_when_cp_dead() {
        let mut game = test_game_with_reinforcements();

        // Kill player 0's CP
        {
            let ds = game.world.get_resource::<DeploymentState>().unwrap();
            let cp = ds.command_posts[0].unwrap();
            if let Some(h) = game.world.get_component_mut::<Health>(cp) {
                h.current = 0.0;
            }
        }

        // Add reinforcement request
        if let Some(pr) = game.world.get_resource_mut::<PendingReinforcements>() {
            pr.requests.push(ReinforcementRequest {
                player: 0,
                unit_type: SpriteId::Thrall,
                count: 5,
                health_pct: 1.0,
                ticks_remaining: 1,
            });
        }

        let count_before = game.world.get_storage::<UnitType>().unwrap()
            .iter().filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Thrall).count();

        game.tick(50.0);

        let count_after = game.world.get_storage::<UnitType>().unwrap()
            .iter().filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Thrall).count();

        assert_eq!(count_after, count_before, "No units should spawn when CP is dead");
    }

    #[test]
    fn test_reinforcement_arrival_timing() {
        let mut game = test_game_with_reinforcements();

        // Request with 5 ticks remaining
        if let Some(pr) = game.world.get_resource_mut::<PendingReinforcements>() {
            pr.requests.push(ReinforcementRequest {
                player: 0,
                unit_type: SpriteId::Thrall,
                count: 2,
                health_pct: 1.0,
                ticks_remaining: 5,
            });
        }

        let count_before = game.world.get_storage::<UnitType>().unwrap()
            .iter().filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Thrall).count();

        // Tick 4 times — should NOT arrive yet
        for _ in 0..4 {
            game.tick(50.0);
        }

        let count_mid = game.world.get_storage::<UnitType>().unwrap()
            .iter().filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Thrall).count();
        assert_eq!(count_mid, count_before, "Should not spawn before arrival time");

        // 5th tick — should arrive
        game.tick(50.0);

        let count_after = game.world.get_storage::<UnitType>().unwrap()
            .iter().filter(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Thrall).count();
        assert_eq!(count_after, count_before + 2, "Should spawn after arrival time");
    }

    #[test]
    fn test_reinforcement_ui_data() {
        let mut game = test_game_with_reinforcements();

        // Add a pending request
        if let Some(pr) = game.world.get_resource_mut::<PendingReinforcements>() {
            pr.requests.push(ReinforcementRequest {
                player: 0,
                unit_type: SpriteId::Thrall,
                count: 5,
                health_pct: 1.0,
                ticks_remaining: 50,
            });
            pr.cooldowns[0] = 30;
        }

        write_reinforcement_ui(&mut game.world);

        let ui = game.world.get_resource::<UIStateBuffer>().unwrap();
        assert_eq!(ui.0[196], 1, "CP should be alive");
        assert_eq!(ui.0[197], 30, "Cooldown should be 30");
        assert_eq!(ui.0[198], 1, "Should have 1 pending request");

        // Available counts
        let avail_thralls = u32::from_le_bytes([ui.0[200], ui.0[201], ui.0[202], ui.0[203]]);
        assert_eq!(avail_thralls, 10);
        let avail_sentinels = u32::from_le_bytes([ui.0[204], ui.0[205], ui.0[206], ui.0[207]]);
        assert_eq!(avail_sentinels, 5);
        let avail_tanks = u32::from_le_bytes([ui.0[208], ui.0[209], ui.0[210], ui.0[211]]);
        assert_eq!(avail_tanks, 2);

        // Pending request 0
        let req_type = u16::from_le_bytes([ui.0[212], ui.0[213]]);
        assert_eq!(req_type, 0, "Request should be Thrall (type 0)");
        let req_count = u16::from_le_bytes([ui.0[214], ui.0[215]]);
        assert_eq!(req_count, 5, "Request should be for 5 units");
        let req_ticks = u32::from_le_bytes([ui.0[216], ui.0[217], ui.0[218], ui.0[219]]);
        assert_eq!(req_ticks, 50, "Request should have 50 ticks remaining");
    }

    #[test]
    fn test_cooldown_ticks_down() {
        let mut game = test_game_with_reinforcements();

        if let Some(pr) = game.world.get_resource_mut::<PendingReinforcements>() {
            pr.cooldowns[0] = 5;
        }

        game.tick(50.0);

        let pr = game.world.get_resource::<PendingReinforcements>().unwrap();
        assert_eq!(pr.cooldowns[0], 4, "Cooldown should tick down by 1");
    }

    #[test]
    fn test_spawn_event_on_arrival() {
        let mut game = test_game_with_reinforcements();

        if let Some(pr) = game.world.get_resource_mut::<PendingReinforcements>() {
            pr.requests.push(ReinforcementRequest {
                player: 0,
                unit_type: SpriteId::Thrall,
                count: 1,
                health_pct: 1.0,
                ticks_remaining: 1,
            });
        }

        game.tick(50.0);

        // Check that a UnitSpawned event was emitted
        let event_count = game.world.get_resource::<crate::game::EventCount>().unwrap().0;
        assert!(event_count > 0, "Should have events after reinforcement arrival");

        let eb = game.world.get_resource::<crate::game::EventBuffer>().unwrap();
        // Find a UnitSpawned event (type 2)
        let mut found_spawn = false;
        for i in 0..event_count as usize {
            let off = i * 32;
            let etype = u16::from_le_bytes([eb.0[off], eb.0[off + 1]]);
            if etype == 2 { // UnitSpawned
                found_spawn = true;
                break;
            }
        }
        assert!(found_spawn, "Should have a UnitSpawned event");
    }
}

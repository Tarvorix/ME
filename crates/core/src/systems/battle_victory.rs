use crate::ecs::World;
use crate::components::{UnitType, Health};
use crate::game::write_event;
use crate::systems::capture::CapturePointCounts;
use crate::types::{EventType, SpriteId};

/// Battle status progression.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BattleStatus {
    /// Pre-battle deployment phase.
    Deployment,
    /// Active battle in progress.
    Active,
    /// Battle has ended with a winner.
    Finished,
}

/// Reason for battle victory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VictoryReason {
    /// Player captured all capture points on the map.
    AllCapturePoints,
    /// Player held majority of capture points for 60 seconds.
    MajorityHold,
    /// All enemy units and buildings eliminated.
    TotalElimination,
}

/// Resource tracking battle state including win conditions.
pub struct BattleState {
    /// Current battle status.
    pub status: BattleStatus,
    /// Per-player majority hold timer (seconds accumulated).
    pub majority_hold_timer: [f32; 8],
    /// Threshold to win by majority hold (seconds).
    pub majority_hold_threshold: f32,
    /// Winner player ID (255 = no winner yet).
    pub winner: u8,
    /// Reason for victory.
    pub victory_reason: Option<VictoryReason>,
    /// Tick when battle started (to prevent early win).
    pub start_tick: u32,
    /// Number of players in the match.
    pub player_count: u8,
}

impl BattleState {
    pub fn new(player_count: u8) -> Self {
        BattleState {
            status: BattleStatus::Active,
            majority_hold_timer: [0.0; 8],
            majority_hold_threshold: 60.0,
            winner: 255,
            victory_reason: None,
            start_tick: 0,
            player_count,
        }
    }

    /// Returns true if the battle has ended.
    pub fn is_finished(&self) -> bool {
        self.status == BattleStatus::Finished
    }

    /// Returns true if battle is in active combat phase.
    pub fn is_active(&self) -> bool {
        self.status == BattleStatus::Active
    }
}

/// Minimum ticks before win conditions are checked (prevent immediate wins).
const MIN_TICKS_BEFORE_WIN: u32 = 200; // 10 seconds at 20 ticks/sec

/// Battle victory system: checks three win conditions each tick.
///
/// 1. **All Capture Points** — instant win if one player owns all CPs.
/// 2. **Majority Hold** — win after holding majority (>50%) of CPs for 60s continuously.
/// 3. **Total Elimination** — win if a player has no combat units AND no capture points.
pub fn battle_victory_system(world: &mut World) {
    // Check if battle is active
    let (is_active, player_count, _start_tick) = {
        let bs = match world.get_resource::<BattleState>() {
            Some(bs) => bs,
            None => return,
        };
        (bs.is_active(), bs.player_count, bs.start_tick)
    };

    if !is_active {
        return;
    }

    let delta_secs = if let Some(td) = world.get_resource::<crate::game::TickDelta>() {
        td.0
    } else {
        return;
    };

    let tick_count = {
        // Approximate tick count from delta (we'll check battle_state.start_tick)
        // Actually, we can check if enough ticks have passed via a simple counter
        // The game tick count is in Game struct, not directly accessible here.
        // Use a heuristic: check BattleState for start_tick offset
        0u32 // Will be set properly below
    };
    let _ = tick_count;

    // Update capture point counts
    let mut cp_counts = CapturePointCounts::new();
    cp_counts.update(world);

    // Count alive combat entities per player
    let mut alive_units: [u32; 8] = [0; 8];
    let mut alive_buildings: [u32; 8] = [0; 8];
    {
        let ut_storage = match world.get_storage::<UnitType>() {
            Some(s) => s,
            None => return,
        };
        let health_storage = world.get_storage::<Health>();

        for (_entity, ut) in ut_storage.iter() {
            let pid = ut.owner as usize;
            if pid >= 8 || ut.owner == 255 {
                continue; // skip neutral entities (capture points)
            }

            // Check alive
            let alive = if let Some(hs) = &health_storage {
                hs.get(_entity).map(|h| !h.is_dead()).unwrap_or(true)
            } else {
                true
            };

            if !alive {
                continue;
            }

            match ut.kind {
                SpriteId::Thrall | SpriteId::Sentinel | SpriteId::HoverTank => {
                    alive_units[pid] += 1;
                }
                SpriteId::CommandPost | SpriteId::Forge => {
                    alive_buildings[pid] += 1;
                }
                SpriteId::CapturePoint => {
                    // Don't count towards player's assets
                }
            }
        }
    }

    // Increment the internal tick counter in BattleState
    let current_tick = {
        let bs = world.get_resource_mut::<BattleState>().unwrap();
        bs.start_tick += 1;
        bs.start_tick
    };

    // Don't check win conditions in the first MIN_TICKS_BEFORE_WIN ticks
    if current_tick < MIN_TICKS_BEFORE_WIN {
        return;
    }

    let mut winner: Option<(u8, VictoryReason)> = None;

    // --- Win Condition 1: All Capture Points ---
    if cp_counts.total > 0 {
        for pid in 0..player_count as usize {
            if cp_counts.per_player[pid] == cp_counts.total {
                winner = Some((pid as u8, VictoryReason::AllCapturePoints));
                break;
            }
        }
    }

    // --- Win Condition 2: Majority Hold for 60 seconds ---
    if winner.is_none() && cp_counts.total > 0 {
        let majority_threshold = (cp_counts.total / 2) + 1; // strict majority

        let bs = world.get_resource_mut::<BattleState>().unwrap();
        for pid in 0..player_count as usize {
            if cp_counts.per_player[pid] >= majority_threshold {
                bs.majority_hold_timer[pid] += delta_secs;
                if bs.majority_hold_timer[pid] >= bs.majority_hold_threshold {
                    winner = Some((pid as u8, VictoryReason::MajorityHold));
                    break;
                }
            } else {
                // Reset timer if player loses majority
                bs.majority_hold_timer[pid] = 0.0;
            }
        }
    }

    // --- Win Condition 3: Total Elimination ---
    if winner.is_none() {
        // A player is eliminated if they have 0 combat units AND 0 buildings AND 0 capture points
        let mut alive_players: Vec<u8> = Vec::new();
        for pid in 0..player_count as usize {
            let has_units = alive_units[pid] > 0;
            let has_buildings = alive_buildings[pid] > 0;
            let has_cp = cp_counts.per_player[pid] > 0;

            if has_units || has_buildings || has_cp {
                alive_players.push(pid as u8);
            }
        }

        if alive_players.len() == 1 {
            winner = Some((alive_players[0], VictoryReason::TotalElimination));
        }
    }

    // Apply winner
    if let Some((winner_pid, reason)) = winner {
        let bs = world.get_resource_mut::<BattleState>().unwrap();
        bs.status = BattleStatus::Finished;
        bs.winner = winner_pid;
        bs.victory_reason = Some(reason);

        // Emit BattleEnd event
        let mut payload = [0u8; 16];
        payload[0] = winner_pid;
        payload[1] = reason as u8;
        write_event(world, EventType::BattleEnd, 0, 0.0, 0.0, &payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::CapturePointState;
    use crate::game::{Game, GameConfig};
    use crate::systems::capture::spawn_capture_points;

    fn test_game_with_battle(cp_count: u8) -> Game {
        let mut game = Game::new(GameConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
        });

        spawn_capture_points(&mut game.world, cp_count, 64, 64, 42);
        game.world.insert_resource(BattleState::new(2));
        game
    }

    #[test]
    fn test_no_winner_initially() {
        let mut game = test_game_with_battle(3);

        // Spawn units for both players
        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_thrall(55.5, 55.5, 1);

        // Tick through the minimum wait period
        for _ in 0..250 {
            game.tick(50.0);
        }

        let bs = game.world.get_resource::<BattleState>().unwrap();
        assert!(!bs.is_finished(), "Should not have a winner without capture points owned");
        assert_eq!(bs.winner, 255);
    }

    #[test]
    fn test_no_win_before_min_ticks() {
        let mut game = test_game_with_battle(3);

        // Give player 0 all capture points immediately
        {
            let cp_storage = game.world.get_storage_mut::<CapturePointState>().unwrap();
            for (_entity, cp) in cp_storage.iter_mut() {
                cp.owner = 0;
                cp.progress = 100.0;
            }
        }

        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_thrall(55.5, 55.5, 1);

        // Tick only a few times (less than MIN_TICKS_BEFORE_WIN=200)
        for _ in 0..50 {
            game.tick(50.0);
        }

        let bs = game.world.get_resource::<BattleState>().unwrap();
        assert!(!bs.is_finished(), "Should not win before minimum ticks");
    }

    #[test]
    fn test_all_capture_points_wins_instantly() {
        let mut game = test_game_with_battle(3);

        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_thrall(55.5, 55.5, 1);

        // Give player 0 all capture points
        {
            let cp_storage = game.world.get_storage_mut::<CapturePointState>().unwrap();
            for (_entity, cp) in cp_storage.iter_mut() {
                cp.owner = 0;
                cp.progress = 100.0;
            }
        }

        // Tick past minimum wait
        for _ in 0..250 {
            game.tick(50.0);
        }

        let bs = game.world.get_resource::<BattleState>().unwrap();
        assert!(bs.is_finished(), "Should be finished when one player owns all CPs");
        assert_eq!(bs.winner, 0);
        assert_eq!(bs.victory_reason, Some(VictoryReason::AllCapturePoints));
    }

    #[test]
    fn test_majority_hold_timer_increments() {
        let mut game = test_game_with_battle(3);

        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_thrall(55.5, 55.5, 1);

        // Give player 0 majority (2 of 3 points)
        {
            let cp_storage = game.world.get_storage_mut::<CapturePointState>().unwrap();
            let entities: Vec<_> = cp_storage.iter().map(|(e, _)| e).collect();
            if let Some(cp) = game.world.get_component_mut::<CapturePointState>(entities[0]) {
                cp.owner = 0;
                cp.progress = 100.0;
            }
            if let Some(cp) = game.world.get_component_mut::<CapturePointState>(entities[1]) {
                cp.owner = 0;
                cp.progress = 100.0;
            }
        }

        // Tick past minimum wait
        for _ in 0..250 {
            game.tick(50.0);
        }

        let bs = game.world.get_resource::<BattleState>().unwrap();
        assert!(!bs.is_finished(), "Should not win yet — need 60s hold");
        assert!(bs.majority_hold_timer[0] > 0.0, "Timer should be incrementing");
    }

    #[test]
    fn test_majority_hold_60s_wins() {
        let mut game = test_game_with_battle(3);

        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_thrall(55.5, 55.5, 1);

        // Give player 0 majority (2 of 3)
        {
            let cp_storage = game.world.get_storage_mut::<CapturePointState>().unwrap();
            let entities: Vec<_> = cp_storage.iter().map(|(e, _)| e).collect();
            if let Some(cp) = game.world.get_component_mut::<CapturePointState>(entities[0]) {
                cp.owner = 0;
                cp.progress = 100.0;
            }
            if let Some(cp) = game.world.get_component_mut::<CapturePointState>(entities[1]) {
                cp.owner = 0;
                cp.progress = 100.0;
            }
        }

        // Tick for 200 (min wait) + 1200 (60s at 50ms) = 1400+ ticks
        for _ in 0..1500 {
            game.tick(50.0);
        }

        let bs = game.world.get_resource::<BattleState>().unwrap();
        assert!(bs.is_finished(), "Should win after 60s majority hold");
        assert_eq!(bs.winner, 0);
        assert_eq!(bs.victory_reason, Some(VictoryReason::MajorityHold));
    }

    #[test]
    fn test_majority_timer_resets_on_loss() {
        let mut game = test_game_with_battle(3);

        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_thrall(55.5, 55.5, 1);

        // Give player 0 majority
        {
            let cp_storage = game.world.get_storage_mut::<CapturePointState>().unwrap();
            let entities: Vec<_> = cp_storage.iter().map(|(e, _)| e).collect();
            if let Some(cp) = game.world.get_component_mut::<CapturePointState>(entities[0]) {
                cp.owner = 0;
                cp.progress = 100.0;
            }
            if let Some(cp) = game.world.get_component_mut::<CapturePointState>(entities[1]) {
                cp.owner = 0;
                cp.progress = 100.0;
            }
        }

        // Tick for 30 seconds (past min wait + some majority time)
        for _ in 0..800 {
            game.tick(50.0);
        }

        // Timer should be accumulated
        let timer_before = game.world.get_resource::<BattleState>().unwrap().majority_hold_timer[0];
        assert!(timer_before > 0.0, "Timer should be positive");

        // Now player 0 loses a point (back to 1 of 3, not majority)
        {
            let cp_storage = game.world.get_storage_mut::<CapturePointState>().unwrap();
            let entities: Vec<_> = cp_storage.iter().map(|(e, _)| e).collect();
            if let Some(cp) = game.world.get_component_mut::<CapturePointState>(entities[1]) {
                cp.owner = 1;
            }
        }

        // Tick a few more times
        for _ in 0..10 {
            game.tick(50.0);
        }

        let timer_after = game.world.get_resource::<BattleState>().unwrap().majority_hold_timer[0];
        assert_eq!(timer_after, 0.0, "Timer should reset when majority is lost");
    }

    #[test]
    fn test_elimination_wins() {
        let mut game = test_game_with_battle(3);

        // Player 0 has a unit
        game.spawn_thrall(8.5, 8.5, 0);
        // Player 1 has a unit that we'll kill
        let unit1 = game.spawn_thrall(55.5, 55.5, 1);

        // Kill player 1's unit
        if let Some(h) = game.world.get_component_mut::<Health>(unit1) {
            h.current = 0.0;
        }

        // Tick past minimum wait (+ death cleanup)
        for _ in 0..250 {
            game.tick(50.0);
        }

        let bs = game.world.get_resource::<BattleState>().unwrap();
        assert!(bs.is_finished(), "Should win by elimination");
        assert_eq!(bs.winner, 0);
        assert_eq!(bs.victory_reason, Some(VictoryReason::TotalElimination));
    }

    #[test]
    fn test_battle_end_event_emitted() {
        let mut game = test_game_with_battle(3);

        game.spawn_thrall(8.5, 8.5, 0);
        let unit1 = game.spawn_thrall(55.5, 55.5, 1);

        // Kill player 1
        if let Some(h) = game.world.get_component_mut::<Health>(unit1) {
            h.current = 0.0;
        }

        let mut found_battle_end = false;
        for _ in 0..300 {
            game.tick(50.0);

            let ec = game.world.get_resource::<crate::game::EventCount>().unwrap().0;
            let eb = &game.world.get_resource::<crate::game::EventBuffer>().unwrap().0;
            for i in 0..ec as usize {
                let off = i * crate::game::EVENT_ENTRY_SIZE;
                let event_type = u16::from_le_bytes([eb[off], eb[off + 1]]);
                if event_type == EventType::BattleEnd as u16 {
                    found_battle_end = true;
                    // Check payload
                    let winner_pid = eb[off + 16]; // payload[0]
                    assert_eq!(winner_pid, 0, "Winner should be player 0");
                    break;
                }
            }
            if found_battle_end {
                break;
            }
        }

        assert!(found_battle_end, "Should emit BattleEnd event");
    }

    #[test]
    fn test_no_double_win() {
        let mut game = test_game_with_battle(3);

        game.spawn_thrall(8.5, 8.5, 0);
        let unit1 = game.spawn_thrall(55.5, 55.5, 1);

        if let Some(h) = game.world.get_component_mut::<Health>(unit1) {
            h.current = 0.0;
        }

        // Tick until win
        for _ in 0..300 {
            game.tick(50.0);
        }

        let bs = game.world.get_resource::<BattleState>().unwrap();
        assert!(bs.is_finished());
        let winner = bs.winner;

        // Tick more — should stay finished with same winner
        for _ in 0..100 {
            game.tick(50.0);
        }

        let bs = game.world.get_resource::<BattleState>().unwrap();
        assert!(bs.is_finished());
        assert_eq!(bs.winner, winner, "Winner should not change after battle ends");
    }
}

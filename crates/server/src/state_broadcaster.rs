use machine_empire_core::ecs::World;
use machine_empire_core::protocol::{EntitySnapshot, ServerMessage};
use machine_empire_core::state_snapshot;

/// Per-player state broadcaster. Handles fog-filtered state extraction and
/// building ServerMessage::State for each player every tick.
pub struct StateBroadcaster {
    /// Number of players in the match.
    player_count: u8,
    /// Previous entity snapshot per player (for future delta compression).
    previous_entities: Vec<Vec<EntitySnapshot>>,
}

impl StateBroadcaster {
    /// Create a new StateBroadcaster for the given number of players.
    pub fn new(player_count: u8) -> Self {
        let previous_entities = (0..player_count)
            .map(|_| Vec::new())
            .collect();
        StateBroadcaster {
            player_count,
            previous_entities,
        }
    }

    /// Build per-player fog-filtered State messages for the current tick.
    /// Returns a Vec of (player_id, ServerMessage::State) tuples.
    pub fn broadcast(&mut self, world: &World, tick: u32) -> Vec<(u8, ServerMessage)> {
        let mut messages = Vec::with_capacity(self.player_count as usize);

        for pid in 0..self.player_count {
            let entities = state_snapshot::snapshot_entities_for_player(world, pid);
            let events = state_snapshot::snapshot_events_for_player(world, pid);
            let fog = state_snapshot::snapshot_fog(world, pid);
            let economy = state_snapshot::snapshot_economy(world, pid);
            let production = state_snapshot::snapshot_production(world, pid);

            // Store entities for future delta compression
            self.previous_entities[pid as usize] = entities.clone();

            let capture_points = state_snapshot::snapshot_capture_points(world);

            let msg = ServerMessage::State {
                tick,
                entities,
                events,
                fog,
                economy,
                production,
                capture_points,
            };

            messages.push((pid, msg));
        }

        messages
    }

    /// Build a full state message for a specific player (on join/reconnect).
    /// Includes map tiles and other static data not sent every tick.
    pub fn full_state(&self, world: &World, player_id: u8, tick: u32, map_width: u32, map_height: u32) -> ServerMessage {
        let entities = state_snapshot::snapshot_entities_for_player(world, player_id);
        let fog = state_snapshot::snapshot_fog(world, player_id);
        let economy = state_snapshot::snapshot_economy(world, player_id);
        let production = state_snapshot::snapshot_production(world, player_id);
        let map_tiles = state_snapshot::snapshot_map_tiles(world);
        let capture_points = state_snapshot::snapshot_capture_points(world);

        ServerMessage::FullState {
            tick,
            entities,
            fog,
            economy,
            production,
            capture_points,
            map_width,
            map_height,
            map_tiles,
        }
    }

    /// Get the previous entity snapshot for a specific player.
    /// Useful for delta compression or debugging.
    #[allow(dead_code)]
    pub fn previous_entities(&self, player_id: u8) -> &[EntitySnapshot] {
        if (player_id as usize) < self.previous_entities.len() {
            &self.previous_entities[player_id as usize]
        } else {
            &[]
        }
    }

    /// Get the player count.
    #[allow(dead_code)]
    pub fn player_count(&self) -> u8 {
        self.player_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use machine_empire_core::game::{Game, GameConfig};
    use machine_empire_core::command::Command;

    fn test_game() -> Game {
        Game::new(GameConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
        })
    }

    #[test]
    fn test_broadcaster_creation() {
        let broadcaster = StateBroadcaster::new(2);
        assert_eq!(broadcaster.player_count(), 2);
        assert_eq!(broadcaster.previous_entities(0).len(), 0);
        assert_eq!(broadcaster.previous_entities(1).len(), 0);
    }

    #[test]
    fn test_broadcast_per_player_state() {
        let mut game = test_game();

        // Spawn units for both players in separate corners
        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_thrall(8.5, 9.5, 0);
        game.spawn_thrall(55.5, 55.5, 1);

        game.tick(50.0);

        let mut broadcaster = StateBroadcaster::new(2);
        let messages = broadcaster.broadcast(&game.world, 1);

        assert_eq!(messages.len(), 2, "Should have one message per player");

        // Check player 0's message
        let (p0_id, p0_msg) = &messages[0];
        assert_eq!(*p0_id, 0);
        match p0_msg {
            ServerMessage::State { tick, entities, fog, economy, production, .. } => {
                assert_eq!(*tick, 1);
                // Player 0 should see own 2 units, not player 1's unit (too far)
                assert_eq!(entities.len(), 2, "Player 0 should see 2 own units");
                assert!(entities.iter().all(|e| e.owner == 0));
                assert!(!fog.is_empty());
                assert!(economy.energy_bank > 0.0);
                assert!(!production.is_empty());
            }
            _ => panic!("Expected State message"),
        }

        // Check player 1's message
        let (p1_id, p1_msg) = &messages[1];
        assert_eq!(*p1_id, 1);
        match p1_msg {
            ServerMessage::State { entities, .. } => {
                // Player 1 should see own 1 unit, not player 0's units (too far)
                assert_eq!(entities.len(), 1, "Player 1 should see 1 own unit");
                assert_eq!(entities[0].owner, 1);
            }
            _ => panic!("Expected State message"),
        }
    }

    #[test]
    fn test_broadcast_stores_previous_entities() {
        let mut game = test_game();
        game.spawn_thrall(8.5, 8.5, 0);
        game.tick(50.0);

        let mut broadcaster = StateBroadcaster::new(2);
        let _messages = broadcaster.broadcast(&game.world, 1);

        // Previous entities should be stored
        assert_eq!(broadcaster.previous_entities(0).len(), 1);
        assert_eq!(broadcaster.previous_entities(1).len(), 0);
    }

    #[test]
    fn test_full_state_includes_map() {
        let mut game = test_game();
        game.spawn_thrall(8.5, 8.5, 0);
        game.tick(50.0);

        let broadcaster = StateBroadcaster::new(2);
        let msg = broadcaster.full_state(&game.world, 0, 1, 64, 64);

        match msg {
            ServerMessage::FullState {
                tick, entities, fog, economy, production, map_width, map_height, map_tiles, ..
            } => {
                assert_eq!(tick, 1);
                assert!(!entities.is_empty());
                assert_eq!(fog.len(), 64 * 64);
                assert!(economy.energy_bank > 0.0);
                assert!(!production.is_empty());
                assert_eq!(map_width, 64);
                assert_eq!(map_height, 64);
                assert_eq!(map_tiles.len(), 64 * 64);
            }
            _ => panic!("Expected FullState message"),
        }
    }

    #[test]
    fn test_own_units_always_included() {
        let mut game = test_game();

        // Both players have units at their corners
        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_thrall(9.5, 8.5, 0);
        game.spawn_thrall(10.5, 8.5, 0);
        game.spawn_thrall(55.5, 55.5, 1);
        game.spawn_thrall(56.5, 55.5, 1);

        game.tick(50.0);

        let mut broadcaster = StateBroadcaster::new(2);
        let messages = broadcaster.broadcast(&game.world, 1);

        // Player 0: should see all 3 own units
        match &messages[0].1 {
            ServerMessage::State { entities, .. } => {
                let own = entities.iter().filter(|e| e.owner == 0).count();
                assert_eq!(own, 3, "Player 0 should see all 3 own units");
            }
            _ => panic!("Expected State"),
        }

        // Player 1: should see all 2 own units
        match &messages[1].1 {
            ServerMessage::State { entities, .. } => {
                let own = entities.iter().filter(|e| e.owner == 1).count();
                assert_eq!(own, 2, "Player 1 should see all 2 own units");
            }
            _ => panic!("Expected State"),
        }
    }

    #[test]
    fn test_fog_filtered_entities_visible() {
        let mut game = test_game();

        // Player 0 and player 1 units close together — should be mutually visible
        game.spawn_thrall(30.5, 30.5, 0);
        game.spawn_thrall(33.5, 30.5, 1);

        game.tick(50.0);

        let mut broadcaster = StateBroadcaster::new(2);
        let messages = broadcaster.broadcast(&game.world, 1);

        // Player 0: should see own + enemy (within vision range)
        match &messages[0].1 {
            ServerMessage::State { entities, .. } => {
                assert_eq!(entities.len(), 2, "Player 0 should see both units (close together)");
            }
            _ => panic!("Expected State"),
        }

        // Player 1: should see own + enemy
        match &messages[1].1 {
            ServerMessage::State { entities, .. } => {
                assert_eq!(entities.len(), 2, "Player 1 should see both units (close together)");
            }
            _ => panic!("Expected State"),
        }
    }

    #[test]
    fn test_economy_snapshot_per_player() {
        let mut game = test_game();
        game.tick(50.0);

        let mut broadcaster = StateBroadcaster::new(2);
        let messages = broadcaster.broadcast(&game.world, 1);

        // Both players should have valid economy data
        for (pid, msg) in &messages {
            match msg {
                ServerMessage::State { economy, .. } => {
                    assert!(economy.energy_bank > 0.0,
                        "Player {} should have positive energy", pid);
                    assert!(economy.income > 0.0,
                        "Player {} should have positive income", pid);
                }
                _ => panic!("Expected State"),
            }
        }
    }

    #[test]
    fn test_event_snapshot_fog_filtered() {
        let mut game = test_game();

        // Player 0 attacks player 1 target near each other (visible to both)
        let attacker = game.spawn_thrall(30.5, 30.5, 0);
        let target = game.spawn_thrall(33.5, 30.5, 1);

        game.push_command(Command::Attack {
            unit_ids: vec![attacker.raw()],
            target_id: target.raw(),
        });

        game.tick(50.0);

        let mut broadcaster = StateBroadcaster::new(2);
        let messages = broadcaster.broadcast(&game.world, 1);

        // Player 0 (attacker) should see events
        match &messages[0].1 {
            ServerMessage::State { events, .. } => {
                assert!(!events.is_empty(), "Player 0 should see combat events");
            }
            _ => panic!("Expected State"),
        }

        // Player 1 (target is nearby, so the fight is visible)
        match &messages[1].1 {
            ServerMessage::State { events, .. } => {
                assert!(!events.is_empty(), "Player 1 should see combat events in visible area");
            }
            _ => panic!("Expected State"),
        }
    }
}

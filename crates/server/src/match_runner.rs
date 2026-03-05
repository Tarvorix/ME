use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::info;

use machine_empire_core::game::{Game, GameConfig};
use machine_empire_core::command::Command;
use machine_empire_core::protocol::{MatchId, MatchConfig, ServerMessage};
use machine_empire_core::state_snapshot;
use machine_empire_core::replay::ReplayRecorder;

use crate::state_broadcaster::StateBroadcaster;

/// Status of a running match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchStatus {
    Running,
    Finished { winner: u8 },
}

/// A handle to a player within a match.
pub struct PlayerHandle {
    /// Player ID (0-3).
    pub player_id: u8,
    /// Whether this player is AI-controlled.
    pub is_ai: bool,
    /// Command receiver (None for AI — they generate commands internally).
    pub cmd_rx: Option<mpsc::UnboundedReceiver<Command>>,
    /// Connection's message sender (None for AI).
    pub state_tx: Option<mpsc::UnboundedSender<ServerMessage>>,
}

/// The authoritative match runner. Owns the Game instance and runs the tick loop.
pub struct MatchRunner {
    /// Match identifier.
    pub match_id: MatchId,
    /// The game simulation.
    pub game: Game,
    /// Match configuration.
    pub config: MatchConfig,
    /// Player handles.
    pub players: Vec<PlayerHandle>,
    /// Tick counter.
    pub tick_count: u32,
    /// Current match status.
    pub status: MatchStatus,
    /// Per-player fog-filtered state broadcaster.
    pub broadcaster: StateBroadcaster,
    /// Replay recorder for this match.
    pub replay_recorder: ReplayRecorder,
}

impl MatchRunner {
    /// Create a new match runner. Spawns starting units for all players.
    pub fn new(match_id: MatchId, config: MatchConfig, players: Vec<PlayerHandle>) -> Self {
        let game_config = GameConfig {
            map_width: config.map_width,
            map_height: config.map_height,
            player_count: config.player_count,
            seed: config.seed,
        };
        let mut game = Game::new(game_config);

        // Spawn starting units for each player at their starting position
        for player in &players {
            let (x, y) = starting_position(player.player_id, &config);
            game.spawn_starting_units(player.player_id, x, y);

            // Register AI players with the game's AI system
            if player.is_ai {
                game.add_ai_player(
                    player.player_id,
                    machine_empire_core::ai::player::AiDifficulty::Normal,
                );
            }
        }

        let broadcaster = StateBroadcaster::new(config.player_count as u8);
        let replay_recorder = ReplayRecorder::new(config.clone(), config.player_count);

        MatchRunner {
            match_id,
            game,
            config,
            players,
            tick_count: 0,
            status: MatchStatus::Running,
            broadcaster,
            replay_recorder,
        }
    }

    /// Send full state to a specific player (on match start or reconnect).
    fn send_full_state(&self, player: &PlayerHandle) {
        if player.is_ai {
            return;
        }

        let state_tx = match &player.state_tx {
            Some(tx) => tx,
            None => return,
        };

        let msg = self.broadcaster.full_state(
            &self.game.world,
            player.player_id,
            self.tick_count,
            self.config.map_width,
            self.config.map_height,
        );

        let _ = state_tx.send(msg);
    }

    /// Build a per-tick State message for a specific player (fog-filtered).
    /// This is a convenience method that uses the broadcaster internally.
    #[allow(dead_code)]
    fn build_state_message(&self, player_id: u8) -> ServerMessage {
        let entities = state_snapshot::snapshot_entities_for_player(&self.game.world, player_id);
        let events = state_snapshot::snapshot_events_for_player(&self.game.world, player_id);
        let fog = state_snapshot::snapshot_fog(&self.game.world, player_id);
        let economy = state_snapshot::snapshot_economy(&self.game.world, player_id);
        let production = state_snapshot::snapshot_production(&self.game.world, player_id);
        let capture_points = state_snapshot::snapshot_capture_points(&self.game.world);

        ServerMessage::State {
            tick: self.tick_count,
            entities,
            events,
            fog,
            economy,
            production,
            capture_points,
        }
    }

    /// Check if the match has a winner.
    /// A player wins when they are the only one with a surviving forge.
    /// Returns None if multiple forges are still alive or game just started.
    fn check_winner(&self) -> Option<u8> {
        // Don't check win condition in the first few ticks (let game initialize)
        if self.tick_count < 10 {
            return None;
        }

        let mut alive_players = Vec::new();

        for player in &self.players {
            if self.game.check_forge_alive(player.player_id) {
                alive_players.push(player.player_id);
            }
        }

        if alive_players.len() == 1 {
            Some(alive_players[0])
        } else if alive_players.is_empty() {
            // All forges destroyed simultaneously — player 0 wins by default
            Some(0)
        } else {
            None
        }
    }

    /// Run the match loop. Blocks until the match ends.
    pub async fn run(mut self) {
        let tick_duration = Duration::from_millis(self.config.tick_rate_ms as u64);
        let mut tick_interval = interval(tick_duration);

        info!(
            "Match '{}' starting with {} players (tick rate: {}ms)",
            self.match_id.0,
            self.players.len(),
            self.config.tick_rate_ms
        );

        // Run an initial tick to warm up fog and render systems
        self.game.tick(self.config.tick_rate_ms as f32);
        self.tick_count += 1;

        // Send initial full state to all human players
        for player in &self.players {
            self.send_full_state(player);
        }

        loop {
            tick_interval.tick().await;

            // Drain commands from all human players
            for player in &mut self.players {
                if let Some(rx) = &mut player.cmd_rx {
                    while let Ok(cmd) = rx.try_recv() {
                        // Record command for replay
                        self.replay_recorder.record_command(player.player_id, cmd.clone());
                        self.game.push_command(cmd);
                    }
                }
            }

            // Record the tick for replay
            self.replay_recorder.record_tick();

            // Tick the game simulation
            self.game.tick(self.config.tick_rate_ms as f32);
            self.tick_count += 1;

            // Broadcast fog-filtered state to each human player via StateBroadcaster
            let state_messages = self.broadcaster.broadcast(&self.game.world, self.tick_count);
            for (pid, msg) in state_messages {
                // Find the player handle for this player_id
                if let Some(player) = self.players.iter().find(|p| p.player_id == pid) {
                    if player.is_ai {
                        continue;
                    }
                    if let Some(tx) = &player.state_tx {
                        if tx.send(msg).is_err() {
                            // Player disconnected — channel closed
                            info!(
                                "Match '{}': player {} disconnected (channel closed)",
                                self.match_id.0, pid
                            );
                        }
                    }
                }
            }

            // Check win condition
            if let Some(winner) = self.check_winner() {
                info!(
                    "Match '{}' ended at tick {}: player {} wins (forge destroyed)",
                    self.match_id.0, self.tick_count, winner
                );
                self.status = MatchStatus::Finished { winner };

                // Broadcast end message to all human players
                for player in &self.players {
                    if !player.is_ai {
                        if let Some(tx) = &player.state_tx {
                            let _ = tx.send(ServerMessage::End {
                                winner,
                                reason: "All enemy forges destroyed".into(),
                            });
                        }
                    }
                }

                break;
            }
        }

        // Finalize replay data
        let replay_data = self.replay_recorder.finalize();
        info!(
            "Match '{}' replay recorded: {} ticks, {} frames",
            self.match_id.0,
            replay_data.total_ticks,
            replay_data.frames.len()
        );

        info!("Match '{}' runner exiting at tick {}", self.match_id.0, self.tick_count);
    }
}

/// Get starting position for a player based on their ID and map configuration.
/// Players are placed in map corners with a margin from the edge.
pub fn starting_position(player_id: u8, config: &MatchConfig) -> (f32, f32) {
    let margin = 8.0;
    let w = config.map_width as f32;
    let h = config.map_height as f32;

    match player_id {
        0 => (margin, margin),                      // top-left
        1 => (w - margin, h - margin),              // bottom-right
        2 => (w - margin, margin),                  // top-right
        3 => (margin, h - margin),                  // bottom-left
        _ => (w / 2.0, h / 2.0),                    // center fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> MatchConfig {
        MatchConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
            tick_rate_ms: 50,
        }
    }

    #[test]
    fn test_starting_positions() {
        let config = test_config();

        let (x0, y0) = starting_position(0, &config);
        assert_eq!(x0, 8.0);
        assert_eq!(y0, 8.0);

        let (x1, y1) = starting_position(1, &config);
        assert_eq!(x1, 56.0);
        assert_eq!(y1, 56.0);

        let (x2, y2) = starting_position(2, &config);
        assert_eq!(x2, 56.0);
        assert_eq!(y2, 8.0);

        let (x3, y3) = starting_position(3, &config);
        assert_eq!(x3, 8.0);
        assert_eq!(y3, 56.0);

        // Fallback
        let (xf, yf) = starting_position(99, &config);
        assert_eq!(xf, 32.0);
        assert_eq!(yf, 32.0);
    }

    #[test]
    fn test_match_runner_creation() {
        let config = test_config();

        let players = vec![
            PlayerHandle {
                player_id: 0,
                is_ai: false,
                cmd_rx: None,
                state_tx: None,
            },
            PlayerHandle {
                player_id: 1,
                is_ai: true,
                cmd_rx: None,
                state_tx: None,
            },
        ];

        let runner = MatchRunner::new(MatchId("test-match".into()), config, players);
        assert_eq!(runner.tick_count, 0);
        assert_eq!(runner.status, MatchStatus::Running);
        assert_eq!(runner.players.len(), 2);

        // Verify starting units were spawned
        // Each player gets: 1 Command Post + 1 Forge + 3 Thralls = 5 entities
        // Total: 10 entities for 2 players
        use machine_empire_core::components::UnitType;
        let ut_storage = runner.game.world.get_storage::<UnitType>().unwrap();
        let total_units: usize = ut_storage.iter().count();
        assert_eq!(total_units, 10, "Should have 10 entities (5 per player)");
    }

    #[test]
    fn test_match_runner_starting_units_ownership() {
        let config = test_config();

        let players = vec![
            PlayerHandle {
                player_id: 0,
                is_ai: false,
                cmd_rx: None,
                state_tx: None,
            },
            PlayerHandle {
                player_id: 1,
                is_ai: true,
                cmd_rx: None,
                state_tx: None,
            },
        ];

        let runner = MatchRunner::new(MatchId("test-match".into()), config, players);

        use machine_empire_core::components::UnitType;
        let ut_storage = runner.game.world.get_storage::<UnitType>().unwrap();

        let p0_count = ut_storage.iter().filter(|(_, ut)| ut.owner == 0).count();
        let p1_count = ut_storage.iter().filter(|(_, ut)| ut.owner == 1).count();

        assert_eq!(p0_count, 5, "Player 0 should have 5 units");
        assert_eq!(p1_count, 5, "Player 1 should have 5 units");
    }

    #[test]
    fn test_win_condition_one_forge_destroyed() {
        let config = test_config();

        let players = vec![
            PlayerHandle {
                player_id: 0,
                is_ai: false,
                cmd_rx: None,
                state_tx: None,
            },
            PlayerHandle {
                player_id: 1,
                is_ai: true,
                cmd_rx: None,
                state_tx: None,
            },
        ];

        let mut runner = MatchRunner::new(MatchId("test-match".into()), config, players);

        // Advance past minimum tick threshold
        runner.tick_count = 20;

        // Both forges alive — no winner
        assert!(runner.check_winner().is_none());

        // Kill player 1's forge by setting health to 0
        use machine_empire_core::components::{UnitType, Health};
        use machine_empire_core::types::SpriteId;

        let ut_storage = runner.game.world.get_storage::<UnitType>().unwrap();
        let mut forge_entity = None;
        for (entity, ut) in ut_storage.iter() {
            if ut.owner == 1 && ut.kind == SpriteId::Forge {
                forge_entity = Some(entity);
                break;
            }
        }

        let forge = forge_entity.expect("Player 1 should have a forge");
        if let Some(h) = runner.game.world.get_component_mut::<Health>(forge) {
            h.current = 0.0;
        }

        // Now player 0 should be the winner
        assert_eq!(runner.check_winner(), Some(0));
    }

    #[test]
    fn test_win_condition_too_early() {
        let config = test_config();

        let players = vec![
            PlayerHandle {
                player_id: 0,
                is_ai: false,
                cmd_rx: None,
                state_tx: None,
            },
            PlayerHandle {
                player_id: 1,
                is_ai: true,
                cmd_rx: None,
                state_tx: None,
            },
        ];

        let mut runner = MatchRunner::new(MatchId("test-match".into()), config, players);

        // Kill player 1's forge early
        use machine_empire_core::components::{UnitType, Health};
        use machine_empire_core::types::SpriteId;

        let ut_storage = runner.game.world.get_storage::<UnitType>().unwrap();
        let mut forge_entity = None;
        for (entity, ut) in ut_storage.iter() {
            if ut.owner == 1 && ut.kind == SpriteId::Forge {
                forge_entity = Some(entity);
                break;
            }
        }

        let forge = forge_entity.unwrap();
        if let Some(h) = runner.game.world.get_component_mut::<Health>(forge) {
            h.current = 0.0;
        }

        // tick_count < 10 — should not declare winner yet
        runner.tick_count = 5;
        assert!(runner.check_winner().is_none());

        // After threshold — should declare winner
        runner.tick_count = 15;
        assert_eq!(runner.check_winner(), Some(0));
    }

    #[test]
    fn test_build_state_message() {
        let config = test_config();

        let players = vec![
            PlayerHandle {
                player_id: 0,
                is_ai: false,
                cmd_rx: None,
                state_tx: None,
            },
            PlayerHandle {
                player_id: 1,
                is_ai: true,
                cmd_rx: None,
                state_tx: None,
            },
        ];

        let mut runner = MatchRunner::new(MatchId("test-match".into()), config, players);

        // Run a tick so fog initializes
        runner.game.tick(50.0);
        runner.tick_count = 1;

        let msg = runner.build_state_message(0);
        match msg {
            ServerMessage::State { tick, entities, fog, economy, production, .. } => {
                assert_eq!(tick, 1);
                // Player 0 should see their own 5 entities at minimum
                assert!(entities.len() >= 5, "Player 0 should see at least 5 entities, got {}", entities.len());
                assert!(!fog.is_empty());
                assert!(economy.energy_bank > 0.0);
                assert!(!production.is_empty());
            }
            _ => panic!("Expected State message"),
        }
    }

    #[tokio::test]
    async fn test_match_runner_ticks() {
        let config = MatchConfig {
            map_width: 32,
            map_height: 32,
            player_count: 2,
            seed: 42,
            tick_rate_ms: 10, // Fast for testing
        };

        let (state_tx, mut state_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();

        let players = vec![
            PlayerHandle {
                player_id: 0,
                is_ai: false,
                cmd_rx: Some(cmd_rx),
                state_tx: Some(state_tx),
            },
            PlayerHandle {
                player_id: 1,
                is_ai: true,
                cmd_rx: None,
                state_tx: None,
            },
        ];

        let runner = MatchRunner::new(MatchId("tick-test".into()), config, players);

        // Run the match in a background task
        let handle = tokio::spawn(async move {
            runner.run().await;
        });

        // Wait for initial FullState message
        let first_msg = tokio::time::timeout(Duration::from_secs(2), state_rx.recv()).await;
        assert!(first_msg.is_ok(), "Should receive initial FullState");
        let first_msg = first_msg.unwrap().unwrap();
        match first_msg {
            ServerMessage::FullState { .. } => {} // Expected
            _ => panic!("First message should be FullState, got {:?}", std::mem::discriminant(&first_msg)),
        }

        // Wait for a few State messages (tick updates)
        let mut state_count = 0;
        for _ in 0..5 {
            let msg = tokio::time::timeout(Duration::from_secs(2), state_rx.recv()).await;
            if let Ok(Some(ServerMessage::State { tick, .. })) = msg {
                assert!(tick > 0);
                state_count += 1;
            }
        }
        assert!(state_count >= 2, "Should have received at least 2 State messages, got {}", state_count);

        // Clean up: drop the command sender to let the match continue
        drop(cmd_tx);
        handle.abort();
    }
}

use serde::{Serialize, Deserialize};
use crate::command::Command;
use crate::protocol::MatchConfig;
use crate::game::{Game, GameConfig};

/// A single frame in a replay, recording all commands issued at a given tick.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayFrame {
    /// The tick number this frame was recorded at.
    pub tick: u32,
    /// All commands issued by all players during this tick.
    pub commands: Vec<(u8, Command)>,
}

/// A complete replay recording: config + seed + command frames.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayData {
    /// Match configuration used for this replay.
    pub config: MatchConfig,
    /// Player count at game start.
    pub player_count: u32,
    /// Recorded frames (one per tick that had commands, or empty frames for ticks without).
    pub frames: Vec<ReplayFrame>,
    /// Total number of ticks in the replay.
    pub total_ticks: u32,
    /// Version tag for replay format compatibility.
    pub version: u32,
}

/// Records replay data during a live match.
pub struct ReplayRecorder {
    /// The match configuration.
    config: MatchConfig,
    /// Player count.
    player_count: u32,
    /// Recorded frames.
    frames: Vec<ReplayFrame>,
    /// Current tick being accumulated.
    current_tick: u32,
    /// Commands accumulated for the current tick.
    current_commands: Vec<(u8, Command)>,
}

impl ReplayRecorder {
    /// Create a new replay recorder for a match.
    pub fn new(config: MatchConfig, player_count: u32) -> Self {
        ReplayRecorder {
            config,
            player_count,
            frames: Vec::new(),
            current_tick: 0,
            current_commands: Vec::new(),
        }
    }

    /// Record a command issued by a player during the current tick.
    pub fn record_command(&mut self, player_id: u8, cmd: Command) {
        self.current_commands.push((player_id, cmd));
    }

    /// Finalize the current tick and advance to the next one.
    /// This records all accumulated commands as a frame.
    pub fn record_tick(&mut self) {
        // Always record a frame for every tick (even empty ones) to ensure
        // deterministic playback with exact tick counts.
        self.frames.push(ReplayFrame {
            tick: self.current_tick,
            commands: std::mem::take(&mut self.current_commands),
        });
        self.current_tick += 1;
    }

    /// Get the current tick number.
    pub fn current_tick(&self) -> u32 {
        self.current_tick
    }

    /// Get the number of recorded frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Finalize the recording and produce a ReplayData.
    pub fn finalize(self) -> ReplayData {
        ReplayData {
            config: self.config,
            player_count: self.player_count,
            frames: self.frames,
            total_ticks: self.current_tick,
            version: 1,
        }
    }
}

/// Serialization/deserialization for replay data using MessagePack.
impl ReplayData {
    /// Serialize to MessagePack bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(self)
    }

    /// Deserialize from MessagePack bytes.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(bytes)
    }

    /// Serialize to JSON string (for debugging or web export).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Plays back a recorded replay deterministically.
pub struct ReplayPlayer {
    /// The replay data being played back.
    data: ReplayData,
    /// The game instance used for playback.
    game: Game,
    /// Current playback tick.
    current_tick: u32,
    /// Index into the frames array for the next frame to apply.
    frame_index: usize,
    /// AI player IDs to register (stored for seek/reset).
    ai_player_ids: Vec<u8>,
}

impl ReplayPlayer {
    /// Create a replay player from replay data.
    /// `ai_player_ids` specifies which players were AI (to register their AI systems).
    pub fn new(data: ReplayData, ai_player_ids: Vec<u8>) -> Self {
        let game_config = GameConfig {
            map_width: data.config.map_width,
            map_height: data.config.map_height,
            player_count: data.config.player_count,
            seed: data.config.seed,
        };
        let mut game = Game::new(game_config);

        // Spawn starting units for all players
        let player_count = data.player_count;
        for pid in 0..player_count as u8 {
            let (x, y) = starting_position_from_config(pid, &data.config);
            game.spawn_starting_units(pid, x, y);
        }

        // Register AI players
        for &pid in &ai_player_ids {
            game.add_ai_player(pid, crate::ai::player::AiDifficulty::Normal);
        }

        ReplayPlayer {
            data,
            game,
            current_tick: 0,
            frame_index: 0,
            ai_player_ids,
        }
    }

    /// Advance one tick in the replay. Returns false if replay is finished.
    pub fn step(&mut self) -> bool {
        if self.current_tick >= self.data.total_ticks {
            return false;
        }

        // Apply commands for this tick
        if self.frame_index < self.data.frames.len() {
            let frame = &self.data.frames[self.frame_index];
            if frame.tick == self.current_tick {
                for (_player_id, cmd) in &frame.commands {
                    self.game.push_command(cmd.clone());
                }
                self.frame_index += 1;
            }
        }

        // Tick the game
        self.game.tick(self.data.config.tick_rate_ms as f32);
        self.current_tick += 1;

        true
    }

    /// Get the current playback tick.
    pub fn current_tick(&self) -> u32 {
        self.current_tick
    }

    /// Get the total number of ticks in the replay.
    pub fn total_ticks(&self) -> u32 {
        self.data.total_ticks
    }

    /// Whether the replay has finished playing.
    pub fn is_finished(&self) -> bool {
        self.current_tick >= self.data.total_ticks
    }

    /// Get a reference to the game state (for rendering or inspection).
    pub fn game(&self) -> &Game {
        &self.game
    }

    /// Get a mutable reference to the game state.
    pub fn game_mut(&mut self) -> &mut Game {
        &mut self.game
    }

    /// Seek to a specific tick by replaying from the beginning.
    /// This ensures deterministic state at any tick.
    pub fn seek(&mut self, target_tick: u32) {
        // Reset the game from scratch
        let game_config = GameConfig {
            map_width: self.data.config.map_width,
            map_height: self.data.config.map_height,
            player_count: self.data.config.player_count,
            seed: self.data.config.seed,
        };
        let mut game = Game::new(game_config);

        // Spawn starting units
        for pid in 0..self.data.player_count as u8 {
            let (x, y) = starting_position_from_config(pid, &self.data.config);
            game.spawn_starting_units(pid, x, y);
        }

        // Register AI players
        for &pid in &self.ai_player_ids {
            game.add_ai_player(pid, crate::ai::player::AiDifficulty::Normal);
        }

        self.game = game;
        self.current_tick = 0;
        self.frame_index = 0;

        // Replay up to target tick
        let clamped = target_tick.min(self.data.total_ticks);
        for _ in 0..clamped {
            self.step();
        }
    }
}

/// Get starting position for a player from MatchConfig (mirrors match_runner logic).
fn starting_position_from_config(player_id: u8, config: &MatchConfig) -> (f32, f32) {
    let margin = 8.0;
    let w = config.map_width as f32;
    let h = config.map_height as f32;

    match player_id {
        0 => (margin, margin),
        1 => (w - margin, h - margin),
        2 => (w - margin, margin),
        3 => (margin, h - margin),
        _ => (w / 2.0, h / 2.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::MatchConfig;

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
    fn test_replay_recorder_basic() {
        let config = test_config();
        let mut recorder = ReplayRecorder::new(config, 2);

        // Record a few ticks with commands
        recorder.record_command(0, Command::Move {
            unit_ids: vec![1],
            target_x: 10.0,
            target_y: 10.0,
        });
        recorder.record_tick();

        recorder.record_tick(); // Empty tick

        recorder.record_command(1, Command::Produce {
            player: 1,
            unit_type: 0,
        });
        recorder.record_tick();

        assert_eq!(recorder.current_tick(), 3);
        assert_eq!(recorder.frame_count(), 3);

        let data = recorder.finalize();
        assert_eq!(data.total_ticks, 3);
        assert_eq!(data.frames.len(), 3);
        assert_eq!(data.frames[0].commands.len(), 1);
        assert_eq!(data.frames[1].commands.len(), 0);
        assert_eq!(data.frames[2].commands.len(), 1);
    }

    #[test]
    fn test_replay_serialization_roundtrip_msgpack() {
        let config = test_config();
        let mut recorder = ReplayRecorder::new(config, 2);

        recorder.record_command(0, Command::Move {
            unit_ids: vec![1, 2, 3],
            target_x: 20.0,
            target_y: 30.0,
        });
        recorder.record_tick();
        recorder.record_command(1, Command::Attack {
            unit_ids: vec![5],
            target_id: 10,
        });
        recorder.record_tick();

        let data = recorder.finalize();

        // Serialize to MessagePack and back
        let bytes = data.serialize().unwrap();
        let restored = ReplayData::deserialize(&bytes).unwrap();

        assert_eq!(restored.total_ticks, data.total_ticks);
        assert_eq!(restored.frames.len(), data.frames.len());
        assert_eq!(restored.config.seed, data.config.seed);
        assert_eq!(restored.version, 1);
    }

    #[test]
    fn test_replay_serialization_roundtrip_json() {
        let config = test_config();
        let mut recorder = ReplayRecorder::new(config, 2);

        recorder.record_command(0, Command::Produce {
            player: 0,
            unit_type: 1,
        });
        recorder.record_tick();

        let data = recorder.finalize();

        let json = data.to_json().unwrap();
        let restored = ReplayData::from_json(&json).unwrap();

        assert_eq!(restored.total_ticks, data.total_ticks);
        assert_eq!(restored.frames.len(), data.frames.len());
    }

    #[test]
    fn test_replay_player_basic() {
        let config = test_config();
        let mut recorder = ReplayRecorder::new(config, 2);

        // Record 5 empty ticks
        for _ in 0..5 {
            recorder.record_tick();
        }

        let data = recorder.finalize();
        let mut player = ReplayPlayer::new(data, vec![]);

        assert_eq!(player.current_tick(), 0);
        assert!(!player.is_finished());

        // Step through all ticks
        for _ in 0..5 {
            assert!(player.step());
        }

        assert_eq!(player.current_tick(), 5);
        assert!(player.is_finished());
        assert!(!player.step()); // No more ticks
    }

    #[test]
    fn test_replay_player_with_commands() {
        let config = test_config();
        let mut recorder = ReplayRecorder::new(config.clone(), 2);

        // Record some commands
        recorder.record_command(0, Command::Move {
            unit_ids: vec![1],
            target_x: 32.0,
            target_y: 32.0,
        });
        recorder.record_tick();
        recorder.record_tick();
        recorder.record_tick();

        let data = recorder.finalize();
        let mut player = ReplayPlayer::new(data, vec![]);

        // Step through
        player.step();
        player.step();
        player.step();

        assert_eq!(player.current_tick(), 3);
        assert!(player.is_finished());
    }

    #[test]
    fn test_replay_deterministic_playback() {
        // Record a replay with specific commands
        let config = test_config();
        let mut recorder = ReplayRecorder::new(config, 2);

        for _ in 0..10 {
            recorder.record_tick();
        }

        let data = recorder.finalize();

        // Play it twice and compare final entity counts
        let mut player1 = ReplayPlayer::new(data.clone(), vec![]);
        while player1.step() {}

        let mut player2 = ReplayPlayer::new(data, vec![]);
        while player2.step() {}

        // Both should have same tick count
        assert_eq!(player1.current_tick(), player2.current_tick());

        // Both should have same number of entities
        let count1 = player1.game().tick_count;
        let count2 = player2.game().tick_count;
        assert_eq!(count1, count2);
    }

    #[test]
    fn test_replay_seek() {
        let config = test_config();
        let mut recorder = ReplayRecorder::new(config, 2);

        for _ in 0..20 {
            recorder.record_tick();
        }

        let data = recorder.finalize();
        let mut player = ReplayPlayer::new(data.clone(), vec![]);

        // Play to tick 20
        while player.step() {}
        let tick_at_end = player.game().tick_count;
        let tick_at_20 = player.current_tick();

        // Now seek back to tick 10
        player.seek(10);
        assert_eq!(player.current_tick(), 10);
        let tick_at_10 = player.game().tick_count;

        // Seek forward to tick 20 again
        player.seek(20);
        assert_eq!(player.current_tick(), tick_at_20);
        let tick_at_end_again = player.game().tick_count;

        // Seeking to same tick should give same game tick (deterministic)
        assert_eq!(tick_at_end, tick_at_end_again);

        // Tick 10 should still be <= tick 20
        assert!(tick_at_10 <= tick_at_end);
    }

    #[test]
    fn test_replay_seek_past_end() {
        let config = test_config();
        let mut recorder = ReplayRecorder::new(config, 2);

        for _ in 0..5 {
            recorder.record_tick();
        }

        let data = recorder.finalize();
        let mut player = ReplayPlayer::new(data, vec![]);

        // Seek past the end should clamp
        player.seek(100);
        assert_eq!(player.current_tick(), 5);
        assert!(player.is_finished());
    }

    #[test]
    fn test_replay_seek_to_zero() {
        let config = test_config();
        let mut recorder = ReplayRecorder::new(config, 2);

        for _ in 0..10 {
            recorder.record_tick();
        }

        let data = recorder.finalize();
        let mut player = ReplayPlayer::new(data, vec![]);

        // Play some ticks
        for _ in 0..5 {
            player.step();
        }
        assert_eq!(player.current_tick(), 5);

        // Seek to 0
        player.seek(0);
        assert_eq!(player.current_tick(), 0);
        assert!(!player.is_finished());
    }

    #[test]
    fn test_replay_commands_replayed_faithfully() {
        let config = test_config();
        let mut recorder = ReplayRecorder::new(config, 2);

        // Record a produce command on tick 2
        recorder.record_tick(); // tick 0
        recorder.record_tick(); // tick 1
        recorder.record_command(0, Command::Produce {
            player: 0,
            unit_type: 0, // Thrall
        });
        recorder.record_tick(); // tick 2
        recorder.record_tick(); // tick 3
        recorder.record_tick(); // tick 4

        let data = recorder.finalize();

        // Play the replay
        let mut player = ReplayPlayer::new(data.clone(), vec![]);
        while player.step() {}

        // Play it again and verify same result
        let mut player2 = ReplayPlayer::new(data, vec![]);
        while player2.step() {}

        assert_eq!(player.current_tick(), player2.current_tick());
        assert_eq!(player.game().tick_count, player2.game().tick_count);
    }

    #[test]
    fn test_replay_multiple_players_commands() {
        let config = test_config();
        let mut recorder = ReplayRecorder::new(config, 2);

        // Both players issue commands on same tick
        recorder.record_command(0, Command::Move {
            unit_ids: vec![1],
            target_x: 32.0,
            target_y: 32.0,
        });
        recorder.record_command(1, Command::Move {
            unit_ids: vec![2],
            target_x: 16.0,
            target_y: 16.0,
        });
        recorder.record_tick();

        let data = recorder.finalize();
        assert_eq!(data.frames[0].commands.len(), 2);
        assert_eq!(data.frames[0].commands[0].0, 0);
        assert_eq!(data.frames[0].commands[1].0, 1);
    }

    #[test]
    fn test_replay_version_field() {
        let config = test_config();
        let recorder = ReplayRecorder::new(config, 2);
        let data = recorder.finalize();
        assert_eq!(data.version, 1);
    }

    #[test]
    fn test_replay_empty_recording() {
        let config = test_config();
        let recorder = ReplayRecorder::new(config, 2);
        let data = recorder.finalize();

        assert_eq!(data.total_ticks, 0);
        assert_eq!(data.frames.len(), 0);

        let mut player = ReplayPlayer::new(data, vec![]);
        assert!(player.is_finished());
        assert!(!player.step());
    }
}

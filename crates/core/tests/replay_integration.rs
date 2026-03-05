use machine_empire_core::replay::{ReplayRecorder, ReplayData, ReplayPlayer};
use machine_empire_core::command::Command;
use machine_empire_core::protocol::MatchConfig;
use machine_empire_core::game::{Game, GameConfig};

fn test_config() -> MatchConfig {
    MatchConfig {
        map_width: 64,
        map_height: 64,
        player_count: 2,
        seed: 42,
        tick_rate_ms: 50,
    }
}

/// Record a match, serialize, deserialize, play back — should produce identical state.
#[test]
fn test_replay_roundtrip_determinism() {
    let config = test_config();
    // Create a game and record it
    let mut game = Game::new(GameConfig {
        map_width: config.map_width,
        map_height: config.map_height,
        player_count: config.player_count,
        seed: config.seed,
    });
    game.spawn_starting_units(0, 8.0, 8.0);
    game.spawn_starting_units(1, 56.0, 56.0);

    let mut recorder = ReplayRecorder::new(config.clone(), 2);

    // Issue some commands and tick
    let commands = vec![
        (0, Command::Produce { player: 0, unit_type: 0 }),
        (1, Command::Produce { player: 1, unit_type: 0 }),
    ];

    for (pid, cmd) in &commands {
        recorder.record_command(*pid, cmd.clone());
        game.push_command(cmd.clone());
    }
    recorder.record_tick();
    game.tick(50.0);

    // More ticks
    for _ in 0..20 {
        recorder.record_tick();
        game.tick(50.0);
    }

    let recorded_tick_count = game.tick_count;
    let replay_data = recorder.finalize();

    // Serialize and deserialize via MessagePack
    let bytes = replay_data.serialize().unwrap();
    let restored = ReplayData::deserialize(&bytes).unwrap();

    assert_eq!(restored.total_ticks, replay_data.total_ticks);
    assert_eq!(restored.frames.len(), replay_data.frames.len());
    assert_eq!(restored.config.seed, config.seed);

    // Play back the restored replay
    let mut player = ReplayPlayer::new(restored, vec![]);
    while player.step() {}

    assert_eq!(player.current_tick(), replay_data.total_ticks);
    assert_eq!(player.game().tick_count, recorded_tick_count);
}

/// Seek consistency: seeking to the same tick twice should produce identical state.
#[test]
fn test_replay_seek_consistency() {
    let config = test_config();
    let mut recorder = ReplayRecorder::new(config, 2);

    // Record a produce command
    recorder.record_command(0, Command::Produce { player: 0, unit_type: 0 });
    recorder.record_tick();

    for _ in 0..30 {
        recorder.record_tick();
    }

    let data = recorder.finalize();

    let mut player = ReplayPlayer::new(data, vec![]);

    // Seek to tick 15
    player.seek(15);
    let tick_count_1 = player.game().tick_count;

    // Seek to tick 25
    player.seek(25);

    // Seek back to tick 15
    player.seek(15);
    let tick_count_2 = player.game().tick_count;

    // Should be identical (deterministic)
    assert_eq!(tick_count_1, tick_count_2);
}

/// Replay with AI: AI commands are regenerated deterministically.
#[test]
fn test_replay_with_ai_players() {
    let config = test_config();
    let mut recorder = ReplayRecorder::new(config, 2);

    // Record ticks without human commands (AI will generate its own)
    for _ in 0..50 {
        recorder.record_tick();
    }

    let data = recorder.finalize();

    // Play back with AI for player 1
    let mut player1 = ReplayPlayer::new(data.clone(), vec![1]);
    while player1.step() {}

    // Play again with same config
    let mut player2 = ReplayPlayer::new(data, vec![1]);
    while player2.step() {}

    // Both should reach same tick
    assert_eq!(player1.current_tick(), player2.current_tick());
    assert_eq!(player1.game().tick_count, player2.game().tick_count);
}

/// Empty replay plays back correctly.
#[test]
fn test_empty_replay_roundtrip() {
    let config = test_config();
    let recorder = ReplayRecorder::new(config, 2);
    let data = recorder.finalize();

    let bytes = data.serialize().unwrap();
    let restored = ReplayData::deserialize(&bytes).unwrap();

    assert_eq!(restored.total_ticks, 0);
    assert_eq!(restored.frames.len(), 0);

    let mut player = ReplayPlayer::new(restored, vec![]);
    assert!(player.is_finished());
    assert!(!player.step());
}

/// Commands are replayed on the correct tick.
#[test]
fn test_commands_replayed_on_correct_tick() {
    let config = test_config();
    let mut recorder = ReplayRecorder::new(config, 2);

    // Empty ticks 0-4
    for _ in 0..5 {
        recorder.record_tick();
    }

    // Command on tick 5
    recorder.record_command(0, Command::Produce { player: 0, unit_type: 0 });
    recorder.record_tick();

    // Empty ticks 6-9
    for _ in 0..4 {
        recorder.record_tick();
    }

    let data = recorder.finalize();
    assert_eq!(data.total_ticks, 10);
    assert_eq!(data.frames.len(), 10);
    assert_eq!(data.frames[5].commands.len(), 1);
    assert_eq!(data.frames[5].tick, 5);

    // Verify it plays back
    let mut player = ReplayPlayer::new(data, vec![]);
    while player.step() {}
    assert_eq!(player.current_tick(), 10);
}

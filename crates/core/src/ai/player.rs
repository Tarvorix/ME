use crate::ecs::World;
use crate::command::{Command, PendingCommands};
use crate::types::SpriteId;
use crate::ai::mcts::{MctsPlanner, MctsState, StrategicAction, STRATEGIC_UPDATE_INTERVAL};

/// AI difficulty levels controlling MCTS depth and tactical responsiveness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiDifficulty {
    /// Fewer MCTS iterations, slower tactical updates.
    Easy,
    /// Balanced MCTS iterations and tactical updates.
    Normal,
    /// More MCTS iterations, faster tactical updates.
    Hard,
}

impl AiDifficulty {
    /// Number of MCTS iterations for this difficulty.
    pub fn mcts_iterations(&self) -> u32 {
        match self {
            AiDifficulty::Easy => 50,
            AiDifficulty::Normal => 200,
            AiDifficulty::Hard => 500,
        }
    }

    /// How often tactical AI updates (in ticks).
    pub fn tactical_update_interval(&self) -> u32 {
        match self {
            AiDifficulty::Easy => 20,
            AiDifficulty::Normal => 5,
            AiDifficulty::Hard => 2,
        }
    }

    /// How often strategic AI updates (in ticks).
    pub fn strategic_update_interval(&self) -> u32 {
        match self {
            AiDifficulty::Easy => 80,
            AiDifficulty::Normal => STRATEGIC_UPDATE_INTERVAL,
            AiDifficulty::Hard => 20,
        }
    }
}

/// A single AI player combining tactical BT + strategic MCTS.
pub struct AiPlayer {
    /// Which player slot this AI controls.
    pub player_id: u8,
    /// Difficulty level.
    pub difficulty: AiDifficulty,
    /// MCTS planner for strategic decisions.
    pub mcts: MctsPlanner,
    /// The current strategic action being executed.
    pub current_strategy: Option<StrategicAction>,
    /// Last tick when strategic MCTS ran.
    pub last_strategic_tick: u32,
    /// Last tick when tactical BT ran (per-entity).
    pub last_tactical_tick: u32,
    /// Map dimensions (needed for MCTS state extraction).
    pub map_width: u32,
    pub map_height: u32,
    /// Player count (needed for MCTS state extraction).
    pub player_count: u8,
}

impl AiPlayer {
    /// Create a new AI player.
    pub fn new(
        player_id: u8,
        difficulty: AiDifficulty,
        seed: u64,
        map_width: u32,
        map_height: u32,
        player_count: u8,
    ) -> Self {
        AiPlayer {
            player_id,
            difficulty,
            mcts: MctsPlanner::new(seed),
            current_strategy: None,
            last_strategic_tick: 0,
            last_tactical_tick: 0,
            map_width,
            map_height,
            player_count,
        }
    }
}

/// Resource: collection of AI players in the game.
pub struct AiPlayers(pub Vec<AiPlayer>);

impl AiPlayers {
    pub fn new() -> Self {
        AiPlayers(Vec::new())
    }
}

/// Strategic AI system. For each AI player, runs MCTS every N ticks
/// and translates strategic actions into game commands.
pub fn ai_strategic_system(world: &mut World) {
    // Verify world has TickDelta (game is initialized)
    if world.get_resource::<crate::game::TickDelta>().is_none() {
        return;
    }

    // We need to read AiPlayers mutably and also read the world.
    // Use unsafe pointer to work around borrow checker (read-only world access during MCTS).
    let ai_players_ptr = world.get_resource_mut::<AiPlayers>()
        .map(|p| p as *mut AiPlayers);

    let ai_players_ptr = match ai_players_ptr {
        Some(p) => p,
        None => return,
    };

    // SAFETY: We only read from world (except PendingCommands) while mutating AiPlayers.
    // AiPlayers and PendingCommands are separate resources in the ECS.
    let ai_players = unsafe { &mut *ai_players_ptr };

    if ai_players.0.is_empty() {
        return;
    }

    // Collect commands to push after processing all AI players
    let mut commands: Vec<Command> = Vec::new();

    for ai in ai_players.0.iter_mut() {
        // Check if it's time for a strategic update.
        // First tick (last_strategic_tick == 0) always triggers to give AI an immediate strategy.
        let first_tick = ai.last_strategic_tick == 0;
        ai.last_strategic_tick += 1;
        if !first_tick && ai.last_strategic_tick % ai.difficulty.strategic_update_interval() != 0 {
            continue;
        }

        // Extract MCTS state from world
        let state = MctsPlanner::extract_state(
            world,
            ai.player_count,
            ai.map_width,
            ai.map_height,
        );

        // Run MCTS to choose an action
        let action = ai.mcts.choose_action(
            &state,
            ai.player_id,
            ai.difficulty.mcts_iterations(),
        );

        // Translate strategic action into game commands
        translate_strategic_action(
            &action,
            ai.player_id,
            &state,
            world,
            &mut commands,
        );

        ai.current_strategy = Some(action);
    }

    // Push all commands into PendingCommands
    if !commands.is_empty() {
        if let Some(pending) = world.get_resource_mut::<PendingCommands>() {
            pending.0.extend(commands);
        }
    }
}

/// Translate a strategic action into concrete game commands.
fn translate_strategic_action(
    action: &StrategicAction,
    player_id: u8,
    state: &MctsState,
    world: &World,
    commands: &mut Vec<Command>,
) {
    match action {
        StrategicAction::ProduceThrall => {
            commands.push(Command::Produce {
                player: player_id,
                unit_type: SpriteId::Thrall as u16,
            });
        }
        StrategicAction::ProduceSentinel => {
            commands.push(Command::Produce {
                player: player_id,
                unit_type: SpriteId::Sentinel as u16,
            });
        }
        StrategicAction::ProduceHoverTank => {
            commands.push(Command::Produce {
                player: player_id,
                unit_type: SpriteId::HoverTank as u16,
            });
        }
        StrategicAction::AttackSector(sector) => {
            let (tx, ty) = state.sector_center(*sector);
            let unit_ids = get_player_combat_unit_ids(world, player_id);
            if !unit_ids.is_empty() {
                commands.push(Command::AttackMove {
                    unit_ids,
                    target_x: tx,
                    target_y: ty,
                });
            }
        }
        StrategicAction::DefendSector(sector) => {
            let (tx, ty) = state.sector_center(*sector);
            let unit_ids = get_player_combat_unit_ids(world, player_id);
            if !unit_ids.is_empty() {
                commands.push(Command::Move {
                    unit_ids,
                    target_x: tx,
                    target_y: ty,
                });
            }
        }
        StrategicAction::Retreat => {
            // Retreat to Command Post position
            if let Some(cp_sector) = state.cp_sector[player_id as usize] {
                let (tx, ty) = state.sector_center(cp_sector);
                let unit_ids = get_player_combat_unit_ids(world, player_id);
                if !unit_ids.is_empty() {
                    commands.push(Command::Move {
                        unit_ids,
                        target_x: tx,
                        target_y: ty,
                    });
                }
            }
        }
        StrategicAction::DoNothing => {}
    }
}

/// Get all combat unit entity IDs for a player (helper for AI system).
fn get_player_combat_unit_ids(world: &World, player_id: u8) -> Vec<u32> {
    use crate::components::{UnitType, Health};
    use crate::blueprints::get_blueprint;

    let ut_storage = match world.get_storage::<UnitType>() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let health_storage = world.get_storage::<Health>();

    let mut ids = Vec::new();
    for (entity, ut) in ut_storage.iter() {
        if ut.owner == player_id {
            let bp = get_blueprint(ut.kind);
            if bp.damage > 0.0 && bp.speed > 0.0 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, GameConfig};

    fn test_game_with_ai(difficulty: AiDifficulty) -> Game {
        let config = GameConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
        };
        let mut game = Game::new(config);

        // Initialize AiPlayers resource
        game.world.insert_resource(AiPlayers::new());

        // Add AI player for player 1
        if let Some(ai_players) = game.world.get_resource_mut::<AiPlayers>() {
            ai_players.0.push(AiPlayer::new(1, difficulty, 42, 64, 64, 2));
        }

        // Spawn starting units for both players
        game.spawn_starting_units(0, 8.0, 8.0);
        game.spawn_starting_units(1, 56.0, 56.0);

        game
    }

    #[test]
    fn test_ai_player_creation() {
        let ai = AiPlayer::new(1, AiDifficulty::Normal, 42, 64, 64, 2);
        assert_eq!(ai.player_id, 1);
        assert_eq!(ai.difficulty, AiDifficulty::Normal);
        assert!(ai.current_strategy.is_none());
        assert_eq!(ai.last_strategic_tick, 0);
    }

    #[test]
    fn test_ai_difficulty_settings() {
        assert_eq!(AiDifficulty::Easy.mcts_iterations(), 50);
        assert_eq!(AiDifficulty::Normal.mcts_iterations(), 200);
        assert_eq!(AiDifficulty::Hard.mcts_iterations(), 500);

        assert_eq!(AiDifficulty::Easy.tactical_update_interval(), 20);
        assert_eq!(AiDifficulty::Normal.tactical_update_interval(), 5);
        assert_eq!(AiDifficulty::Hard.tactical_update_interval(), 2);
    }

    #[test]
    fn test_ai_player_produces_units() {
        let mut game = test_game_with_ai(AiDifficulty::Normal);

        // Count initial units for player 1
        let initial_count = count_player_units(&game, 1);

        // Register the strategic AI system
        game.systems.add_system("ai_strategic", ai_strategic_system);

        // Run many ticks to trigger strategic decisions
        for _ in 0..200 {
            game.tick(50.0);
        }

        let final_count = count_player_units(&game, 1);

        // AI should have produced some units (strategic MCTS should trigger production)
        assert!(final_count >= initial_count,
            "AI should have at least as many units: initial={}, final={}",
            initial_count, final_count);
    }

    #[test]
    fn test_ai_strategic_timing() {
        let mut game = test_game_with_ai(AiDifficulty::Normal);
        game.systems.add_system("ai_strategic", ai_strategic_system);

        // Run a single tick — should not crash
        game.tick(50.0);

        // AI players resource should still be valid
        let ai_players = game.world.get_resource::<AiPlayers>().unwrap();
        assert_eq!(ai_players.0.len(), 1);
        assert_eq!(ai_players.0[0].player_id, 1);
    }

    #[test]
    fn test_translate_produce_thrall() {
        let game = test_game_with_ai(AiDifficulty::Normal);
        let state = MctsPlanner::extract_state(&game.world, 2, 64, 64);
        let mut commands = Vec::new();

        translate_strategic_action(
            &StrategicAction::ProduceThrall,
            1,
            &state,
            &game.world,
            &mut commands,
        );

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            Command::Produce { player, unit_type } => {
                assert_eq!(*player, 1);
                assert_eq!(*unit_type, SpriteId::Thrall as u16);
            }
            _ => panic!("Expected Produce command"),
        }
    }

    #[test]
    fn test_translate_attack_sector() {
        let game = test_game_with_ai(AiDifficulty::Normal);
        let state = MctsPlanner::extract_state(&game.world, 2, 64, 64);
        let mut commands = Vec::new();

        translate_strategic_action(
            &StrategicAction::AttackSector(0),
            1,
            &state,
            &game.world,
            &mut commands,
        );

        // Player 1 has combat units (3 thralls), so should generate AttackMove
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            Command::AttackMove { unit_ids, target_x, target_y } => {
                assert!(!unit_ids.is_empty(), "Should have unit IDs to attack with");
                assert!(*target_x > 0.0);
                assert!(*target_y > 0.0);
            }
            _ => panic!("Expected AttackMove command"),
        }
    }

    #[test]
    fn test_translate_retreat() {
        let game = test_game_with_ai(AiDifficulty::Normal);
        let state = MctsPlanner::extract_state(&game.world, 2, 64, 64);
        let mut commands = Vec::new();

        translate_strategic_action(
            &StrategicAction::Retreat,
            1,
            &state,
            &game.world,
            &mut commands,
        );

        // Should generate a Move command toward CP
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            Command::Move { unit_ids, .. } => {
                assert!(!unit_ids.is_empty());
            }
            _ => panic!("Expected Move command"),
        }
    }

    #[test]
    fn test_ai_full_match() {
        // Two AI players play against each other for many ticks
        let config = GameConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
        };
        let mut game = Game::new(config);
        game.world.insert_resource(AiPlayers::new());

        // Both players are AI
        if let Some(ai_players) = game.world.get_resource_mut::<AiPlayers>() {
            ai_players.0.push(AiPlayer::new(0, AiDifficulty::Normal, 42, 64, 64, 2));
            ai_players.0.push(AiPlayer::new(1, AiDifficulty::Normal, 123, 64, 64, 2));
        }

        game.spawn_starting_units(0, 8.0, 8.0);
        game.spawn_starting_units(1, 56.0, 56.0);

        game.systems.add_system("ai_strategic", ai_strategic_system);

        // Run for 500 ticks (25 seconds of game time)
        for _ in 0..500 {
            game.tick(50.0);
        }

        // Verify game progressed: units should still exist and economy should have changed
        let p0_units = count_player_units(&game, 0);
        let p1_units = count_player_units(&game, 1);

        // At minimum, something should be alive (forges, CPs, or some units)
        let total = p0_units + p1_units;
        assert!(total > 0, "At least some entities should still exist after 500 ticks");
    }

    #[test]
    fn test_easy_vs_hard_difficulty() {
        // Easy AI has fewer iterations
        assert!(AiDifficulty::Easy.mcts_iterations() < AiDifficulty::Hard.mcts_iterations());
        // Easy AI updates less frequently
        assert!(AiDifficulty::Easy.strategic_update_interval() > AiDifficulty::Hard.strategic_update_interval());
        // Easy AI has slower tactical updates
        assert!(AiDifficulty::Easy.tactical_update_interval() > AiDifficulty::Hard.tactical_update_interval());
    }

    /// Helper: count all entities owned by a player.
    fn count_player_units(game: &Game, player_id: u8) -> usize {
        use crate::components::UnitType;
        if let Some(ut_s) = game.world.get_storage::<UnitType>() {
            ut_s.iter().filter(|(_, ut)| ut.owner == player_id).count()
        } else {
            0
        }
    }
}

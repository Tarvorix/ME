use crate::ecs::World;
use crate::components::{UnitType, Deployed};
use crate::blueprints::get_blueprint;
use crate::game::TickDelta;

/// Base strain decay rate (strain units per second at 0% strain).
const BASE_STRAIN_DECAY: f32 = 5.0;

/// Per-player economy state.
#[derive(Debug, Clone)]
pub struct PlayerEconomy {
    /// Current energy bank balance.
    pub energy_bank: f32,
    /// Base income from Node (energy/sec).
    pub base_income: f32,
    /// Income from mining stations (energy/sec).
    pub mining_income: f32,
    /// Income from relic sites (energy/sec).
    pub relic_income: f32,
    /// Conscription Strain (0.0 = healthy, 100.0 = crisis).
    pub conscription_strain: f32,
    /// Current production energy spending rate (energy/sec), set by production system.
    pub production_spending: f32,
}

impl PlayerEconomy {
    pub fn new() -> Self {
        PlayerEconomy {
            energy_bank: 500.0,
            base_income: 5.0,
            mining_income: 0.0,
            relic_income: 0.0,
            conscription_strain: 0.0,
            production_spending: 0.0,
        }
    }

    /// Compute the strain-based income penalty multiplier (0.0 = no penalty, 0.5 = 50% penalty).
    pub fn strain_income_penalty(&self) -> f32 {
        let s = self.conscription_strain;
        if s <= 30.0 {
            0.0
        } else if s <= 50.0 {
            // Linear from 5% to 15% over [30, 50]
            let t = (s - 30.0) / 20.0;
            0.05 + t * 0.10
        } else if s <= 70.0 {
            // Linear from 15% to 30% over [50, 70]
            let t = (s - 50.0) / 20.0;
            0.15 + t * 0.15
        } else if s <= 90.0 {
            // Linear from 30% to 50% over [70, 90]
            let t = (s - 70.0) / 20.0;
            0.30 + t * 0.20
        } else {
            // 50% + additional up to ~65% at strain 100
            let t = (s - 90.0) / 10.0;
            0.50 + t * 0.15
        }
    }

    /// Compute the strain-based production speed penalty multiplier (0.0 = no penalty).
    pub fn strain_production_penalty(&self) -> f32 {
        let s = self.conscription_strain;
        if s <= 30.0 {
            0.0
        } else if s <= 50.0 {
            let t = (s - 30.0) / 20.0;
            t * 0.10
        } else if s <= 70.0 {
            let t = (s - 50.0) / 20.0;
            0.10 + t * 0.15
        } else if s <= 90.0 {
            let t = (s - 70.0) / 20.0;
            0.25 + t * 0.25
        } else {
            let t = (s - 90.0) / 10.0;
            (0.50 + t * 0.40).min(0.90)
        }
    }

    /// Gross income before penalties (energy/sec).
    pub fn gross_income(&self) -> f32 {
        self.base_income + self.mining_income + self.relic_income
    }

    /// Net income after strain penalty (energy/sec).
    pub fn net_income(&self) -> f32 {
        self.gross_income() * (1.0 - self.strain_income_penalty())
    }

    /// Add strain from conscription (capped at 100).
    pub fn add_strain(&mut self, amount: f32) {
        self.conscription_strain = (self.conscription_strain + amount).min(100.0);
    }
}

/// Resource: per-player economies.
pub struct Economies(pub Vec<PlayerEconomy>);

impl Economies {
    pub fn new(player_count: u32) -> Self {
        Economies(
            (0..player_count)
                .map(|_| PlayerEconomy::new())
                .collect()
        )
    }
}

/// Resource system: computes income, upkeep, strain decay, energy balance.
pub fn resource_system(world: &mut World) {
    let delta = if let Some(td) = world.get_resource::<TickDelta>() {
        td.0
    } else {
        return;
    };

    // Compute per-player upkeep by scanning all units
    let upkeep_per_player: Vec<f32> = {
        let ut_storage = world.get_storage::<UnitType>();
        let deployed_storage = world.get_storage::<Deployed>();

        let economies = world.get_resource::<Economies>();
        if economies.is_none() {
            return;
        }
        let player_count = economies.unwrap().0.len();

        let mut upkeep = vec![0.0f32; player_count];

        if let Some(ut_s) = ut_storage {
            for (_entity, ut) in ut_s.iter() {
                let player = ut.owner as usize;
                if player >= player_count {
                    continue;
                }
                let bp = get_blueprint(ut.kind);

                // Check if deployed
                let is_deployed = deployed_storage.as_ref()
                    .and_then(|ds| ds.get(_entity))
                    .map(|d| d.0)
                    .unwrap_or(true); // Default to deployed if no Deployed component

                if is_deployed {
                    upkeep[player] += bp.deployed_upkeep;
                } else {
                    upkeep[player] += bp.garrisoned_upkeep;
                }
            }
        }

        upkeep
    };

    // Update economies
    let economies = if let Some(e) = world.get_resource_mut::<Economies>() {
        e
    } else {
        return;
    };

    for (i, econ) in economies.0.iter_mut().enumerate() {
        let upkeep = if i < upkeep_per_player.len() {
            upkeep_per_player[i]
        } else {
            0.0
        };

        // Net income = (base + mining + relic) * (1 - strain_penalty) - upkeep - production_spending
        let income = econ.net_income();
        let net = income - upkeep - econ.production_spending;

        econ.energy_bank += net * delta;
        if econ.energy_bank < 0.0 {
            econ.energy_bank = 0.0;
        }

        // Strain decay: recovery = BASE_DECAY × (1 - strain/100)²
        if econ.conscription_strain > 0.0 {
            let factor = 1.0 - econ.conscription_strain / 100.0;
            let recovery = BASE_STRAIN_DECAY * factor * factor;
            econ.conscription_strain -= recovery * delta;
            if econ.conscription_strain < 0.0 {
                econ.conscription_strain = 0.0;
            }
        }
    }
}

/// UIStateBuffer resource: 256 bytes of UI data readable by the client.
/// Layout: [0-3] energy_bank f32, [4-7] income f32, [8-11] expense f32,
///         [12-15] strain f32, [16-19] strain_income_penalty f32,
///         [20-23] game_tick u32, [24-27] game_time_secs f32,
///         [28] game_state u8, [29-31] padding,
///         [32-63] reserved (selected_ids, etc.),
///         [64-67] selected_count u32,
///         [68-195] production_queues,
///         [196-255] reserved
pub struct UIStateBuffer(pub Vec<u8>);

impl UIStateBuffer {
    pub fn new() -> Self {
        UIStateBuffer(vec![0u8; 256])
    }
}

/// Write current economy state to UIStateBuffer.
pub fn write_ui_state(world: &mut World) {
    let (energy, income, expense, strain, strain_penalty, game_tick) = {
        let economies = world.get_resource::<Economies>();
        if economies.is_none() {
            return;
        }
        let econ = &economies.unwrap().0;
        if econ.is_empty() {
            return;
        }
        // Local player = 0 for now
        let e = &econ[0];
        let upkeep = {
            // We'll compute a rough expense from the bank delta
            // For accuracy, recompute upkeep
            let ut_storage = world.get_storage::<UnitType>();
            let deployed_storage = world.get_storage::<Deployed>();
            let mut up = 0.0f32;
            if let Some(ut_s) = ut_storage {
                for (_entity, ut) in ut_s.iter() {
                    if ut.owner != 0 { continue; }
                    let bp = get_blueprint(ut.kind);
                    let is_deployed = deployed_storage.as_ref()
                        .and_then(|ds| ds.get(_entity))
                        .map(|d| d.0)
                        .unwrap_or(true);
                    if is_deployed {
                        up += bp.deployed_upkeep;
                    } else {
                        up += bp.garrisoned_upkeep;
                    }
                }
            }
            up + e.production_spending
        };
        (e.energy_bank, e.net_income(), upkeep, e.conscription_strain,
         e.strain_income_penalty(), 0u32) // game_tick filled below
    };

    // Get game_tick from somewhere — we don't have direct access, so we use 0 placeholder
    // The actual tick count will be set by game.rs before calling this

    let buf = if let Some(ui) = world.get_resource_mut::<UIStateBuffer>() {
        &mut ui.0
    } else {
        return;
    };

    // [0-3] energy_bank: f32
    buf[0..4].copy_from_slice(&energy.to_le_bytes());
    // [4-7] income: f32
    buf[4..8].copy_from_slice(&income.to_le_bytes());
    // [8-11] expense: f32
    buf[8..12].copy_from_slice(&expense.to_le_bytes());
    // [12-15] strain: f32
    buf[12..16].copy_from_slice(&strain.to_le_bytes());
    // [16-19] strain_income_penalty: f32
    buf[16..20].copy_from_slice(&strain_penalty.to_le_bytes());
    // [20-23] game_tick: u32 (placeholder, set by game.rs)
    buf[20..24].copy_from_slice(&game_tick.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, GameConfig};
    use crate::types::SpriteId;

    fn test_game() -> Game {
        Game::new(GameConfig {
            map_width: 16,
            map_height: 16,
            player_count: 2,
            seed: 42,
        })
    }

    #[test]
    fn test_starting_economy() {
        let econ = PlayerEconomy::new();
        assert_eq!(econ.energy_bank, 500.0);
        assert_eq!(econ.base_income, 5.0);
        assert_eq!(econ.conscription_strain, 0.0);
    }

    #[test]
    fn test_income_adds_to_bank() {
        let mut game = test_game();
        // No units, so no upkeep. Base income = 5.0/sec
        game.tick(50.0); // 0.05 seconds

        let econ = game.world.get_resource::<Economies>().unwrap();
        let bank = econ.0[0].energy_bank;
        // Expected: 500 + (5.0 * 0.05) = 500.25
        assert!((bank - 500.25).abs() < 0.01,
            "Bank should be ~500.25, got {}", bank);
    }

    #[test]
    fn test_upkeep_deducts() {
        let mut game = test_game();
        // Spawn a Thrall (deployed upkeep = 0.3/sec)
        game.spawn_thrall(5.5, 5.5, 0);

        game.tick(50.0); // 0.05s

        let econ = game.world.get_resource::<Economies>().unwrap();
        let bank = econ.0[0].energy_bank;
        // Expected: 500 + (5.0 - 0.3) * 0.05 = 500 + 0.235 = 500.235
        assert!((bank - 500.235).abs() < 0.01,
            "Bank should be ~500.235 with Thrall upkeep, got {}", bank);
    }

    #[test]
    fn test_strain_decays() {
        let mut game = test_game();

        // Set strain to 50
        if let Some(econ) = game.world.get_resource_mut::<Economies>() {
            econ.0[0].conscription_strain = 50.0;
        }

        game.tick(50.0); // 0.05s

        let econ = game.world.get_resource::<Economies>().unwrap();
        let strain = econ.0[0].conscription_strain;
        // recovery = 5.0 * (1 - 50/100)^2 = 5.0 * 0.25 = 1.25/sec
        // After 0.05s: 50 - 1.25*0.05 = 50 - 0.0625 = 49.9375
        assert!(strain < 50.0, "Strain should decay, got {}", strain);
        assert!((strain - 49.9375).abs() < 0.01,
            "Strain should be ~49.9375, got {}", strain);
    }

    #[test]
    fn test_strain_income_penalty() {
        let mut econ = PlayerEconomy::new();

        // No penalty below 30
        econ.conscription_strain = 0.0;
        assert_eq!(econ.strain_income_penalty(), 0.0);
        econ.conscription_strain = 30.0;
        assert_eq!(econ.strain_income_penalty(), 0.0);

        // 15% at strain 50
        econ.conscription_strain = 50.0;
        assert!((econ.strain_income_penalty() - 0.15).abs() < 0.001);

        // 30% at strain 70
        econ.conscription_strain = 70.0;
        assert!((econ.strain_income_penalty() - 0.30).abs() < 0.001);

        // 50% at strain 90
        econ.conscription_strain = 90.0;
        assert!((econ.strain_income_penalty() - 0.50).abs() < 0.001);
    }

    #[test]
    fn test_strain_squared_recovery() {
        // Recovery rate should follow squared curve
        // At strain 0: recovery = 5.0 * 1.0 = 5.0/sec
        let r0 = BASE_STRAIN_DECAY * (1.0 - 0.0 / 100.0_f32).powi(2);
        assert_eq!(r0, 5.0);

        // At strain 50: recovery = 5.0 * 0.25 = 1.25/sec
        let r50 = BASE_STRAIN_DECAY * (1.0 - 50.0 / 100.0_f32).powi(2);
        assert_eq!(r50, 1.25);

        // At strain 90: recovery = 5.0 * 0.01 = 0.05/sec
        let r90 = BASE_STRAIN_DECAY * (1.0 - 90.0 / 100.0_f32).powi(2);
        assert!((r90 - 0.05).abs() < 0.001);

        // At strain 99: recovery = 5.0 * 0.0001 = 0.0005/sec (very slow)
        let r99 = BASE_STRAIN_DECAY * (1.0 - 99.0 / 100.0_f32).powi(2);
        assert!((r99 - 0.0005).abs() < 0.0001);
    }

    #[test]
    fn test_bank_cannot_go_negative() {
        let mut game = test_game();

        // Set bank very low and add expensive units
        if let Some(econ) = game.world.get_resource_mut::<Economies>() {
            econ.0[0].energy_bank = 0.01;
        }

        // Spawn many Hover Tanks (high upkeep: 2.0/sec each)
        for i in 0..10 {
            game.spawn_unit(SpriteId::HoverTank, 5.5 + i as f32, 5.5, 0);
        }

        // Tick many times
        for _ in 0..100 {
            game.tick(50.0);
        }

        let econ = game.world.get_resource::<Economies>().unwrap();
        assert!(econ.0[0].energy_bank >= 0.0, "Bank should never go negative, got {}", econ.0[0].energy_bank);
    }

    #[test]
    fn test_strain_production_penalty() {
        let mut econ = PlayerEconomy::new();

        // No penalty below 30
        econ.conscription_strain = 0.0;
        assert_eq!(econ.strain_production_penalty(), 0.0);

        // 10% at strain 50
        econ.conscription_strain = 50.0;
        assert!((econ.strain_production_penalty() - 0.10).abs() < 0.001);

        // 25% at strain 70
        econ.conscription_strain = 70.0;
        assert!((econ.strain_production_penalty() - 0.25).abs() < 0.001);

        // 50% at strain 90
        econ.conscription_strain = 90.0;
        assert!((econ.strain_production_penalty() - 0.50).abs() < 0.001);
    }

    #[test]
    fn test_ui_state_buffer_energy() {
        let mut game = test_game();
        game.world.insert_resource(UIStateBuffer::new());

        game.tick(50.0);

        // Manually call write_ui_state
        write_ui_state(&mut game.world);

        let ui = game.world.get_resource::<UIStateBuffer>().unwrap();
        let energy = f32::from_le_bytes([ui.0[0], ui.0[1], ui.0[2], ui.0[3]]);
        assert!(energy > 0.0, "UI state should have positive energy, got {}", energy);
        assert!((energy - 500.25).abs() < 0.1, "Energy should be ~500.25, got {}", energy);
    }
}

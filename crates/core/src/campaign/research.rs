use serde::{Serialize, Deserialize};
use crate::blueprints::UnitBlueprint;
use crate::types::SpriteId;

/// Technology identifiers for the research system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TechId {
    // Tier 1 (200 energy, 60s, 1 relic)
    ThrallPlating,          // +20% HP for Thralls
    SentinelHeavyWeapons,   // +20% damage for Sentinels
    HoverTankReactiveArmor, // +15% HP for Hover Tanks
    ImprovedVision,         // +2 vision range for all units

    // Tier 2 (500 energy, 120s, 1 relic + prereq)
    ThrallFireRate,         // -20% attack cooldown for Thralls
    SentinelShields,        // +30% HP for Sentinels
    HoverTankSiege,         // +30% attack range for Hover Tanks
    FastProduction,         // -15% production time for all units

    // Tier 3 (1000 energy, 180s, 2 relics + prereq)
    ThrallRange,            // +2 attack range for Thralls
    SentinelStealth,        // Sentinels not visible in fog unless adjacent
    HoverTankOvercharge,    // +50% speed burst ability for Hover Tanks
    EconomicEfficiency,     // +20% income from all sources
}

impl TechId {
    /// Get the tier of this technology (1, 2, or 3).
    pub fn tier(&self) -> u8 {
        match self {
            TechId::ThrallPlating | TechId::SentinelHeavyWeapons |
            TechId::HoverTankReactiveArmor | TechId::ImprovedVision => 1,

            TechId::ThrallFireRate | TechId::SentinelShields |
            TechId::HoverTankSiege | TechId::FastProduction => 2,

            TechId::ThrallRange | TechId::SentinelStealth |
            TechId::HoverTankOvercharge | TechId::EconomicEfficiency => 3,
        }
    }

    /// Get the prerequisite technology for this tech (None for Tier 1).
    pub fn prerequisite(&self) -> Option<TechId> {
        match self {
            // Tier 1: no prerequisites
            TechId::ThrallPlating | TechId::SentinelHeavyWeapons |
            TechId::HoverTankReactiveArmor | TechId::ImprovedVision => None,

            // Tier 2: requires corresponding Tier 1
            TechId::ThrallFireRate => Some(TechId::ThrallPlating),
            TechId::SentinelShields => Some(TechId::SentinelHeavyWeapons),
            TechId::HoverTankSiege => Some(TechId::HoverTankReactiveArmor),
            TechId::FastProduction => Some(TechId::ImprovedVision),

            // Tier 3: requires corresponding Tier 2
            TechId::ThrallRange => Some(TechId::ThrallFireRate),
            TechId::SentinelStealth => Some(TechId::SentinelShields),
            TechId::HoverTankOvercharge => Some(TechId::HoverTankSiege),
            TechId::EconomicEfficiency => Some(TechId::FastProduction),
        }
    }
}

/// Definition for a technology.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TechDefinition {
    pub id: TechId,
    pub energy_cost: f32,
    pub research_time: f32,   // seconds
    pub required_relics: u32,
    pub prerequisite: Option<TechId>,
}

/// Get the definition for a technology.
pub fn get_tech_definition(tech: TechId) -> TechDefinition {
    let tier = tech.tier();
    let (cost, time, relics) = match tier {
        1 => (200.0, 60.0, 1),
        2 => (500.0, 120.0, 1),
        3 => (1000.0, 180.0, 2),
        _ => unreachable!(),
    };

    TechDefinition {
        id: tech,
        energy_cost: cost,
        research_time: time,
        required_relics: relics,
        prerequisite: tech.prerequisite(),
    }
}

/// Active research job.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResearchJob {
    pub tech_id: TechId,
    pub progress: f32,         // 0.0 to research_time
    pub research_time: f32,    // total time needed
}

/// Per-player research state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerResearch {
    /// Set of completed technologies.
    pub completed: Vec<TechId>,
    /// Currently active research job (one at a time).
    pub active_job: Option<ResearchJob>,
}

impl PlayerResearch {
    pub fn new() -> Self {
        PlayerResearch {
            completed: Vec::new(),
            active_job: None,
        }
    }

    /// Check if a technology has been researched.
    pub fn has_tech(&self, tech: TechId) -> bool {
        self.completed.contains(&tech)
    }

    /// Check if a technology can be researched.
    pub fn can_research(
        &self,
        tech: TechId,
        owned_relics: u32,
        available_energy: f32,
    ) -> bool {
        // Already researched?
        if self.has_tech(tech) {
            return false;
        }

        // Already researching something?
        if self.active_job.is_some() {
            return false;
        }

        let def = get_tech_definition(tech);

        // Check prerequisite
        if let Some(prereq) = def.prerequisite {
            if !self.has_tech(prereq) {
                return false;
            }
        }

        // Check relic requirement
        if owned_relics < def.required_relics {
            return false;
        }

        // Check energy
        if available_energy < def.energy_cost {
            return false;
        }

        true
    }

    /// Start researching a technology. Returns the energy cost.
    pub fn start_research(&mut self, tech: TechId) -> f32 {
        let def = get_tech_definition(tech);
        self.active_job = Some(ResearchJob {
            tech_id: tech,
            progress: 0.0,
            research_time: def.research_time,
        });
        def.energy_cost
    }

    /// Tick research progress. Returns true if research completed this tick.
    pub fn research_tick(&mut self, delta_secs: f32) -> bool {
        if let Some(job) = &mut self.active_job {
            job.progress += delta_secs;
            if job.progress >= job.research_time {
                let tech = job.tech_id;
                self.completed.push(tech);
                self.active_job = None;
                return true;
            }
        }
        false
    }
}

/// Apply technology modifiers to a unit blueprint.
pub fn apply_tech_modifiers(base: &UnitBlueprint, kind: SpriteId, researched: &[TechId]) -> UnitBlueprint {
    let mut bp = base.clone();

    for &tech in researched {
        match tech {
            TechId::ThrallPlating => {
                if kind == SpriteId::Thrall {
                    bp.max_hp *= 1.2;
                }
            }
            TechId::SentinelHeavyWeapons => {
                if kind == SpriteId::Sentinel {
                    bp.damage *= 1.2;
                }
            }
            TechId::HoverTankReactiveArmor => {
                if kind == SpriteId::HoverTank {
                    bp.max_hp *= 1.15;
                }
            }
            TechId::ImprovedVision => {
                if kind == SpriteId::Thrall || kind == SpriteId::Sentinel || kind == SpriteId::HoverTank {
                    bp.vision_range += 2.0;
                }
            }
            TechId::ThrallFireRate => {
                if kind == SpriteId::Thrall {
                    bp.attack_cooldown *= 0.8;
                }
            }
            TechId::SentinelShields => {
                if kind == SpriteId::Sentinel {
                    bp.max_hp *= 1.3;
                }
            }
            TechId::HoverTankSiege => {
                if kind == SpriteId::HoverTank {
                    bp.attack_range *= 1.3;
                }
            }
            TechId::FastProduction => {
                bp.build_time_secs *= 0.85;
            }
            TechId::ThrallRange => {
                if kind == SpriteId::Thrall {
                    bp.attack_range += 2.0;
                }
            }
            TechId::SentinelStealth => {
                // Handled at fog system level, not blueprint
            }
            TechId::HoverTankOvercharge => {
                // Speed burst is an ability, not a permanent stat change
                // Handled at command level; base speed increased slightly
                if kind == SpriteId::HoverTank {
                    bp.speed *= 1.1; // Base 10% speed bonus, burst is ability
                }
            }
            TechId::EconomicEfficiency => {
                // Handled at campaign economy level, not blueprint
            }
        }
    }

    bp
}

/// Get a modified blueprint incorporating all researched technologies.
pub fn get_modified_blueprint(kind: SpriteId, researched: &[TechId]) -> UnitBlueprint {
    let base = crate::blueprints::get_blueprint(kind);
    apply_tech_modifiers(base, kind, researched)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_requirements() {
        let def1 = get_tech_definition(TechId::ThrallPlating);
        assert_eq!(def1.energy_cost, 200.0);
        assert_eq!(def1.research_time, 60.0);
        assert_eq!(def1.required_relics, 1);
        assert!(def1.prerequisite.is_none());

        let def2 = get_tech_definition(TechId::ThrallFireRate);
        assert_eq!(def2.energy_cost, 500.0);
        assert_eq!(def2.research_time, 120.0);
        assert_eq!(def2.required_relics, 1);
        assert_eq!(def2.prerequisite, Some(TechId::ThrallPlating));

        let def3 = get_tech_definition(TechId::ThrallRange);
        assert_eq!(def3.energy_cost, 1000.0);
        assert_eq!(def3.research_time, 180.0);
        assert_eq!(def3.required_relics, 2);
        assert_eq!(def3.prerequisite, Some(TechId::ThrallFireRate));
    }

    #[test]
    fn test_can_research_basic() {
        let pr = PlayerResearch::new();
        // Can research Tier 1 with 1 relic and enough energy
        assert!(pr.can_research(TechId::ThrallPlating, 1, 500.0));
    }

    #[test]
    fn test_cannot_research_without_relics() {
        let pr = PlayerResearch::new();
        assert!(!pr.can_research(TechId::ThrallPlating, 0, 500.0));
    }

    #[test]
    fn test_cannot_research_without_energy() {
        let pr = PlayerResearch::new();
        assert!(!pr.can_research(TechId::ThrallPlating, 1, 100.0)); // needs 200
    }

    #[test]
    fn test_cannot_research_without_prereq() {
        let pr = PlayerResearch::new();
        // ThrallFireRate requires ThrallPlating
        assert!(!pr.can_research(TechId::ThrallFireRate, 1, 1000.0));
    }

    #[test]
    fn test_research_progress_and_completion() {
        let mut pr = PlayerResearch::new();
        let _cost = pr.start_research(TechId::ThrallPlating);

        // Not done yet
        let done = pr.research_tick(30.0);
        assert!(!done);
        assert!(pr.active_job.is_some());

        // Complete it (60s total)
        let done = pr.research_tick(31.0);
        assert!(done);
        assert!(pr.has_tech(TechId::ThrallPlating));
        assert!(pr.active_job.is_none());
    }

    #[test]
    fn test_one_at_a_time() {
        let mut pr = PlayerResearch::new();
        pr.start_research(TechId::ThrallPlating);
        assert!(!pr.can_research(TechId::SentinelHeavyWeapons, 1, 500.0),
            "Should not research while another is active");
    }

    #[test]
    fn test_hp_modifier_applied() {
        let techs = vec![TechId::ThrallPlating];
        let bp = get_modified_blueprint(SpriteId::Thrall, &techs);
        assert!((bp.max_hp - 80.0 * 1.2).abs() < 0.01, "Thrall HP should be +20%");
    }

    #[test]
    fn test_damage_modifier_applied() {
        let techs = vec![TechId::SentinelHeavyWeapons];
        let bp = get_modified_blueprint(SpriteId::Sentinel, &techs);
        let base = crate::blueprints::get_blueprint(SpriteId::Sentinel);
        assert!((bp.damage - base.damage * 1.2).abs() < 0.01, "Sentinel damage should be +20%");
    }

    #[test]
    fn test_range_modifier_applied() {
        let techs = vec![TechId::ThrallPlating, TechId::ThrallFireRate, TechId::ThrallRange];
        let bp = get_modified_blueprint(SpriteId::Thrall, &techs);
        let base = crate::blueprints::get_blueprint(SpriteId::Thrall);
        assert!((bp.attack_range - (base.attack_range + 2.0)).abs() < 0.01,
            "Thrall range should be +2");
    }

    #[test]
    fn test_tech_persists_after_relic_loss() {
        let mut pr = PlayerResearch::new();
        pr.start_research(TechId::ThrallPlating);
        pr.research_tick(61.0);

        assert!(pr.has_tech(TechId::ThrallPlating));
        // Tech stays even if player loses relics — already completed
    }

    #[test]
    fn test_all_definitions_valid() {
        let all_techs = vec![
            TechId::ThrallPlating, TechId::SentinelHeavyWeapons,
            TechId::HoverTankReactiveArmor, TechId::ImprovedVision,
            TechId::ThrallFireRate, TechId::SentinelShields,
            TechId::HoverTankSiege, TechId::FastProduction,
            TechId::ThrallRange, TechId::SentinelStealth,
            TechId::HoverTankOvercharge, TechId::EconomicEfficiency,
        ];

        assert_eq!(all_techs.len(), 12, "Should have 12 technologies");

        for tech in &all_techs {
            let def = get_tech_definition(*tech);
            assert!(def.energy_cost > 0.0);
            assert!(def.research_time > 0.0);
            assert!(def.required_relics >= 1);
        }
    }

    #[test]
    fn test_vision_modifier() {
        let techs = vec![TechId::ImprovedVision];
        let bp = get_modified_blueprint(SpriteId::Thrall, &techs);
        let base = crate::blueprints::get_blueprint(SpriteId::Thrall);
        assert!((bp.vision_range - (base.vision_range + 2.0)).abs() < 0.01);
    }

    #[test]
    fn test_fast_production_modifier() {
        let techs = vec![TechId::FastProduction];
        let bp = get_modified_blueprint(SpriteId::Thrall, &techs);
        let base = crate::blueprints::get_blueprint(SpriteId::Thrall);
        assert!((bp.build_time_secs - base.build_time_secs * 0.85).abs() < 0.01);
    }

    #[test]
    fn test_cannot_research_already_completed() {
        let mut pr = PlayerResearch::new();
        pr.start_research(TechId::ThrallPlating);
        pr.research_tick(61.0);
        assert!(!pr.can_research(TechId::ThrallPlating, 1, 500.0),
            "Should not research already completed tech");
    }
}

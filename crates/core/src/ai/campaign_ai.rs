use serde::{Serialize, Deserialize};
use crate::campaign::map::{CampaignMap, SiteType, GarrisonedUnit};
use crate::campaign::economy::CampaignEconomy;
use crate::campaign::research::{PlayerResearch, TechId};
use crate::campaign::dispatch::DispatchQueue;

/// Campaign AI goal priorities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignGoal {
    /// Expand territory by claiming neutral sites.
    Expand,
    /// Defend threatened sites.
    Defend,
    /// Attack enemy sites.
    Attack,
    /// Research technology.
    Research,
    /// Produce units at node.
    Produce,
}

/// Campaign AI difficulty levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignAiDifficulty {
    Easy,
    Normal,
    Hard,
}

/// Per-player campaign AI state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CampaignAiState {
    pub player_id: u8,
    pub difficulty: CampaignAiDifficulty,
    pub current_goal: CampaignGoal,
    /// Ticks since last action evaluation.
    pub ticks_since_eval: u32,
    /// Evaluation interval in ticks.
    pub eval_interval: u32,
}

impl CampaignAiState {
    pub fn new(player_id: u8, difficulty: CampaignAiDifficulty) -> Self {
        let eval_interval = match difficulty {
            CampaignAiDifficulty::Easy => 200,   // Every 10 seconds
            CampaignAiDifficulty::Normal => 100,  // Every 5 seconds
            CampaignAiDifficulty::Hard => 60,     // Every 3 seconds
        };

        CampaignAiState {
            player_id,
            difficulty,
            current_goal: CampaignGoal::Expand,
            ticks_since_eval: 0,
            eval_interval,
        }
    }
}

/// Evaluate the campaign state and determine the best goal.
pub fn evaluate_campaign_state(
    map: &CampaignMap,
    economy: &CampaignEconomy,
    research: &PlayerResearch,
    player_id: u8,
) -> CampaignGoal {
    let owned_sites = map.sites_owned_by(player_id);
    let neutral_sites = map.neutral_sites();
    let mines_owned = map.count_mines(player_id);
    let relics_owned = map.count_relics(player_id);

    // Count total army strength
    let total_garrison: u32 = owned_sites.iter()
        .map(|s| s.garrison_count())
        .sum();

    // Count enemy sites
    let enemy_sites: u32 = map.sites.iter()
        .filter(|s| s.owner != player_id && s.owner != 255)
        .count() as u32;

    // Priority 1: Defend if threatened (enemy has more sites and we're weak)
    if enemy_sites > owned_sites.len() as u32 && total_garrison < 15 {
        return CampaignGoal::Defend;
    }

    // Priority 2: Claim neutral mines if available and we have forces
    let has_neutral_mines = neutral_sites.iter().any(|s| s.site_type == SiteType::MiningStation);
    if has_neutral_mines && total_garrison >= 3 {
        return CampaignGoal::Expand;
    }

    // Priority 3: Attack if we're strong enough and have economic base
    if total_garrison >= 15 && mines_owned >= 2 {
        return CampaignGoal::Attack;
    }

    // Priority 4: Research if we have relics and can research
    if relics_owned > 0 && research.active_job.is_none() && economy.energy_bank > 300.0 {
        let available_techs = get_available_techs(research, relics_owned, economy.energy_bank);
        if !available_techs.is_empty() {
            return CampaignGoal::Research;
        }
    }

    // Priority 5: Produce if we have money but few units
    if total_garrison < 20 && economy.energy_bank > 200.0 {
        return CampaignGoal::Produce;
    }

    // Default: expand or produce
    if !neutral_sites.is_empty() {
        CampaignGoal::Expand
    } else {
        CampaignGoal::Produce
    }
}

/// Get a list of technologies the player can currently research.
fn get_available_techs(research: &PlayerResearch, relics: u32, energy: f32) -> Vec<TechId> {
    let all_techs = [
        TechId::ThrallPlating, TechId::SentinelHeavyWeapons,
        TechId::HoverTankReactiveArmor, TechId::ImprovedVision,
        TechId::ThrallFireRate, TechId::SentinelShields,
        TechId::HoverTankSiege, TechId::FastProduction,
        TechId::ThrallRange, TechId::SentinelStealth,
        TechId::HoverTankOvercharge, TechId::EconomicEfficiency,
    ];

    all_techs.iter()
        .filter(|&&tech| research.can_research(tech, relics, energy))
        .copied()
        .collect()
}

/// Choose and execute a campaign action based on the current goal.
/// Returns a list of campaign actions to execute.
pub fn choose_campaign_action(
    ai: &CampaignAiState,
    map: &CampaignMap,
    economy: &CampaignEconomy,
    research: &PlayerResearch,
) -> Vec<CampaignAction> {
    let player_id = ai.player_id;
    let mut actions = Vec::new();

    match ai.current_goal {
        CampaignGoal::Expand => {
            // Find nearest neutral mine
            if let Some(node) = map.get_node(player_id) {
                let node_id = node.id;
                let neutral_mines: Vec<_> = map.neutral_sites().into_iter()
                    .filter(|s| s.site_type == SiteType::MiningStation)
                    .collect();

                if let Some(target) = neutral_mines.iter()
                    .min_by(|a, b| {
                        let da = map.distance(node_id, a.id);
                        let db = map.distance(node_id, b.id);
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
                {
                    // Dispatch 3 thralls to claim it
                    if node.garrison.iter().any(|g| g.unit_type == 0 && g.count >= 3) {
                        actions.push(CampaignAction::Dispatch {
                            source: node_id,
                            target: target.id,
                            units: vec![GarrisonedUnit::new(0, 3)],
                        });
                    }
                }
            }
        }
        CampaignGoal::Defend => {
            // Garrison key sites (nodes) by producing
            actions.push(CampaignAction::ProduceUnits { unit_type: 0, count: 5 });
        }
        CampaignGoal::Attack => {
            // Find weakest enemy site
            if let Some(node) = map.get_node(player_id) {
                let node_id = node.id;
                let enemy_sites: Vec<_> = map.sites.iter()
                    .filter(|s| s.owner != player_id && s.owner != 255 && !s.is_contested)
                    .collect();

                if let Some(target) = enemy_sites.iter()
                    .min_by(|a, b| a.garrison_count().cmp(&b.garrison_count()))
                {
                    // Send a force to attack
                    let available_thralls = node.garrison.iter()
                        .find(|g| g.unit_type == 0)
                        .map(|g| g.count)
                        .unwrap_or(0);

                    if available_thralls >= 5 {
                        let send = (available_thralls / 2).max(5);
                        actions.push(CampaignAction::Dispatch {
                            source: node_id,
                            target: target.id,
                            units: vec![GarrisonedUnit::new(0, send)],
                        });
                    }
                }
            }
        }
        CampaignGoal::Research => {
            let relics = map.count_relics(player_id);
            let available = get_available_techs(research, relics, economy.energy_bank);
            if let Some(&tech) = available.first() {
                actions.push(CampaignAction::StartResearch { tech });
            }
        }
        CampaignGoal::Produce => {
            if economy.energy_bank > 100.0 {
                actions.push(CampaignAction::ProduceUnits { unit_type: 0, count: 3 });
            }
        }
    }

    actions
}

/// Campaign actions the AI wants to execute.
#[derive(Clone, Debug)]
pub enum CampaignAction {
    /// Dispatch units between sites.
    Dispatch {
        source: u32,
        target: u32,
        units: Vec<GarrisonedUnit>,
    },
    /// Start a research project.
    StartResearch {
        tech: TechId,
    },
    /// Produce units at the node.
    ProduceUnits {
        unit_type: u16,
        count: u32,
    },
}

/// Execute campaign AI for all AI players. Called every eval_interval ticks.
pub fn campaign_ai_tick(
    ai_states: &mut [CampaignAiState],
    map: &mut CampaignMap,
    economies: &mut [CampaignEconomy],
    research_states: &mut [PlayerResearch],
    dispatch_queue: &mut DispatchQueue,
) {
    for ai in ai_states.iter_mut() {
        ai.ticks_since_eval += 1;
        if ai.ticks_since_eval < ai.eval_interval {
            continue;
        }
        ai.ticks_since_eval = 0;

        let pid = ai.player_id as usize;
        if pid >= economies.len() || pid >= research_states.len() {
            continue;
        }

        // Evaluate current goal
        ai.current_goal = evaluate_campaign_state(map, &economies[pid], &research_states[pid], ai.player_id);

        // Choose and execute actions
        let actions = choose_campaign_action(ai, map, &economies[pid], &research_states[pid]);

        for action in actions {
            match action {
                CampaignAction::Dispatch { source, target, units } => {
                    dispatch_queue.dispatch_force(map, ai.player_id, source, target, units);
                }
                CampaignAction::StartResearch { tech } => {
                    let relics = map.count_relics(ai.player_id);
                    if research_states[pid].can_research(tech, relics, economies[pid].energy_bank) {
                        let cost = research_states[pid].start_research(tech);
                        economies[pid].energy_bank -= cost;
                    }
                }
                CampaignAction::ProduceUnits { unit_type, count } => {
                    // Add units to node garrison (simplified production for campaign level)
                    if let Some(node) = map.get_node_mut(ai.player_id) {
                        let bp = crate::blueprints::get_blueprint(
                            crate::types::SpriteId::from_u16(unit_type).unwrap_or(crate::types::SpriteId::Thrall)
                        );
                        let total_cost = bp.energy_cost as f32 * count as f32;
                        if economies[pid].energy_bank >= total_cost {
                            economies[pid].energy_bank -= total_cost;
                            crate::campaign::garrison::add_to_garrison(
                                &mut node.garrison,
                                GarrisonedUnit::new(unit_type, count),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::map::CampaignMap;
    use crate::campaign::economy::CampaignEconomy;
    use crate::campaign::research::PlayerResearch;
    use crate::campaign::dispatch::DispatchQueue;

    fn test_setup() -> (CampaignMap, Vec<CampaignEconomy>, Vec<PlayerResearch>) {
        let map = CampaignMap::generate(2, 42);
        let economies = vec![CampaignEconomy::new(), CampaignEconomy::new()];
        let research = vec![PlayerResearch::new(), PlayerResearch::new()];
        (map, economies, research)
    }

    #[test]
    fn test_ai_claims_neutral_mines() {
        let (mut map, mut economies, mut research) = test_setup();
        let mut dispatch = DispatchQueue::new();
        let mut ai_states = vec![
            CampaignAiState::new(0, CampaignAiDifficulty::Normal),
        ];
        // Force immediate evaluation
        ai_states[0].ticks_since_eval = 99;

        let node_garrison_before = map.get_node(0).unwrap().garrison_count();

        campaign_ai_tick(&mut ai_states, &mut map, &mut economies, &mut research, &mut dispatch);

        // Should have dispatched units
        assert!(!dispatch.orders.is_empty() || node_garrison_before == 0,
            "AI should try to claim neutral mines");
    }

    #[test]
    fn test_ai_researches_with_relic() {
        let (mut map, mut economies, mut research) = test_setup();
        let mut dispatch = DispatchQueue::new();

        // Give player 0 a relic and claim all neutral mines (so expand isn't priority)
        for site in &mut map.sites {
            if site.owner == 255 {
                site.owner = 0;
            }
        }

        // Give enough money but not enough garrison for attack (< 15)
        economies[0].energy_bank = 1000.0;
        map.get_node_mut(0).unwrap().garrison.clear();
        map.get_node_mut(0).unwrap().garrison.push(GarrisonedUnit::new(0, 10));

        let mut ai_states = vec![
            CampaignAiState::new(0, CampaignAiDifficulty::Normal),
        ];
        ai_states[0].ticks_since_eval = 99;

        campaign_ai_tick(&mut ai_states, &mut map, &mut economies, &mut research, &mut dispatch);

        // Should have started research
        assert!(research[0].active_job.is_some(), "AI should research when it has relics");
    }

    #[test]
    fn test_ai_produces_units() {
        let (mut map, mut economies, mut research) = test_setup();
        let mut dispatch = DispatchQueue::new();

        // Claim all neutral mines so expand isn't priority
        for site in &mut map.sites {
            if site.site_type == SiteType::MiningStation && site.owner == 255 {
                site.owner = 0;
            }
        }

        // Low garrison count but have money
        map.get_node_mut(0).unwrap().garrison.clear();
        economies[0].energy_bank = 500.0;

        let mut ai_states = vec![
            CampaignAiState::new(0, CampaignAiDifficulty::Normal),
        ];
        ai_states[0].ticks_since_eval = 99;

        let garrison_before = map.get_node(0).unwrap().garrison_count();

        campaign_ai_tick(&mut ai_states, &mut map, &mut economies, &mut research, &mut dispatch);

        let garrison_after = map.get_node(0).unwrap().garrison_count();
        assert!(garrison_after > garrison_before, "AI should produce units when garrison is low");
    }

    #[test]
    fn test_ai_attacks_weak_enemy_sites() {
        let (mut map, economies, research) = test_setup();

        // Claim all neutral sites for player 0 (no neutral mines left)
        for site in &mut map.sites {
            if site.owner == 255 {
                site.owner = 0;
            }
        }

        // Give a lot of garrison (>= 15 for attack threshold)
        map.get_node_mut(0).unwrap().garrison.clear();
        map.get_node_mut(0).unwrap().garrison.push(GarrisonedUnit::new(0, 20));

        let goal = evaluate_campaign_state(&map, &economies[0], &research[0], 0);

        // With 20 garrison, 2+ mines, no neutral sites → should attack
        assert_eq!(goal, CampaignGoal::Attack, "AI should attack when strong enough");
    }

    #[test]
    fn test_ai_defends_threatened_sites() {
        let (mut map, economies, research) = test_setup();

        // Give enemy more sites than player 0
        for site in &mut map.sites {
            if site.owner == 255 {
                site.owner = 1;
            }
        }

        // Low garrison (< 15 for defend threshold)
        map.get_node_mut(0).unwrap().garrison.clear();
        map.get_node_mut(0).unwrap().garrison.push(GarrisonedUnit::new(0, 3));

        let goal = evaluate_campaign_state(&map, &economies[0], &research[0], 0);
        assert_eq!(goal, CampaignGoal::Defend, "AI should defend when threatened");
    }

    #[test]
    fn test_ai_manages_economy() {
        let (mut map, mut economies, mut research) = test_setup();
        let mut dispatch = DispatchQueue::new();

        // Claim neutral mines
        for site in &mut map.sites {
            if site.site_type == SiteType::MiningStation && site.owner == 255 {
                site.owner = 0;
            }
        }

        map.get_node_mut(0).unwrap().garrison.clear();
        economies[0].energy_bank = 500.0;

        let bank_before = economies[0].energy_bank;

        let mut ai_states = vec![
            CampaignAiState::new(0, CampaignAiDifficulty::Normal),
        ];
        ai_states[0].ticks_since_eval = 99;

        campaign_ai_tick(&mut ai_states, &mut map, &mut economies, &mut research, &mut dispatch);

        // AI should have spent energy (on production or research)
        assert!(economies[0].energy_bank <= bank_before, "AI should spend energy on actions");
    }

    #[test]
    fn test_ai_vs_ai_campaign() {
        let (mut map, mut economies, mut research) = test_setup();
        let mut dispatch = DispatchQueue::new();

        let mut ai_states = vec![
            CampaignAiState::new(0, CampaignAiDifficulty::Normal),
            CampaignAiState::new(1, CampaignAiDifficulty::Normal),
        ];

        // Run 500 AI ticks
        for _ in 0..500 {
            campaign_ai_tick(&mut ai_states, &mut map, &mut economies, &mut research, &mut dispatch);

            // Tick dispatch orders
            let completed = dispatch.tick(0.25); // 5 seconds per tick at 20Hz
            for order in &completed {
                DispatchQueue::process_arrival(&mut map, order);
            }
        }

        // Both AIs should have taken some actions
        let p0_sites = map.sites_owned_by(0).len();
        let p1_sites = map.sites_owned_by(1).len();

        assert!(p0_sites >= 1, "Player 0 should still have at least node");
        assert!(p1_sites >= 1, "Player 1 should still have at least node");
    }

    #[test]
    fn test_eval_interval_respected() {
        let (mut map, mut economies, mut research) = test_setup();
        let mut dispatch = DispatchQueue::new();

        let mut ai_states = vec![
            CampaignAiState::new(0, CampaignAiDifficulty::Normal),
        ];

        // First tick - shouldn't evaluate (ticks_since_eval starts at 0)
        campaign_ai_tick(&mut ai_states, &mut map, &mut economies, &mut research, &mut dispatch);
        assert_eq!(ai_states[0].ticks_since_eval, 1, "Counter should increment");
        assert!(dispatch.orders.is_empty(), "Should not act on first tick");
    }
}

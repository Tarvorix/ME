use crate::campaign::map::{CampaignMap, GarrisonedUnit};
use crate::campaign::economy::{CampaignEconomy, campaign_resource_tick};
use crate::campaign::research::PlayerResearch;
use crate::campaign::dispatch::DispatchQueue;
use crate::campaign::bridge::{BattleRequest, BattleResult, create_battle_from_campaign, extract_battle_result, apply_battle_result};
use crate::ai::campaign_ai::{CampaignAiState, campaign_ai_tick};
use crate::game::Game;

/// Represents an active battle on the campaign map.
pub struct ActiveBattle {
    pub site_id: u32,
    pub attacker: u8,
    pub defender: u8,
    pub game: Game,
}

/// Top-level campaign game state.
pub struct CampaignGame {
    /// Campaign map with all sites.
    pub campaign_map: CampaignMap,
    /// Per-player economies.
    pub economies: Vec<CampaignEconomy>,
    /// Per-player research states.
    pub research: Vec<PlayerResearch>,
    /// Force dispatch queue.
    pub dispatch_queue: DispatchQueue,
    /// Active battles.
    pub active_battles: Vec<ActiveBattle>,
    /// Campaign AI states (for AI players).
    pub ai_states: Vec<CampaignAiState>,
    /// Total ticks elapsed.
    pub tick_count: u32,
    /// Whether the campaign is paused.
    pub paused: bool,
    /// Number of players.
    pub player_count: u8,
    /// Next battle ID.
    next_battle_id: u32,
}

impl CampaignGame {
    pub fn new(player_count: u8, seed: u64) -> Self {
        let map = CampaignMap::generate(player_count, seed);
        let economies: Vec<_> = (0..player_count).map(|_| CampaignEconomy::new()).collect();
        let research: Vec<_> = (0..player_count).map(|_| PlayerResearch::new()).collect();

        CampaignGame {
            campaign_map: map,
            economies,
            research,
            dispatch_queue: DispatchQueue::new(),
            active_battles: Vec::new(),
            ai_states: Vec::new(),
            tick_count: 0,
            paused: false,
            player_count,
            next_battle_id: 1, // Start at 1; 0 is the "no battle" sentinel on the client
        }
    }

    /// Add an AI player to the campaign.
    pub fn add_ai_player(&mut self, player_id: u8, difficulty: crate::ai::campaign_ai::CampaignAiDifficulty) {
        self.ai_states.push(CampaignAiState::new(player_id, difficulty));
    }

    /// Run one campaign tick (50ms at 20Hz).
    pub fn tick(&mut self) {
        if self.paused {
            return;
        }

        let delta_secs = 0.05; // 50ms

        // Update economies
        campaign_resource_tick(&mut self.economies, &self.campaign_map, delta_secs);

        // Advance research
        for pr in &mut self.research {
            pr.research_tick(delta_secs);
        }

        // Advance dispatch orders
        let completed_dispatches = self.dispatch_queue.tick(delta_secs);
        for order in &completed_dispatches {
            let triggers_battle = DispatchQueue::process_arrival(&mut self.campaign_map, order);
            if triggers_battle {
                self.trigger_battle(
                    order.target_site,
                    order.player,
                    order.units.clone(),
                );
            }
        }

        // Tick active battles
        let mut resolved_battles = Vec::new();
        for battle in &mut self.active_battles {
            battle.game.tick(50.0);

            if let Some(bs) = battle.game.world.get_resource::<crate::systems::battle_victory::BattleState>() {
                if bs.is_finished() {
                    if let Some(result) = extract_battle_result(&battle.game, battle.site_id, battle.attacker, battle.defender) {
                        resolved_battles.push(result);
                    }
                }
            }
        }

        // Resolve completed battles
        for result in &resolved_battles {
            self.resolve_battle(result);
        }

        // Remove resolved battles
        self.active_battles.retain(|b| {
            !resolved_battles.iter().any(|r| r.site_id == b.site_id)
        });

        // Run campaign AI
        campaign_ai_tick(
            &mut self.ai_states,
            &mut self.campaign_map,
            &mut self.economies,
            &mut self.research,
            &mut self.dispatch_queue,
        );

        self.tick_count += 1;
    }

    /// Trigger a battle at a site.
    pub fn trigger_battle(
        &mut self,
        site_id: u32,
        attacker: u8,
        attacker_forces: Vec<GarrisonedUnit>,
    ) {
        let site = match self.campaign_map.get_site(site_id) {
            Some(s) => s,
            None => return,
        };

        let defender = site.owner;
        let defender_forces = site.garrison.clone();

        // Mark site as contested
        if let Some(site) = self.campaign_map.get_site_mut(site_id) {
            site.is_contested = true;
            site.battle_id = Some(self.next_battle_id);
            self.next_battle_id += 1;
        }

        let request = BattleRequest {
            site_id,
            attacker,
            defender,
            attacker_forces,
            defender_forces,
            map_seed: (site_id * 1000 + self.tick_count) as u32,
        };

        let game = create_battle_from_campaign(&request);

        self.active_battles.push(ActiveBattle {
            site_id,
            attacker,
            defender,
            game,
        });
    }

    /// Resolve a completed battle.
    pub fn resolve_battle(&mut self, result: &BattleResult) {
        if let Some(site) = self.campaign_map.get_site_mut(result.site_id) {
            apply_battle_result(site, result);
        }
    }

    /// Check if any player has been eliminated (no forge alive).
    pub fn eliminated_players(&self) -> Vec<u8> {
        let mut eliminated = Vec::new();
        for pid in 0..self.player_count {
            let forge = self.campaign_map.get_forge(pid);
            if forge.is_none() {
                eliminated.push(pid);
            }
        }
        eliminated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::campaign_ai::CampaignAiDifficulty;

    #[test]
    fn test_campaign_game_creation() {
        let game = CampaignGame::new(2, 42);
        assert_eq!(game.player_count, 2);
        assert_eq!(game.tick_count, 0);
        assert_eq!(game.economies.len(), 2);
        assert_eq!(game.research.len(), 2);
        assert!(!game.paused);
    }

    #[test]
    fn test_campaign_tick_advances() {
        let mut game = CampaignGame::new(2, 42);
        game.tick();
        assert_eq!(game.tick_count, 1);
    }

    #[test]
    fn test_paused_does_nothing() {
        let mut game = CampaignGame::new(2, 42);
        game.paused = true;
        let bank_before = game.economies[0].energy_bank;
        game.tick();
        assert_eq!(game.tick_count, 0, "Tick count should not advance when paused");
        assert_eq!(game.economies[0].energy_bank, bank_before, "Economy should not change when paused");
    }

    #[test]
    fn test_campaign_with_ai() {
        let mut game = CampaignGame::new(2, 42);
        game.add_ai_player(0, CampaignAiDifficulty::Normal);
        game.add_ai_player(1, CampaignAiDifficulty::Normal);

        // Run many ticks
        for _ in 0..1000 {
            game.tick();
        }

        // Both players should still have forges
        assert!(game.campaign_map.get_forge(0).is_some());
        assert!(game.campaign_map.get_forge(1).is_some());
    }

    #[test]
    fn test_trigger_battle() {
        let mut game = CampaignGame::new(2, 42);

        // Give player 0 a mine
        let mine_id = game.campaign_map.sites.iter()
            .find(|s| s.site_type == crate::campaign::map::SiteType::MiningStation)
            .unwrap().id;
        game.campaign_map.get_site_mut(mine_id).unwrap().owner = 0;
        game.campaign_map.get_site_mut(mine_id).unwrap().garrison.push(GarrisonedUnit::new(0, 5));

        // Player 1 attacks
        game.trigger_battle(mine_id, 1, vec![GarrisonedUnit::new(0, 8)]);

        assert_eq!(game.active_battles.len(), 1);
        assert!(game.campaign_map.get_site(mine_id).unwrap().is_contested);
    }

    #[test]
    fn test_economic_progression() {
        let mut game = CampaignGame::new(2, 42);

        let bank_initial = game.economies[0].energy_bank;

        // Tick for 1 second (20 ticks)
        for _ in 0..20 {
            game.tick();
        }

        let bank_after = game.economies[0].energy_bank;
        // With forge income of 5/s and garrison upkeep, should be different from initial
        assert!(bank_after != bank_initial, "Economy should change over time");
    }
}

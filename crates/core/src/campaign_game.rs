use crate::campaign::map::{CampaignMap, GarrisonedUnit};
use crate::campaign::economy::{CampaignEconomy, campaign_resource_tick};
use crate::campaign::production::{
    CampaignProductions,
    CAMPAIGN_THRALL_STRAIN_AMOUNT,
    campaign_production_tick,
};
use crate::campaign::research::PlayerResearch;
use crate::campaign::dispatch::DispatchQueue;
use crate::campaign::bridge::{BattleRequest, BattleResult, create_battle_from_campaign, extract_battle_result, apply_battle_result, battle_local_slots};
use crate::campaign::garrison::remove_from_garrison;
use crate::ai::campaign_ai::{CampaignAiState, campaign_ai_tick};
use crate::game::Game;
use crate::systems::reinforcement::{PendingReinforcements, ReinforcementAvailability, ReinforcementRequest};
use crate::types::SpriteId;
use crate::components::Health;
use crate::deployment::DeploymentState;
use crate::protocol::EventSnapshot;
use crate::state_snapshot::snapshot_events;
use crate::types::EventType;

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
    /// Per-player production queues.
    pub productions: CampaignProductions,
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
        let productions = CampaignProductions::new(player_count);

        CampaignGame {
            campaign_map: map,
            economies,
            research,
            productions,
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

        // Advance queued node production.
        campaign_production_tick(
            &mut self.economies,
            &mut self.productions,
            &mut self.campaign_map,
            delta_secs,
        );

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

        // Update reinforcement availability for each battle (before ticking battles)
        self.update_reinforcement_availability();

        // Tick active battles
        let mut resolved_battles = Vec::new();
        let mut strain_relief: Vec<(u8, f32)> = Vec::new();
        for battle in &mut self.active_battles {
            battle.game.tick(50.0);
            collect_thrall_strain_relief(battle, &mut strain_relief);

            if let Some(bs) = battle.game.world.get_resource::<crate::systems::battle_victory::BattleState>() {
                if bs.is_finished() {
                    if let Some(result) = extract_battle_result(&battle.game, battle.site_id, battle.attacker, battle.defender) {
                        resolved_battles.push(result);
                    }
                }
            }
        }

        for (player, relief) in strain_relief {
            if let Some(econ) = self.economies.get_mut(player as usize) {
                econ.reduce_conscription_strain(relief);
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
            &mut self.productions,
            &mut self.research,
            &mut self.dispatch_queue,
        );

        // AI reinforcement requests: check every 3 seconds (60 ticks) if AI players
        // in active battles should request reinforcements
        if self.tick_count % 60 == 0 {
            self.ai_request_reinforcements();
        }

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

    /// Request reinforcements for a player in a specific battle.
    /// Validates: CP alive, units available in Node garrison, off cooldown.
    /// Deducts units from Node garrison and queues them in the battle.
    /// Returns true on success.
    pub fn request_reinforcement(
        &mut self,
        battle_index: usize,
        player: u8,
        unit_type: u16,
        count: u32,
    ) -> bool {
        if count == 0 {
            return false;
        }

        let sprite_id = match SpriteId::from_u16(unit_type) {
            Some(s) => s,
            None => return false,
        };

        // Only allow combat unit types (not buildings)
        if sprite_id == SpriteId::CommandPost || sprite_id == SpriteId::Node || sprite_id == SpriteId::CapturePoint {
            return false;
        }

        let battle = match self.active_battles.get_mut(battle_index) {
            Some(b) => b,
            None => return false,
        };
        let (local_attacker, local_defender) = battle_local_slots(battle.attacker, battle.defender);
        let local_player = if player == battle.attacker {
            local_attacker
        } else if player == battle.defender {
            local_defender
        } else {
            return false;
        };

        // Check CP is alive in the battle
        let cp_alive = {
            let ds = match battle.game.world.get_resource::<DeploymentState>() {
                Some(ds) => ds,
                None => return false,
            };

            let cp_entity = match ds.command_posts.get(local_player as usize) {
                Some(Some(e)) => *e,
                _ => return false,
            };

            let health_storage = battle.game.world.get_storage::<Health>();
            if let Some(hs) = health_storage {
                if let Some(h) = hs.get(cp_entity) {
                    !h.is_dead()
                } else {
                    true
                }
            } else {
                true
            }
        };

        if !cp_alive {
            return false;
        }

        // Check cooldown
        {
            let pr = match battle.game.world.get_resource::<PendingReinforcements>() {
                Some(pr) => pr,
                None => return false,
            };
            if !pr.can_request(player) {
                return false;
            }
        }

        // Find the player's Node on the campaign map and check garrison
        let node = match self.campaign_map.get_node_mut(player) {
            Some(n) => n,
            None => return false,
        };

        // Try to remove units from Node garrison
        let removed = match remove_from_garrison(&mut node.garrison, unit_type, count) {
            Some(r) => r,
            None => return false, // Not enough units
        };

        // Queue the reinforcement in the battle
        let battle = self.active_battles.get_mut(battle_index).unwrap();
        if let Some(pr) = battle.game.world.get_resource_mut::<PendingReinforcements>() {
            pr.requests.push(ReinforcementRequest {
                player: local_player,
                unit_type: sprite_id,
                count,
                health_pct: removed.health_pct,
                ticks_remaining: 100, // 5 seconds at 20Hz
            });
            pr.start_cooldown(local_player);
        }

        true
    }

    /// Update ReinforcementAvailability in each active battle based on current Node garrison.
    fn update_reinforcement_availability(&mut self) {
        for battle in &mut self.active_battles {
            let mut avail = ReinforcementAvailability::new(2);
            let (local_attacker, local_defender) = battle_local_slots(battle.attacker, battle.defender);

            // For each player in the battle (campaign owner + localized battle slot)
            for &(player, local_player) in &[
                (battle.attacker, local_attacker),
                (battle.defender, local_defender),
            ] {
                let pidx = local_player as usize;

                // Check CP alive
                let cp_alive = {
                    let ds = battle.game.world.get_resource::<DeploymentState>();
                    if let Some(ds) = ds {
                        if let Some(Some(cp)) = ds.command_posts.get(pidx) {
                            let hs = battle.game.world.get_storage::<Health>();
                            if let Some(hs) = hs {
                                hs.get(*cp).map(|h| !h.is_dead()).unwrap_or(true)
                            } else {
                                true
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };
                avail.cp_alive[pidx] = cp_alive;

                // Read Node garrison for this player
                if let Some(node) = self.campaign_map.get_node(player) {
                    avail.available[pidx] = node.garrison.iter()
                        .filter(|g| {
                            // Only include combat unit types
                            let sid = SpriteId::from_u16(g.unit_type);
                            matches!(sid, Some(SpriteId::Thrall) | Some(SpriteId::Sentinel) | Some(SpriteId::HoverTank))
                        })
                        .map(|g| (g.unit_type, g.count))
                        .collect();
                }
            }

            battle.game.world.insert_resource(avail);
        }
    }

    /// AI players in active battles request reinforcements when they're losing.
    fn ai_request_reinforcements(&mut self) {
        // Collect AI player IDs
        let ai_player_ids: Vec<u8> = self.ai_states.iter().map(|a| a.player_id).collect();

        // First pass: collect what reinforcements to request
        let mut requests: Vec<(usize, u8, u16, u32)> = Vec::new(); // (battle_idx, player, unit_type, count)

        for battle_idx in 0..self.active_battles.len() {
            let battle = &self.active_battles[battle_idx];
                let attacker = battle.attacker;
                let defender = battle.defender;
                let (local_attacker, local_defender) = battle_local_slots(attacker, defender);

                for &player in &[attacker, defender] {
                    if !ai_player_ids.contains(&player) {
                        continue;
                    }

                    let local_player = if player == attacker {
                        local_attacker
                    } else {
                        local_defender
                    };
                    let alive_units = battle.game.get_player_combat_unit_ids(local_player).len();

                if alive_units < 5 {
                    // Check garrison availability
                    if let Some(node) = self.campaign_map.get_node(player) {
                        let avail_thralls = node.garrison.iter()
                            .find(|g| g.unit_type == 0)
                            .map(|g| g.count)
                            .unwrap_or(0);

                        if avail_thralls >= 3 {
                            requests.push((battle_idx, player, 0, 3));
                        } else if avail_thralls > 0 {
                            requests.push((battle_idx, player, 0, avail_thralls));
                        } else {
                            let avail_sentinels = node.garrison.iter()
                                .find(|g| g.unit_type == 1)
                                .map(|g| g.count)
                                .unwrap_or(0);
                            if avail_sentinels > 0 {
                                requests.push((battle_idx, player, 1, 1));
                            }
                        }
                    }
                }
            }
        }

        // Second pass: execute requests
        for (battle_idx, player, unit_type, count) in requests {
            self.request_reinforcement(battle_idx, player, unit_type, count);
        }
    }

    /// Check if any player has been eliminated (no node alive).
    pub fn eliminated_players(&self) -> Vec<u8> {
        let mut eliminated = Vec::new();
        for pid in 0..self.player_count {
            let node = self.campaign_map.get_node(pid);
            if node.is_none() {
                eliminated.push(pid);
            }
        }
        eliminated
    }
}

fn collect_thrall_strain_relief(battle: &ActiveBattle, relief: &mut Vec<(u8, f32)>) {
    for event in snapshot_events(&battle.game.world) {
        if !is_thrall_death_event(&event) {
            continue;
        }

        let owner = event.payload[2];
        let (local_attacker, local_defender) = battle_local_slots(battle.attacker, battle.defender);
        let campaign_player = if owner == local_attacker {
            battle.attacker
        } else if owner == local_defender {
            battle.defender
        } else {
            continue;
        };

        relief.push((campaign_player, CAMPAIGN_THRALL_STRAIN_AMOUNT));
    }
}

fn is_thrall_death_event(event: &EventSnapshot) -> bool {
    if event.event_type != EventType::Death as u16 {
        return false;
    }

    let sprite_id = u16::from_le_bytes([event.payload[0], event.payload[1]]);
    sprite_id == SpriteId::Thrall as u16
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

        // Both players should still have nodes
        assert!(game.campaign_map.get_node(0).is_some());
        assert!(game.campaign_map.get_node(1).is_some());
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
    fn test_reinforcement_request_deducts_garrison() {
        let mut game = CampaignGame::new(2, 42);

        // Get initial garrison count at player 0's node
        let initial_thralls = game.campaign_map.get_node(0).unwrap().garrison.iter()
            .find(|g| g.unit_type == 0).map(|g| g.count).unwrap_or(0);

        // Trigger a battle so we have an active battle
        let mine_id = game.campaign_map.sites.iter()
            .find(|s| s.site_type == crate::campaign::map::SiteType::MiningStation)
            .unwrap().id;
        game.campaign_map.get_site_mut(mine_id).unwrap().owner = 1;
        game.campaign_map.get_site_mut(mine_id).unwrap().garrison.push(GarrisonedUnit::new(0, 3));

        game.trigger_battle(mine_id, 0, vec![GarrisonedUnit::new(0, 5)]);
        assert_eq!(game.active_battles.len(), 1);

        // Tick once to set up reinforcement availability
        game.tick();

        // Request reinforcements
        let result = game.request_reinforcement(0, 0, 0, 3);
        assert!(result, "Reinforcement request should succeed");

        // Check garrison was deducted
        let after_thralls = game.campaign_map.get_node(0).unwrap().garrison.iter()
            .find(|g| g.unit_type == 0).map(|g| g.count).unwrap_or(0);
        assert_eq!(after_thralls, initial_thralls - 3, "Garrison should be deducted by 3");
    }

    #[test]
    fn test_reinforcement_fails_insufficient_garrison() {
        let mut game = CampaignGame::new(2, 42);

        // Trigger a battle
        let mine_id = game.campaign_map.sites.iter()
            .find(|s| s.site_type == crate::campaign::map::SiteType::MiningStation)
            .unwrap().id;
        game.campaign_map.get_site_mut(mine_id).unwrap().owner = 1;
        game.campaign_map.get_site_mut(mine_id).unwrap().garrison.push(GarrisonedUnit::new(0, 3));

        game.trigger_battle(mine_id, 0, vec![GarrisonedUnit::new(0, 5)]);
        game.tick();

        // Request more Hover Tanks than available (should have 1 in garrison)
        let result = game.request_reinforcement(0, 0, 2, 100);
        assert!(!result, "Should fail when requesting more units than available");
    }

    #[test]
    fn test_reinforcement_cooldown() {
        let mut game = CampaignGame::new(2, 42);

        // Trigger a battle
        let mine_id = game.campaign_map.sites.iter()
            .find(|s| s.site_type == crate::campaign::map::SiteType::MiningStation)
            .unwrap().id;
        game.campaign_map.get_site_mut(mine_id).unwrap().owner = 1;
        game.campaign_map.get_site_mut(mine_id).unwrap().garrison.push(GarrisonedUnit::new(0, 3));

        game.trigger_battle(mine_id, 0, vec![GarrisonedUnit::new(0, 5)]);
        game.tick();

        // First request should succeed
        let result1 = game.request_reinforcement(0, 0, 0, 1);
        assert!(result1, "First request should succeed");

        // Second request should fail (cooldown)
        let result2 = game.request_reinforcement(0, 0, 0, 1);
        assert!(!result2, "Second request should fail due to cooldown");
    }

    #[test]
    fn test_reinforcement_units_arrive() {
        let mut game = CampaignGame::new(2, 42);

        // Trigger a battle
        let mine_id = game.campaign_map.sites.iter()
            .find(|s| s.site_type == crate::campaign::map::SiteType::MiningStation)
            .unwrap().id;
        game.campaign_map.get_site_mut(mine_id).unwrap().owner = 1;
        game.campaign_map.get_site_mut(mine_id).unwrap().garrison.push(GarrisonedUnit::new(0, 3));

        game.trigger_battle(mine_id, 0, vec![GarrisonedUnit::new(0, 5)]);
        game.tick();

        // Count player 0's thralls in the battle before reinforcement
        let thralls_before = game.active_battles[0].game.get_player_combat_unit_ids(0).len();

        // Request 2 thrall reinforcements
        let result = game.request_reinforcement(0, 0, 0, 2);
        assert!(result);

        // Tick until reinforcements arrive (100+ ticks)
        for _ in 0..105 {
            game.tick();
        }

        // Count thralls after reinforcement
        let thralls_after = game.active_battles[0].game.get_player_combat_unit_ids(0).len();
        assert!(thralls_after > thralls_before,
            "Should have more units after reinforcement, before={} after={}", thralls_before, thralls_after);
    }

    #[test]
    fn test_reinforcement_does_not_add_duplicate_strain() {
        let mut game = CampaignGame::new(2, 42);

        let mine_id = game.campaign_map.sites.iter()
            .find(|s| s.site_type == crate::campaign::map::SiteType::MiningStation)
            .unwrap().id;
        game.campaign_map.get_site_mut(mine_id).unwrap().owner = 1;
        game.campaign_map.get_site_mut(mine_id).unwrap().garrison.push(GarrisonedUnit::new(0, 3));

        game.trigger_battle(mine_id, 0, vec![GarrisonedUnit::new(0, 5)]);
        game.tick();

        game.economies[0].strain = 18.0;
        let result = game.request_reinforcement(0, 0, 0, 2);
        assert!(result, "Reinforcement request should succeed");
        assert_eq!(game.economies[0].strain, 18.0, "Moving existing thralls should not add strain again");
    }

    #[test]
    fn test_thrall_death_lowers_campaign_strain() {
        let mut game = CampaignGame::new(2, 42);

        let mine_id = game.campaign_map.sites.iter()
            .find(|s| s.site_type == crate::campaign::map::SiteType::MiningStation)
            .unwrap().id;
        game.campaign_map.get_site_mut(mine_id).unwrap().owner = 1;
        game.campaign_map.get_site_mut(mine_id).unwrap().garrison.push(GarrisonedUnit::new(0, 1));

        game.trigger_battle(mine_id, 0, vec![GarrisonedUnit::new(0, 2)]);
        game.economies[0].strain = 30.0;

        let attacker_thrall = {
            let ut_storage = game.active_battles[0].game.world.get_storage::<crate::components::UnitType>().unwrap();
            ut_storage.iter()
                .find(|(_, ut)| ut.owner == 0 && ut.kind == SpriteId::Thrall)
                .map(|(entity, _)| entity)
                .unwrap()
        };
        if let Some(health) = game.active_battles[0].game.world.get_component_mut::<Health>(attacker_thrall) {
            health.current = 0.0;
        }

        game.tick();

        assert!(
            game.economies[0].strain < 30.0,
            "A dead Thrall should immediately relieve campaign strain",
        );
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
        // With node income of 5/s and garrison upkeep, should be different from initial
        assert!(bank_after != bank_initial, "Economy should change over time");
    }
}

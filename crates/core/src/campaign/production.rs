use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::blueprints::get_blueprint;
use crate::types::SpriteId;

use super::economy::CampaignEconomy;
use super::map::GarrisonedUnit;

/// Campaign strain added when a Thrall finishes production.
pub const CAMPAIGN_THRALL_STRAIN_AMOUNT: f32 = 2.0;

/// A single unit currently being built at the campaign node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CampaignProductionJob {
    pub unit_type: u16,
    pub progress: f32,
    pub total_time: f32,
}

/// Per-player campaign production queue.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerProductionQueue {
    pub active_job: Option<CampaignProductionJob>,
    pub queued_units: VecDeque<u16>,
}

impl PlayerProductionQueue {
    pub fn new() -> Self {
        PlayerProductionQueue {
            active_job: None,
            queued_units: VecDeque::new(),
        }
    }

    pub fn queue_unit(&mut self, unit_type: SpriteId, count: u32) {
        for _ in 0..count {
            self.queued_units.push_back(unit_type as u16);
        }
        self.ensure_active_job();
    }

    pub fn queued_count(&self) -> u32 {
        self.queued_units.len() as u32
    }

    pub fn queued_counts_by_type(&self) -> (u32, u32, u32) {
        let mut thralls = 0u32;
        let mut sentinels = 0u32;
        let mut tanks = 0u32;

        for unit_type in &self.queued_units {
            match SpriteId::from_u16(*unit_type) {
                Some(SpriteId::Thrall) => thralls += 1,
                Some(SpriteId::Sentinel) => sentinels += 1,
                Some(SpriteId::HoverTank) => tanks += 1,
                _ => {}
            }
        }

        (thralls, sentinels, tanks)
    }

    pub fn tick(&mut self, delta_secs: f32, speed_scale: f32) -> Option<SpriteId> {
        self.ensure_active_job();

        let active_job = self.active_job.as_mut()?;
        active_job.progress += delta_secs * speed_scale.max(0.0);

        if active_job.progress < active_job.total_time {
            return None;
        }

        let completed = SpriteId::from_u16(active_job.unit_type)?;
        self.active_job = None;
        self.ensure_active_job();
        Some(completed)
    }

    fn ensure_active_job(&mut self) {
        if self.active_job.is_some() {
            return;
        }

        let unit_type = match self.queued_units.pop_front() {
            Some(unit_type) => unit_type,
            None => return,
        };

        let kind = match SpriteId::from_u16(unit_type) {
            Some(kind) => kind,
            None => return,
        };
        let blueprint = get_blueprint(kind);

        self.active_job = Some(CampaignProductionJob {
            unit_type,
            progress: 0.0,
            total_time: blueprint.build_time_secs,
        });
    }
}

impl Default for PlayerProductionQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-player campaign production state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CampaignProductions(pub Vec<PlayerProductionQueue>);

impl CampaignProductions {
    pub fn new(player_count: u8) -> Self {
        CampaignProductions(
            (0..player_count)
                .map(|_| PlayerProductionQueue::new())
                .collect(),
        )
    }
}

/// Queue campaign production for a player's node, paying the energy cost up front.
pub fn queue_campaign_production(
    economies: &mut [CampaignEconomy],
    productions: &mut CampaignProductions,
    player: u8,
    unit_type: u16,
    count: u32,
    node_exists: bool,
) -> bool {
    if count == 0 || !node_exists {
        return false;
    }

    let kind = match SpriteId::from_u16(unit_type) {
        Some(kind @ SpriteId::Thrall) | Some(kind @ SpriteId::Sentinel) | Some(kind @ SpriteId::HoverTank) => kind,
        _ => return false,
    };

    let player_idx = player as usize;
    if player_idx >= economies.len() || player_idx >= productions.0.len() {
        return false;
    }

    let blueprint = get_blueprint(kind);
    let total_cost = blueprint.energy_cost as f32 * count as f32;
    if economies[player_idx].energy_bank < total_cost {
        return false;
    }

    economies[player_idx].energy_bank -= total_cost;
    productions.0[player_idx].queue_unit(kind, count);
    true
}

/// Advance all campaign production queues and move completed units into the owning node garrison.
pub fn campaign_production_tick(
    economies: &mut [CampaignEconomy],
    productions: &mut CampaignProductions,
    map: &mut super::map::CampaignMap,
    delta_secs: f32,
) {
    let mut completed_units: Vec<(u8, SpriteId)> = Vec::new();

    for (player_idx, queue) in productions.0.iter_mut().enumerate() {
        let speed_scale = economies
            .get(player_idx)
            .map(|econ| 1.0 - econ.strain_production_penalty())
            .unwrap_or(1.0);

        if let Some(unit_type) = queue.tick(delta_secs, speed_scale) {
            completed_units.push((player_idx as u8, unit_type));
        }
    }

    for (player, unit_type) in completed_units {
        let delivered = if let Some(node) = map.get_node_mut(player) {
            super::garrison::add_to_garrison(
                &mut node.garrison,
                GarrisonedUnit::new(unit_type as u16, 1),
            );
            true
        } else {
            false
        };

        if delivered && unit_type == SpriteId::Thrall {
            if let Some(econ) = economies.get_mut(player as usize) {
                econ.add_conscription_strain(CAMPAIGN_THRALL_STRAIN_AMOUNT);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::economy::CampaignEconomy;
    use crate::campaign::map::CampaignMap;

    #[test]
    fn queue_builds_one_unit_at_a_time() {
        let mut economies = vec![CampaignEconomy::new(), CampaignEconomy::new()];
        let mut productions = CampaignProductions::new(2);
        let mut map = CampaignMap::generate(2, 42);

        let queued = queue_campaign_production(
            &mut economies,
            &mut productions,
            0,
            SpriteId::Thrall as u16,
            2,
            true,
        );
        assert!(queued, "Thralls should queue successfully");

        // First Thrall finishes after 15 seconds.
        for _ in 0..300 {
            campaign_production_tick(&mut economies, &mut productions, &mut map, 0.05);
        }

        let node = map.get_node(0).unwrap();
        let thralls_after_first = node.garrison.iter()
            .find(|g| g.unit_type == SpriteId::Thrall as u16)
            .map(|g| g.count)
            .unwrap_or(0);
        assert_eq!(thralls_after_first, 11, "Only one queued Thrall should be complete");

        // Second Thrall should still be active, not complete yet.
        let active_job = productions.0[0].active_job.as_ref();
        assert!(active_job.is_some(), "Second Thrall should still be building");

        for _ in 0..300 {
            campaign_production_tick(&mut economies, &mut productions, &mut map, 0.05);
        }

        let node = map.get_node(0).unwrap();
        let thralls_after_second = node.garrison.iter()
            .find(|g| g.unit_type == SpriteId::Thrall as u16)
            .map(|g| g.count)
            .unwrap_or(0);
        assert_eq!(thralls_after_second, 12, "Second queued Thrall should complete afterward");
    }

    #[test]
    fn strain_is_added_when_thrall_completes() {
        let mut economies = vec![CampaignEconomy::new(), CampaignEconomy::new()];
        let mut productions = CampaignProductions::new(2);
        let mut map = CampaignMap::generate(2, 42);

        let queued = queue_campaign_production(
            &mut economies,
            &mut productions,
            0,
            SpriteId::Thrall as u16,
            1,
            true,
        );
        assert!(queued);
        assert_eq!(economies[0].strain, 0.0, "Queueing should not add strain immediately");

        for _ in 0..300 {
            campaign_production_tick(&mut economies, &mut productions, &mut map, 0.05);
        }

        assert!(
            economies[0].strain >= CAMPAIGN_THRALL_STRAIN_AMOUNT,
            "Thrall completion should add campaign strain",
        );
    }

    #[test]
    fn high_strain_slows_campaign_production() {
        let mut economies = vec![CampaignEconomy::new(), CampaignEconomy::new()];
        let mut productions = CampaignProductions::new(2);
        let mut map = CampaignMap::generate(2, 42);

        economies[0].strain = 90.0;
        let queued = queue_campaign_production(
            &mut economies,
            &mut productions,
            0,
            SpriteId::Thrall as u16,
            1,
            true,
        );
        assert!(queued);

        for _ in 0..300 {
            campaign_production_tick(&mut economies, &mut productions, &mut map, 0.05);
        }

        let active_job = productions.0[0].active_job.as_ref();
        assert!(active_job.is_some(), "High strain should keep the Thrall in production after 15 seconds");
    }
}

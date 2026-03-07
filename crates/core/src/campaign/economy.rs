use serde::{Serialize, Deserialize};
use super::map::CampaignMap;

/// Campaign-level economy for a single player.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CampaignEconomy {
    /// Current energy bank.
    pub energy_bank: f32,
    /// Base income from node (5 energy/s).
    pub node_income: f32,
    /// Income from owned mining stations (8 energy/s each).
    pub mine_income: f32,
    /// Income from owned relic sites (3 energy/s each).
    pub relic_income: f32,
    /// Total garrison upkeep cost per second.
    pub garrison_upkeep: f32,
    /// Total deployed (in-battle) upkeep cost per second.
    pub deployed_upkeep: f32,
    /// Research spending per second (active research drain).
    pub research_spending: f32,
    /// Conscription strain level (0-100).
    pub strain: f32,
}

impl CampaignEconomy {
    pub fn new() -> Self {
        CampaignEconomy {
            energy_bank: 300.0, // Starting energy
            node_income: 5.0,
            mine_income: 0.0,
            relic_income: 0.0,
            garrison_upkeep: 0.0,
            deployed_upkeep: 0.0,
            research_spending: 0.0,
            strain: 0.0,
        }
    }

    /// Compute total income per second.
    pub fn total_income(&self) -> f32 {
        let base = self.node_income + self.mine_income + self.relic_income;
        let strain_penalty = compute_strain_penalty(self.strain);
        base * (1.0 - strain_penalty)
    }

    /// Compute total expenses per second.
    pub fn total_expenses(&self) -> f32 {
        self.garrison_upkeep + self.deployed_upkeep + self.research_spending
    }

    /// Net income rate (income - expenses).
    pub fn net_rate(&self) -> f32 {
        self.total_income() - self.total_expenses()
    }

    /// Add conscription strain (capped at 100).
    pub fn add_conscription_strain(&mut self, amount: f32) {
        self.strain = (self.strain + amount).min(100.0);
    }
}

/// Compute the strain-based income penalty (0.0 to 0.5+).
pub fn compute_strain_penalty(strain: f32) -> f32 {
    if strain < 30.0 {
        0.0
    } else if strain < 50.0 {
        // 0% to 15%
        let t = (strain - 30.0) / 20.0;
        t * 0.15
    } else if strain < 70.0 {
        // 15% to 30%
        let t = (strain - 50.0) / 20.0;
        0.15 + t * 0.15
    } else if strain < 90.0 {
        // 30% to 50%
        let t = (strain - 70.0) / 20.0;
        0.30 + t * 0.20
    } else {
        // 50%+
        0.50 + (strain - 90.0) * 0.005
    }
}

/// Update campaign income based on owned sites.
pub fn compute_campaign_income(map: &CampaignMap, player_id: u8) -> (f32, f32, f32) {
    let mut node_income = 0.0;
    let mut mine_income = 0.0;
    let mut relic_income = 0.0;

    for site in &map.sites {
        if site.owner == player_id && !site.is_contested {
            match site.site_type {
                super::map::SiteType::Node => node_income += 5.0,
                super::map::SiteType::MiningStation => mine_income += 8.0,
                super::map::SiteType::RelicSite => relic_income += 3.0,
            }
        }
    }

    (node_income, mine_income, relic_income)
}

/// Compute garrison upkeep for a player's garrisoned units.
/// Garrisoned units cost 50% of their deployed upkeep.
pub fn compute_garrison_upkeep(map: &CampaignMap, player_id: u8) -> f32 {
    let mut total = 0.0;
    for site in &map.sites {
        if site.owner == player_id {
            for gu in &site.garrison {
                let bp = crate::blueprints::get_blueprint(
                    crate::types::SpriteId::from_u16(gu.unit_type).unwrap_or(crate::types::SpriteId::Thrall)
                );
                total += bp.garrisoned_upkeep * gu.count as f32;
            }
        }
    }
    total
}

/// Campaign resource tick: update economies based on owned sites.
pub fn campaign_resource_tick(economies: &mut [CampaignEconomy], map: &CampaignMap, delta_secs: f32) {
    for (player_id, econ) in economies.iter_mut().enumerate() {
        let (node_inc, mine_inc, relic_inc) = compute_campaign_income(map, player_id as u8);
        econ.node_income = node_inc;
        econ.mine_income = mine_inc;
        econ.relic_income = relic_inc;

        econ.garrison_upkeep = compute_garrison_upkeep(map, player_id as u8);

        let net = econ.net_rate();
        econ.energy_bank += net * delta_secs;
        if econ.energy_bank < 0.0 {
            econ.energy_bank = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::map::{CampaignMap, SiteType};

    #[test]
    fn test_node_base_income() {
        let map = CampaignMap::generate(2, 42);
        let (node_inc, mine_inc, relic_inc) = compute_campaign_income(&map, 0);
        assert_eq!(node_inc, 5.0, "Node should give 5 energy/s");
        assert_eq!(mine_inc, 0.0, "No mines owned initially");
        assert_eq!(relic_inc, 0.0, "No relics owned initially");
    }

    #[test]
    fn test_mine_income() {
        let mut map = CampaignMap::generate(2, 42);

        // Give player 0 a mine
        let mine_id = map.sites.iter()
            .find(|s| s.site_type == SiteType::MiningStation)
            .unwrap().id;
        map.get_site_mut(mine_id).unwrap().owner = 0;

        let (_, mine_inc, _) = compute_campaign_income(&map, 0);
        assert_eq!(mine_inc, 8.0, "Mine should give 8 energy/s");
    }

    #[test]
    fn test_relic_income() {
        let mut map = CampaignMap::generate(2, 42);

        // Give player 0 a relic
        let relic_id = map.sites.iter()
            .find(|s| s.site_type == SiteType::RelicSite)
            .unwrap().id;
        map.get_site_mut(relic_id).unwrap().owner = 0;

        let (_, _, relic_inc) = compute_campaign_income(&map, 0);
        assert_eq!(relic_inc, 3.0, "Relic should give 3 energy/s");
    }

    #[test]
    fn test_combined_income() {
        let mut map = CampaignMap::generate(2, 42);

        // Give player 0 two mines and one relic
        let mines: Vec<u32> = map.sites.iter()
            .filter(|s| s.site_type == SiteType::MiningStation)
            .take(2)
            .map(|s| s.id)
            .collect();
        for id in mines {
            map.get_site_mut(id).unwrap().owner = 0;
        }

        let relic_id = map.sites.iter()
            .find(|s| s.site_type == SiteType::RelicSite)
            .unwrap().id;
        map.get_site_mut(relic_id).unwrap().owner = 0;

        let (node_inc, mine_inc, relic_inc) = compute_campaign_income(&map, 0);
        assert_eq!(node_inc, 5.0);
        assert_eq!(mine_inc, 16.0); // 2 mines * 8
        assert_eq!(relic_inc, 3.0);
    }

    #[test]
    fn test_garrison_upkeep() {
        let map = CampaignMap::generate(2, 42);
        let upkeep = compute_garrison_upkeep(&map, 0);
        // Node garrison: 10 Thralls + 3 Sentinels + 1 HoverTank
        // Garrisoned upkeep: Thrall=0.1, Sentinel=0.3, HoverTank=0.8
        let expected = 10.0 * 0.1 + 3.0 * 0.3 + 1.0 * 0.8;
        assert!((upkeep - expected).abs() < 0.01,
            "Garrison upkeep should be {}, got {}", expected, upkeep);
    }

    #[test]
    fn test_strain_penalty() {
        assert_eq!(compute_strain_penalty(0.0), 0.0);
        assert_eq!(compute_strain_penalty(29.0), 0.0);
        assert!(compute_strain_penalty(40.0) > 0.0);
        assert!(compute_strain_penalty(60.0) > compute_strain_penalty(40.0));
        assert!(compute_strain_penalty(80.0) > compute_strain_penalty(60.0));
        assert!(compute_strain_penalty(95.0) >= 0.50);
    }

    #[test]
    fn test_net_rate_calculation() {
        let mut econ = CampaignEconomy::new();
        econ.node_income = 5.0;
        econ.mine_income = 16.0;
        econ.garrison_upkeep = 5.5;
        let net = econ.net_rate();
        assert!((net - (21.0 - 5.5)).abs() < 0.01, "Net rate should be 15.5, got {}", net);
    }

    #[test]
    fn test_bank_drains_on_negative() {
        let mut economies = vec![CampaignEconomy::new(), CampaignEconomy::new()];
        economies[0].energy_bank = 10.0;
        economies[0].deployed_upkeep = 200.0; // Way more than income to guarantee drain

        let map = CampaignMap::generate(2, 42);
        campaign_resource_tick(&mut economies, &map, 1.0);

        assert_eq!(economies[0].energy_bank, 0.0, "Bank should not go below 0");
    }

    #[test]
    fn test_losing_mine_reduces_income() {
        let mut map = CampaignMap::generate(2, 42);

        let mine_id = map.sites.iter()
            .find(|s| s.site_type == SiteType::MiningStation)
            .unwrap().id;
        map.get_site_mut(mine_id).unwrap().owner = 0;

        let (_, mine_before, _) = compute_campaign_income(&map, 0);
        assert_eq!(mine_before, 8.0);

        // Lose the mine
        map.get_site_mut(mine_id).unwrap().owner = 255;

        let (_, mine_after, _) = compute_campaign_income(&map, 0);
        assert_eq!(mine_after, 0.0, "Losing mine should reduce income");
    }
}

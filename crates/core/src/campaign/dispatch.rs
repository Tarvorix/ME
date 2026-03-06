use serde::{Serialize, Deserialize};
use super::map::{CampaignMap, GarrisonedUnit};

/// A force dispatch order moving units between sites.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DispatchOrder {
    /// Unique order identifier.
    pub order_id: u32,
    /// Player who issued the dispatch.
    pub player: u8,
    /// Units being dispatched.
    pub units: Vec<GarrisonedUnit>,
    /// Source site ID.
    pub source_site: u32,
    /// Target site ID.
    pub target_site: u32,
    /// Remaining travel time in seconds.
    pub travel_remaining: f32,
    /// Total travel time for the journey.
    pub total_time: f32,
}

/// Active dispatch orders for all players.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DispatchQueue {
    pub orders: Vec<DispatchOrder>,
    next_order_id: u32,
}

impl DispatchQueue {
    pub fn new() -> Self {
        DispatchQueue {
            orders: Vec::new(),
            next_order_id: 0,
        }
    }

    /// Dispatch a force from one site to another.
    /// Removes units from source garrison and creates a dispatch order.
    /// Returns the order ID or None if invalid.
    pub fn dispatch_force(
        &mut self,
        map: &mut CampaignMap,
        player: u8,
        source_site: u32,
        target_site: u32,
        units: Vec<GarrisonedUnit>,
    ) -> Option<u32> {
        // Validate source site ownership
        let source = map.get_site(source_site)?;
        if source.owner != player {
            return None;
        }

        // Validate units are available at source
        for requested in &units {
            let available = source.garrison.iter()
                .find(|g| g.unit_type == requested.unit_type)
                .map(|g| g.count)
                .unwrap_or(0);
            if available < requested.count {
                return None; // Insufficient units
            }
        }

        // Calculate travel time
        let travel_time = map.travel_time(source_site, target_site);

        // Remove units from source garrison
        let source_mut = map.get_site_mut(source_site)?;
        for requested in &units {
            if let Some(g) = source_mut.garrison.iter_mut()
                .find(|g| g.unit_type == requested.unit_type)
            {
                g.count -= requested.count;
            }
        }
        // Clean up empty entries
        source_mut.garrison.retain(|g| g.count > 0);

        let order_id = self.next_order_id;
        self.next_order_id += 1;

        self.orders.push(DispatchOrder {
            order_id,
            player,
            units,
            source_site,
            target_site,
            travel_remaining: travel_time,
            total_time: travel_time,
        });

        Some(order_id)
    }

    /// Tick all active dispatch orders. Returns completed orders.
    pub fn tick(&mut self, delta_secs: f32) -> Vec<DispatchOrder> {
        let mut completed = Vec::new();

        self.orders.retain_mut(|order| {
            order.travel_remaining -= delta_secs;
            if order.travel_remaining <= 0.0 {
                completed.push(order.clone());
                false // Remove from active orders
            } else {
                true
            }
        });

        completed
    }

    /// Process a completed dispatch order: add units to target site or trigger battle.
    /// Returns true if a battle should be triggered.
    pub fn process_arrival(
        map: &mut CampaignMap,
        order: &DispatchOrder,
    ) -> bool {
        let target = match map.get_site(order.target_site) {
            Some(s) => s,
            None => return false,
        };

        let target_owner = target.owner;

        if target_owner == 255 {
            // Neutral site: claim it
            let target_mut = map.get_site_mut(order.target_site).unwrap();
            target_mut.owner = order.player;
            for unit in &order.units {
                super::garrison::add_to_garrison(&mut target_mut.garrison, unit.clone());
            }
            false
        } else if target_owner == order.player {
            // Own site: reinforce garrison
            let target_mut = map.get_site_mut(order.target_site).unwrap();
            for unit in &order.units {
                super::garrison::add_to_garrison(&mut target_mut.garrison, unit.clone());
            }
            false
        } else {
            // Enemy site: trigger battle
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::map::{CampaignMap, SiteType};

    #[test]
    fn test_dispatch_removes_from_source() {
        let mut map = CampaignMap::generate(2, 42);
        let mut queue = DispatchQueue::new();

        let node_id = map.player_nodes[0];
        let count_before = map.get_site(node_id).unwrap().garrison_count();

        // Dispatch 5 thralls from node to a mine
        let mine_id = map.sites.iter()
            .find(|s| s.site_type == SiteType::MiningStation)
            .unwrap().id;

        let result = queue.dispatch_force(
            &mut map, 0, node_id, mine_id,
            vec![GarrisonedUnit::new(0, 5)],
        );
        assert!(result.is_some());

        let count_after = map.get_site(node_id).unwrap().garrison_count();
        assert_eq!(count_after, count_before - 5, "Source should lose dispatched units");
    }

    #[test]
    fn test_travel_time_proportional_to_distance() {
        let mut map = CampaignMap::generate(2, 42);
        let mut queue = DispatchQueue::new();

        let node_id = map.player_nodes[0];
        let mine_id = map.sites.iter()
            .find(|s| s.site_type == SiteType::MiningStation)
            .unwrap().id;

        let expected_time = map.travel_time(node_id, mine_id);

        queue.dispatch_force(
            &mut map, 0, node_id, mine_id,
            vec![GarrisonedUnit::new(0, 2)],
        );

        assert!((queue.orders[0].travel_remaining - expected_time).abs() < 0.01);
    }

    #[test]
    fn test_arrival_adds_to_neutral_target() {
        let mut map = CampaignMap::generate(2, 42);
        let mut queue = DispatchQueue::new();

        let node_id = map.player_nodes[0];
        let mine_id = map.sites.iter()
            .find(|s| s.site_type == SiteType::MiningStation)
            .unwrap().id;

        queue.dispatch_force(
            &mut map, 0, node_id, mine_id,
            vec![GarrisonedUnit::new(0, 3)],
        );

        // Fast-forward to arrival
        let completed = queue.tick(1000.0);
        assert_eq!(completed.len(), 1);

        // Process arrival (neutral site → claim)
        let battle = DispatchQueue::process_arrival(&mut map, &completed[0]);
        assert!(!battle, "Should not trigger battle at neutral site");

        let mine = map.get_site(mine_id).unwrap();
        assert_eq!(mine.owner, 0, "Mine should be claimed by player 0");
        assert_eq!(mine.garrison_count(), 3, "Should have 3 garrisoned units");
    }

    #[test]
    fn test_enemy_arrival_triggers_battle() {
        let mut map = CampaignMap::generate(2, 42);
        let mut queue = DispatchQueue::new();

        // Give player 0 a mine
        let mine_id = map.sites.iter()
            .find(|s| s.site_type == SiteType::MiningStation)
            .unwrap().id;
        map.get_site_mut(mine_id).unwrap().owner = 0;

        // Player 1 dispatches force to that mine
        let node1_id = map.player_nodes[1];
        queue.dispatch_force(
            &mut map, 1, node1_id, mine_id,
            vec![GarrisonedUnit::new(0, 5)],
        );

        let completed = queue.tick(1000.0);
        let battle = DispatchQueue::process_arrival(&mut map, &completed[0]);
        assert!(battle, "Should trigger battle at enemy site");
    }

    #[test]
    fn test_insufficient_units_error() {
        let mut map = CampaignMap::generate(2, 42);
        let mut queue = DispatchQueue::new();

        let node_id = map.player_nodes[0];
        let mine_id = map.sites.iter()
            .find(|s| s.site_type == SiteType::MiningStation)
            .unwrap().id;

        // Try to dispatch 100 thralls (only have 10)
        let result = queue.dispatch_force(
            &mut map, 0, node_id, mine_id,
            vec![GarrisonedUnit::new(0, 100)],
        );
        assert!(result.is_none(), "Should fail with insufficient units");
    }

    #[test]
    fn test_own_site_reinforcement() {
        let mut map = CampaignMap::generate(2, 42);
        let mut queue = DispatchQueue::new();

        // Give player 0 a mine with some garrison
        let mine_id = map.sites.iter()
            .find(|s| s.site_type == SiteType::MiningStation)
            .unwrap().id;
        map.get_site_mut(mine_id).unwrap().owner = 0;
        map.get_site_mut(mine_id).unwrap().garrison.push(GarrisonedUnit::new(0, 2));

        // Dispatch more units to own site
        let node_id = map.player_nodes[0];
        queue.dispatch_force(
            &mut map, 0, node_id, mine_id,
            vec![GarrisonedUnit::new(0, 3)],
        );

        let completed = queue.tick(1000.0);
        let battle = DispatchQueue::process_arrival(&mut map, &completed[0]);
        assert!(!battle, "Should not trigger battle at own site");

        let mine = map.get_site(mine_id).unwrap();
        assert_eq!(mine.garrison_count(), 5, "Should have 2 + 3 = 5 garrisoned units");
    }
}

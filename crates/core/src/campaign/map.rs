use serde::{Serialize, Deserialize};

/// Types of sites on the campaign map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SiteType {
    /// Player home base. Production hub. If destroyed, player is eliminated.
    Node,
    /// Resource extraction site. Generates 8 energy/s when owned.
    MiningStation,
    /// Ancient technology cache. Generates 3 energy/s and gates research tiers.
    RelicSite,
}

/// A unit garrison entry for a campaign site.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GarrisonedUnit {
    /// Unit type (SpriteId as u16).
    pub unit_type: u16,
    /// Number of units of this type.
    pub count: u32,
    /// Average health percentage (0.0 - 1.0).
    pub health_pct: f32,
}

impl GarrisonedUnit {
    pub fn new(unit_type: u16, count: u32) -> Self {
        GarrisonedUnit {
            unit_type,
            count,
            health_pct: 1.0,
        }
    }

    pub fn with_health(unit_type: u16, count: u32, health_pct: f32) -> Self {
        GarrisonedUnit {
            unit_type,
            count,
            health_pct: health_pct.clamp(0.0, 1.0),
        }
    }
}

/// A site on the campaign map.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CampaignSite {
    /// Unique site identifier.
    pub id: u32,
    /// Type of site.
    pub site_type: SiteType,
    /// X coordinate on campaign map.
    pub x: f32,
    /// Y coordinate on campaign map.
    pub y: f32,
    /// Owning player (255 = neutral/unowned).
    pub owner: u8,
    /// Units garrisoned at this site.
    pub garrison: Vec<GarrisonedUnit>,
    /// Whether a battle is currently happening at this site.
    pub is_contested: bool,
    /// Active battle ID if contested (None otherwise).
    pub battle_id: Option<u32>,
}

impl CampaignSite {
    pub fn new(id: u32, site_type: SiteType, x: f32, y: f32) -> Self {
        CampaignSite {
            id,
            site_type,
            x,
            y,
            owner: 255, // neutral
            garrison: Vec::new(),
            is_contested: false,
            battle_id: None,
        }
    }

    pub fn with_owner(mut self, owner: u8) -> Self {
        self.owner = owner;
        self
    }

    /// Returns true if the site is unowned (neutral).
    pub fn is_neutral(&self) -> bool {
        self.owner == 255
    }

    /// Returns the total number of garrisoned units.
    pub fn garrison_count(&self) -> u32 {
        self.garrison.iter().map(|g| g.count).sum()
    }
}

/// The full campaign map state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CampaignMap {
    /// All sites on the map.
    pub sites: Vec<CampaignSite>,
    /// Player node site IDs (index = player_id).
    pub player_nodes: Vec<u32>,
    /// Map width in campaign units.
    pub width: f32,
    /// Map height in campaign units.
    pub height: f32,
}

impl CampaignMap {
    /// Generate a campaign map for the given player count.
    ///
    /// Layout:
    /// - Nodes at corners (one per player)
    /// - Mining stations spread in no-man's land between bases
    /// - Relic sites placed centrally
    pub fn generate(player_count: u8, seed: u64) -> Self {
        let width = 100.0;
        let height = 100.0;
        let margin = 10.0;
        let mut sites = Vec::new();
        let mut player_nodes = Vec::new();
        let mut next_id = 0u32;

        // Place nodes at corners
        let node_positions = match player_count {
            2 => vec![
                (margin, margin),
                (width - margin, height - margin),
            ],
            3 => vec![
                (margin, margin),
                (width - margin, height - margin),
                (width - margin, margin),
            ],
            _ => vec![
                (margin, margin),
                (width - margin, height - margin),
                (width - margin, margin),
                (margin, height - margin),
            ],
        };

        for (i, &(fx, fy)) in node_positions.iter().enumerate() {
            let mut site = CampaignSite::new(next_id, SiteType::Node, fx, fy)
                .with_owner(i as u8);
            // Each node starts with a default garrison: 10 Thralls + 3 Sentinels + 1 HoverTank
            site.garrison.push(GarrisonedUnit::new(0, 10)); // Thralls
            site.garrison.push(GarrisonedUnit::new(1, 3));  // Sentinels
            site.garrison.push(GarrisonedUnit::new(2, 1));  // HoverTank
            player_nodes.push(next_id);
            sites.push(site);
            next_id += 1;
        }

        // Deterministic pseudo-random for placement
        let mut rng = SimpleRng::new(seed);

        // Place mining stations (6-8 depending on player count)
        let mine_count = if player_count <= 2 { 6 } else { 8 };
        let mine_positions = generate_spread_positions(
            &mut rng,
            mine_count,
            width,
            height,
            margin + 5.0,
            &sites,
            8.0, // minimum distance between sites
        );

        for (mx, my) in mine_positions {
            sites.push(CampaignSite::new(next_id, SiteType::MiningStation, mx, my));
            next_id += 1;
        }

        // Place relic sites (2-3 centrally)
        let relic_count = if player_count <= 2 { 2 } else { 3 };
        let center_margin = 25.0;
        let relic_positions = generate_spread_positions(
            &mut rng,
            relic_count,
            width,
            height,
            center_margin,
            &sites,
            10.0,
        );

        for (rx, ry) in relic_positions {
            sites.push(CampaignSite::new(next_id, SiteType::RelicSite, rx, ry));
            next_id += 1;
        }

        CampaignMap {
            sites,
            player_nodes,
            width,
            height,
        }
    }

    /// Get a site by its ID.
    pub fn get_site(&self, site_id: u32) -> Option<&CampaignSite> {
        self.sites.iter().find(|s| s.id == site_id)
    }

    /// Get a mutable reference to a site by its ID.
    pub fn get_site_mut(&mut self, site_id: u32) -> Option<&mut CampaignSite> {
        self.sites.iter_mut().find(|s| s.id == site_id)
    }

    /// Get the node site for a player.
    pub fn get_node(&self, player_id: u8) -> Option<&CampaignSite> {
        self.player_nodes.get(player_id as usize)
            .and_then(|&id| self.get_site(id))
    }

    /// Get a mutable node site for a player.
    pub fn get_node_mut(&mut self, player_id: u8) -> Option<&mut CampaignSite> {
        let node_id = self.player_nodes.get(player_id as usize).copied();
        node_id.and_then(move |id| self.get_site_mut(id))
    }

    /// Calculate Euclidean distance between two sites.
    pub fn distance(&self, site_a: u32, site_b: u32) -> f32 {
        let a = match self.get_site(site_a) {
            Some(s) => s,
            None => return f32::MAX,
        };
        let b = match self.get_site(site_b) {
            Some(s) => s,
            None => return f32::MAX,
        };
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Calculate travel time between two sites (distance / speed).
    /// Base travel speed is 2.5 campaign units per second.
    pub fn travel_time(&self, site_a: u32, site_b: u32) -> f32 {
        const TRAVEL_SPEED: f32 = 2.5;
        self.distance(site_a, site_b) / TRAVEL_SPEED
    }

    /// Get all sites owned by a player.
    pub fn sites_owned_by(&self, player_id: u8) -> Vec<&CampaignSite> {
        self.sites.iter().filter(|s| s.owner == player_id).collect()
    }

    /// Get all neutral (unowned) sites.
    pub fn neutral_sites(&self) -> Vec<&CampaignSite> {
        self.sites.iter().filter(|s| s.is_neutral()).collect()
    }

    /// Count mining stations owned by a player.
    pub fn count_mines(&self, player_id: u8) -> u32 {
        self.sites.iter()
            .filter(|s| s.owner == player_id && s.site_type == SiteType::MiningStation)
            .count() as u32
    }

    /// Count relic sites owned by a player.
    pub fn count_relics(&self, player_id: u8) -> u32 {
        self.sites.iter()
            .filter(|s| s.owner == player_id && s.site_type == SiteType::RelicSite)
            .count() as u32
    }
}

/// Simple deterministic pseudo-random number generator (xorshift64).
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        SimpleRng {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Returns a float in [0.0, 1.0).
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() % 10000) as f32 / 10000.0
    }

    /// Returns a float in [min, max).
    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
}

/// Generate spread-out positions for sites, avoiding existing sites.
fn generate_spread_positions(
    rng: &mut SimpleRng,
    count: u32,
    map_width: f32,
    map_height: f32,
    margin: f32,
    existing_sites: &[CampaignSite],
    min_distance: f32,
) -> Vec<(f32, f32)> {
    let mut positions = Vec::new();
    let max_attempts = 100;

    for _ in 0..count {
        let mut best_pos = (map_width / 2.0, map_height / 2.0);
        let mut best_min_dist = 0.0f32;

        for _ in 0..max_attempts {
            let x = rng.range_f32(margin, map_width - margin);
            let y = rng.range_f32(margin, map_height - margin);

            // Check distance from existing sites
            let mut min_dist = f32::MAX;
            for site in existing_sites {
                let dx = x - site.x;
                let dy = y - site.y;
                let d = (dx * dx + dy * dy).sqrt();
                min_dist = min_dist.min(d);
            }

            // Check distance from already-placed positions
            for &(px, py) in &positions {
                let dx: f32 = x - px;
                let dy: f32 = y - py;
                let d = (dx * dx + dy * dy).sqrt();
                min_dist = min_dist.min(d);
            }

            if min_dist >= min_distance && min_dist > best_min_dist {
                best_min_dist = min_dist;
                best_pos = (x, y);
            }
        }

        positions.push(best_pos);
    }

    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_2p_map() {
        let map = CampaignMap::generate(2, 42);
        assert_eq!(map.player_nodes.len(), 2);
        assert_eq!(map.width, 100.0);
        assert_eq!(map.height, 100.0);

        // Should have 2 nodes + 6 mines + 2 relics = 10 sites
        assert_eq!(map.sites.len(), 10);

        // Nodes should be owned
        let node0 = map.get_site(map.player_nodes[0]).unwrap();
        assert_eq!(node0.owner, 0);
        assert_eq!(node0.site_type, SiteType::Node);

        let node1 = map.get_site(map.player_nodes[1]).unwrap();
        assert_eq!(node1.owner, 1);
        assert_eq!(node1.site_type, SiteType::Node);
    }

    #[test]
    fn test_generate_4p_map() {
        let map = CampaignMap::generate(4, 42);
        assert_eq!(map.player_nodes.len(), 4);

        // Should have 4 nodes + 8 mines + 3 relics = 15 sites
        assert_eq!(map.sites.len(), 15);

        // All nodes should be owned by different players
        for i in 0..4 {
            let node = map.get_site(map.player_nodes[i]).unwrap();
            assert_eq!(node.owner, i as u8);
            assert_eq!(node.site_type, SiteType::Node);
        }
    }

    #[test]
    fn test_nodes_at_corners() {
        let map = CampaignMap::generate(2, 42);

        let node0 = map.get_site(map.player_nodes[0]).unwrap();
        let node1 = map.get_site(map.player_nodes[1]).unwrap();

        // Node 0 near bottom-left corner
        assert!(node0.x < 50.0, "Node 0 x={} should be in left half", node0.x);
        assert!(node0.y < 50.0, "Node 0 y={} should be in bottom half", node0.y);

        // Node 1 near top-right corner
        assert!(node1.x > 50.0, "Node 1 x={} should be in right half", node1.x);
        assert!(node1.y > 50.0, "Node 1 y={} should be in top half", node1.y);
    }

    #[test]
    fn test_mines_between_bases() {
        let map = CampaignMap::generate(2, 42);

        let mines: Vec<&CampaignSite> = map.sites.iter()
            .filter(|s| s.site_type == SiteType::MiningStation)
            .collect();

        assert_eq!(mines.len(), 6);

        // All mines should be neutral
        for mine in &mines {
            assert!(mine.is_neutral(), "Mine {} should be neutral", mine.id);
        }
    }

    #[test]
    fn test_relics_central() {
        let map = CampaignMap::generate(2, 42);

        let relics: Vec<&CampaignSite> = map.sites.iter()
            .filter(|s| s.site_type == SiteType::RelicSite)
            .collect();

        assert_eq!(relics.len(), 2);

        // Relics should be in the middle area (within center_margin=25 from edges)
        for relic in &relics {
            assert!(relic.x >= 25.0 && relic.x <= 75.0,
                "Relic x={} should be in center area", relic.x);
            assert!(relic.y >= 25.0 && relic.y <= 75.0,
                "Relic y={} should be in center area", relic.y);
            assert!(relic.is_neutral(), "Relic should be neutral");
        }
    }

    #[test]
    fn test_distance_symmetric() {
        let map = CampaignMap::generate(2, 42);

        let site_a = map.sites[0].id;
        let site_b = map.sites[1].id;

        let d_ab = map.distance(site_a, site_b);
        let d_ba = map.distance(site_b, site_a);

        assert!((d_ab - d_ba).abs() < 0.001, "Distance should be symmetric: {} vs {}", d_ab, d_ba);
        assert!(d_ab > 0.0, "Distance between different sites should be positive");
    }

    #[test]
    fn test_travel_time_proportional() {
        let map = CampaignMap::generate(2, 42);

        let site_a = map.sites[0].id;
        let site_b = map.sites[1].id;

        let dist = map.distance(site_a, site_b);
        let time = map.travel_time(site_a, site_b);

        // travel_time = distance / 2.5
        assert!((time - dist / 2.5).abs() < 0.001,
            "Travel time should be distance/speed: time={}, dist={}", time, dist);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let map = CampaignMap::generate(2, 42);

        let serialized = serde_json::to_string(&map).expect("serialize");
        let deserialized: CampaignMap = serde_json::from_str(&serialized).expect("deserialize");

        assert_eq!(deserialized.sites.len(), map.sites.len());
        assert_eq!(deserialized.player_nodes.len(), map.player_nodes.len());
        assert_eq!(deserialized.width, map.width);
        assert_eq!(deserialized.height, map.height);

        // Check site data preserved
        for (orig, deser) in map.sites.iter().zip(deserialized.sites.iter()) {
            assert_eq!(orig.id, deser.id);
            assert_eq!(orig.site_type, deser.site_type);
            assert!((orig.x - deser.x).abs() < 0.001);
            assert!((orig.y - deser.y).abs() < 0.001);
            assert_eq!(orig.owner, deser.owner);
        }
    }

    #[test]
    fn test_garrison_tracking() {
        let mut site = CampaignSite::new(0, SiteType::Node, 10.0, 10.0);
        assert_eq!(site.garrison_count(), 0);

        site.garrison.push(GarrisonedUnit::new(0, 10)); // 10 Thralls
        site.garrison.push(GarrisonedUnit::new(1, 3));  // 3 Sentinels
        assert_eq!(site.garrison_count(), 13);

        // Check health tracking
        let damaged = GarrisonedUnit::with_health(0, 5, 0.6);
        assert_eq!(damaged.count, 5);
        assert!((damaged.health_pct - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_initial_ownership() {
        let map = CampaignMap::generate(2, 42);

        // Nodes should be owned
        let owned = map.sites_owned_by(0);
        assert_eq!(owned.len(), 1, "Player 0 should own 1 site (node)");
        assert_eq!(owned[0].site_type, SiteType::Node);

        let owned1 = map.sites_owned_by(1);
        assert_eq!(owned1.len(), 1, "Player 1 should own 1 site (node)");

        // All other sites should be neutral
        let neutrals = map.neutral_sites();
        assert_eq!(neutrals.len(), 8, "8 sites should be neutral (6 mines + 2 relics)");
    }

    #[test]
    fn test_sites_spread() {
        let map = CampaignMap::generate(2, 42);

        // All non-node sites should be spread apart (at least 8 units)
        for i in 0..map.sites.len() {
            for j in (i + 1)..map.sites.len() {
                let dist = map.distance(map.sites[i].id, map.sites[j].id);
                assert!(dist >= 5.0,
                    "Sites {} and {} too close: {} units",
                    map.sites[i].id, map.sites[j].id, dist);
            }
        }
    }

    #[test]
    fn test_count_mines_and_relics() {
        let map = CampaignMap::generate(2, 42);

        // Initially no mines/relics owned
        assert_eq!(map.count_mines(0), 0);
        assert_eq!(map.count_mines(1), 0);
        assert_eq!(map.count_relics(0), 0);
        assert_eq!(map.count_relics(1), 0);
    }

    #[test]
    fn test_node_starting_garrison() {
        let map = CampaignMap::generate(2, 42);

        let node = map.get_node(0).unwrap();
        assert_eq!(node.garrison.len(), 3, "Node should have 3 garrison entries");
        assert_eq!(node.garrison_count(), 14, "Node should have 14 total units (10T + 3S + 1HT)");

        // Check unit types
        assert_eq!(node.garrison[0].unit_type, 0); // Thralls
        assert_eq!(node.garrison[0].count, 10);
        assert_eq!(node.garrison[1].unit_type, 1); // Sentinels
        assert_eq!(node.garrison[1].count, 3);
        assert_eq!(node.garrison[2].unit_type, 2); // HoverTank
        assert_eq!(node.garrison[2].count, 1);
    }

    #[test]
    fn test_get_node() {
        let map = CampaignMap::generate(2, 42);

        let node0 = map.get_node(0);
        assert!(node0.is_some());
        assert_eq!(node0.unwrap().owner, 0);

        let node_none = map.get_node(5); // invalid player
        assert!(node_none.is_none());
    }
}

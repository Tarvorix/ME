use crate::ecs::World;
use crate::components::{Position, UnitType, Health};
use crate::systems::resource::Economies;
use crate::systems::production::Productions;
use crate::types::SpriteId;
use crate::blueprints;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// How often the strategic AI updates (in game ticks).
pub const STRATEGIC_UPDATE_INTERVAL: u32 = 40; // 2 seconds at 20Hz

/// Number of MCTS iterations per planning cycle.
pub const MCTS_ITERATIONS: u32 = 200;

/// Maximum rollout depth (simulated turns).
pub const MCTS_ROLLOUT_DEPTH: u32 = 20;

/// UCB1 exploration constant.
const UCB1_C: f64 = std::f64::consts::SQRT_2;

/// Army strength weights per unit type.
const THRALL_STRENGTH: f32 = 1.0;
const SENTINEL_STRENGTH: f32 = 3.0;
const HOVER_TANK_STRENGTH: f32 = 7.0;

/// Number of sectors per dimension (8x8 grid).
const SECTORS_PER_DIM: u32 = 8;
/// Total number of sectors.
const TOTAL_SECTORS: usize = (SECTORS_PER_DIM * SECTORS_PER_DIM) as usize;

/// High-level strategic actions the AI can take.
#[derive(Clone, Debug, PartialEq)]
pub enum StrategicAction {
    /// Produce a Thrall infantry unit.
    ProduceThrall,
    /// Produce a Sentinel elite unit.
    ProduceSentinel,
    /// Produce a Hover Tank.
    ProduceHoverTank,
    /// Attack-move all combat units to a specific sector.
    AttackSector(u8),
    /// Move combat units to defend a specific sector.
    DefendSector(u8),
    /// Retreat all combat units toward Command Post.
    Retreat,
    /// Do nothing this planning cycle.
    DoNothing,
}

/// Per-player simplified economy state for MCTS simulation.
#[derive(Clone, Debug)]
pub struct MctsEconomy {
    pub energy_bank: f32,
    pub income: f32,
    pub upkeep: f32,
    pub strain: f32,
}

/// Per-sector unit counts for a player.
#[derive(Clone, Debug, Default)]
pub struct SectorUnitCounts {
    pub thralls: u16,
    pub sentinels: u16,
    pub hover_tanks: u16,
}

impl SectorUnitCounts {
    /// Calculate the total combat strength of units in this sector.
    pub fn strength(&self) -> f32 {
        self.thralls as f32 * THRALL_STRENGTH
            + self.sentinels as f32 * SENTINEL_STRENGTH
            + self.hover_tanks as f32 * HOVER_TANK_STRENGTH
    }

    /// Total unit count in this sector.
    pub fn total(&self) -> u16 {
        self.thralls + self.sentinels + self.hover_tanks
    }
}

/// Simplified game state for MCTS tree search.
#[derive(Clone, Debug)]
pub struct MctsState {
    /// Number of players.
    pub player_count: u8,
    /// Map dimensions (used for sector calculations).
    pub map_width: u32,
    pub map_height: u32,
    /// Per-player per-sector unit counts. [player][sector_index]
    pub sector_units: Vec<Vec<SectorUnitCounts>>,
    /// Per-player economy state.
    pub economies: Vec<MctsEconomy>,
    /// Per-player node alive status.
    pub node_alive: Vec<bool>,
    /// Per-player command post sector index.
    pub cp_sector: Vec<Option<u8>>,
    /// Per-player: infantry production line busy.
    pub infantry_line_busy: Vec<bool>,
    /// Per-player: armor production line busy.
    pub armor_line_busy: Vec<bool>,
}

impl MctsState {
    /// Get the sector index for a given tile position.
    pub fn sector_index(&self, x: f32, y: f32) -> u8 {
        let sector_w = self.map_width as f32 / SECTORS_PER_DIM as f32;
        let sector_h = self.map_height as f32 / SECTORS_PER_DIM as f32;
        let sx = (x / sector_w).floor().min((SECTORS_PER_DIM - 1) as f32).max(0.0) as u8;
        let sy = (y / sector_h).floor().min((SECTORS_PER_DIM - 1) as f32).max(0.0) as u8;
        sy * SECTORS_PER_DIM as u8 + sx
    }

    /// Get the center position of a sector.
    pub fn sector_center(&self, sector: u8) -> (f32, f32) {
        let sx = (sector % SECTORS_PER_DIM as u8) as f32;
        let sy = (sector / SECTORS_PER_DIM as u8) as f32;
        let sector_w = self.map_width as f32 / SECTORS_PER_DIM as f32;
        let sector_h = self.map_height as f32 / SECTORS_PER_DIM as f32;
        (sx * sector_w + sector_w / 2.0, sy * sector_h + sector_h / 2.0)
    }

    /// Total army strength for a player across all sectors.
    pub fn total_strength(&self, player_id: u8) -> f32 {
        let pid = player_id as usize;
        if pid >= self.sector_units.len() {
            return 0.0;
        }
        self.sector_units[pid].iter().map(|s| s.strength()).sum()
    }

    /// Total unit count for a player.
    pub fn total_units(&self, player_id: u8) -> u32 {
        let pid = player_id as usize;
        if pid >= self.sector_units.len() {
            return 0;
        }
        self.sector_units[pid].iter().map(|s| s.total() as u32).sum()
    }

    /// Find the sector with the highest enemy strength relative to a player.
    pub fn highest_enemy_sector(&self, player_id: u8) -> Option<u8> {
        let mut best_sector = None;
        let mut best_strength = 0.0f32;

        for sector_idx in 0..TOTAL_SECTORS {
            let mut enemy_strength = 0.0f32;
            for pid in 0..self.player_count as usize {
                if pid == player_id as usize {
                    continue;
                }
                enemy_strength += self.sector_units[pid][sector_idx].strength();
            }
            if enemy_strength > best_strength {
                best_strength = enemy_strength;
                best_sector = Some(sector_idx as u8);
            }
        }

        best_sector
    }

    /// Find the sector with the most friendly units for a player.
    pub fn strongest_friendly_sector(&self, player_id: u8) -> Option<u8> {
        let pid = player_id as usize;
        if pid >= self.sector_units.len() {
            return None;
        }
        let mut best_sector = None;
        let mut best_strength = 0.0f32;

        for sector_idx in 0..TOTAL_SECTORS {
            let strength = self.sector_units[pid][sector_idx].strength();
            if strength > best_strength {
                best_strength = strength;
                best_sector = Some(sector_idx as u8);
            }
        }

        best_sector
    }
}

/// A node in the MCTS tree.
#[derive(Clone, Debug)]
struct MctsNode {
    /// The action taken to reach this node.
    action: StrategicAction,
    /// Parent node index (None for root).
    parent: Option<usize>,
    /// Child node indices.
    children: Vec<usize>,
    /// Number of times this node was visited.
    visits: u32,
    /// Total accumulated value.
    total_value: f64,
    /// Actions not yet expanded from this node.
    untried_actions: Vec<StrategicAction>,
}

impl MctsNode {
    fn new(action: StrategicAction, parent: Option<usize>) -> Self {
        MctsNode {
            action,
            parent,
            children: Vec::new(),
            visits: 0,
            total_value: 0.0,
            untried_actions: Vec::new(),
        }
    }

    /// UCB1 value for this node.
    fn ucb1(&self, parent_visits: u32) -> f64 {
        if self.visits == 0 {
            return f64::MAX;
        }
        let exploitation = self.total_value / self.visits as f64;
        let exploration = UCB1_C * ((parent_visits as f64).ln() / self.visits as f64).sqrt();
        exploitation + exploration
    }

    /// Average value of this node.
    #[allow(dead_code)]
    fn average_value(&self) -> f64 {
        if self.visits == 0 {
            return 0.0;
        }
        self.total_value / self.visits as f64
    }
}

/// Monte Carlo Tree Search planner for strategic AI decisions.
pub struct MctsPlanner {
    /// Random number generator for rollouts.
    rng: SmallRng,
}

impl MctsPlanner {
    /// Create a new MCTS planner with the given seed.
    pub fn new(seed: u64) -> Self {
        MctsPlanner {
            rng: SmallRng::seed_from_u64(seed),
        }
    }

    /// Extract a simplified MctsState from the game world.
    pub fn extract_state(world: &World, player_count: u8, map_width: u32, map_height: u32) -> MctsState {
        let mut sector_units: Vec<Vec<SectorUnitCounts>> = (0..player_count)
            .map(|_| (0..TOTAL_SECTORS).map(|_| SectorUnitCounts::default()).collect())
            .collect();

        let mut economies: Vec<MctsEconomy> = Vec::new();
        let mut node_alive: Vec<bool> = vec![false; player_count as usize];
        let mut cp_sector: Vec<Option<u8>> = vec![None; player_count as usize];
        let mut infantry_line_busy: Vec<bool> = vec![false; player_count as usize];
        let mut armor_line_busy: Vec<bool> = vec![false; player_count as usize];

        let sector_w = map_width as f32 / SECTORS_PER_DIM as f32;
        let sector_h = map_height as f32 / SECTORS_PER_DIM as f32;

        // Collect unit positions into sectors
        if let (Some(pos_s), Some(ut_s)) = (
            world.get_storage::<Position>(),
            world.get_storage::<UnitType>(),
        ) {
            let health_s = world.get_storage::<Health>();

            for (entity, pos) in pos_s.iter() {
                let ut = match ut_s.get(entity) {
                    Some(ut) => ut,
                    None => continue,
                };

                // Skip dead units
                if let Some(hs) = &health_s {
                    if let Some(h) = hs.get(entity) {
                        if h.current <= 0.0 {
                            continue;
                        }
                    }
                }

                let pid = ut.owner as usize;
                if pid >= player_count as usize {
                    continue;
                }

                let sx = (pos.x / sector_w).floor().min((SECTORS_PER_DIM - 1) as f32).max(0.0) as usize;
                let sy = (pos.y / sector_h).floor().min((SECTORS_PER_DIM - 1) as f32).max(0.0) as usize;
                let sector_idx = sy * SECTORS_PER_DIM as usize + sx;

                match ut.kind {
                    SpriteId::Thrall => sector_units[pid][sector_idx].thralls += 1,
                    SpriteId::Sentinel => sector_units[pid][sector_idx].sentinels += 1,
                    SpriteId::HoverTank => sector_units[pid][sector_idx].hover_tanks += 1,
                    SpriteId::CommandPost => {
                        cp_sector[pid] = Some(sector_idx as u8);
                    }
                    SpriteId::Node => {
                        node_alive[pid] = true;
                    }
                    SpriteId::CapturePoint => {
                        // Capture points are neutral objectives, not counted per player
                    }
                }
            }
        }

        // Extract economy
        if let Some(econs) = world.get_resource::<Economies>() {
            for pid in 0..player_count as usize {
                if pid < econs.0.len() {
                    let e = &econs.0[pid];
                    let gross_income = e.base_income + e.mining_income + e.relic_income;
                    let penalty = e.strain_income_penalty();
                    let net_income = gross_income * (1.0 - penalty);
                    economies.push(MctsEconomy {
                        energy_bank: e.energy_bank,
                        income: net_income,
                        upkeep: e.production_spending,
                        strain: e.conscription_strain,
                    });
                } else {
                    economies.push(MctsEconomy {
                        energy_bank: 0.0,
                        income: 0.0,
                        upkeep: 0.0,
                        strain: 0.0,
                    });
                }
            }
        } else {
            for _ in 0..player_count {
                economies.push(MctsEconomy {
                    energy_bank: 0.0,
                    income: 0.0,
                    upkeep: 0.0,
                    strain: 0.0,
                });
            }
        }

        // Check production line status
        if let Some(prods) = world.get_resource::<Productions>() {
            for pid in 0..player_count as usize {
                if pid < prods.0.len() {
                    let prod = &prods.0[pid];
                    infantry_line_busy[pid] = prod.infantry_lines.iter().all(|l| l.is_some());
                    armor_line_busy[pid] = prod.armor_lines.iter().all(|l| l.is_some());
                }
            }
        }

        MctsState {
            player_count,
            map_width,
            map_height,
            sector_units,
            economies,
            node_alive,
            cp_sector,
            infantry_line_busy,
            armor_line_busy,
        }
    }

    /// Get legal actions for a player in the given state.
    pub fn get_legal_actions(state: &MctsState, player_id: u8) -> Vec<StrategicAction> {
        let pid = player_id as usize;
        let mut actions = Vec::new();

        // Always can do nothing
        actions.push(StrategicAction::DoNothing);

        if pid >= state.economies.len() {
            return actions;
        }

        let econ = &state.economies[pid];
        let thrall_bp = blueprints::get_blueprint(SpriteId::Thrall);
        let sentinel_bp = blueprints::get_blueprint(SpriteId::Sentinel);
        let hover_tank_bp = blueprints::get_blueprint(SpriteId::HoverTank);

        // Production actions — only if we can afford and a line is free
        if !state.infantry_line_busy[pid] {
            if econ.energy_bank >= thrall_bp.energy_cost as f32 {
                actions.push(StrategicAction::ProduceThrall);
            }
            if econ.energy_bank >= sentinel_bp.energy_cost as f32 {
                actions.push(StrategicAction::ProduceSentinel);
            }
        }
        if !state.armor_line_busy[pid] {
            if econ.energy_bank >= hover_tank_bp.energy_cost as f32 {
                actions.push(StrategicAction::ProduceHoverTank);
            }
        }

        // Attack/defend sectors where there are enemy units
        let has_combat_units = state.total_units(player_id) > 0;
        if has_combat_units {
            // Retreat to CP
            if state.cp_sector[pid].is_some() {
                actions.push(StrategicAction::Retreat);
            }

            // Attack sectors with enemy presence
            for sector_idx in 0..TOTAL_SECTORS {
                let mut enemy_present = false;
                for other_pid in 0..state.player_count as usize {
                    if other_pid == pid {
                        continue;
                    }
                    if state.sector_units[other_pid][sector_idx].total() > 0 {
                        enemy_present = true;
                        break;
                    }
                }
                if enemy_present {
                    actions.push(StrategicAction::AttackSector(sector_idx as u8));
                }
            }

            // Defend sectors with own presence or CP/node
            if let Some(cp_sec) = state.cp_sector[pid] {
                actions.push(StrategicAction::DefendSector(cp_sec));
            }
        }

        actions
    }

    /// Evaluate a game state from a player's perspective. Returns 0.0 to 1.0.
    /// 50% army strength ratio, 30% economy health, 20% base survival.
    pub fn evaluate_state(state: &MctsState, player_id: u8) -> f64 {
        let pid = player_id as usize;

        // Base survival: 20% — node alive is critical
        let base_score = if pid < state.node_alive.len() && state.node_alive[pid] {
            1.0
        } else {
            0.0
        };

        // If our node is dead, overall score is 0
        if base_score == 0.0 {
            return 0.0;
        }

        // Army strength ratio: 50%
        let own_strength = state.total_strength(player_id) as f64;
        let mut enemy_strength = 0.0f64;
        for other_pid in 0..state.player_count {
            if other_pid == player_id {
                continue;
            }
            enemy_strength += state.total_strength(other_pid) as f64;
        }

        let army_score = if own_strength + enemy_strength > 0.0 {
            own_strength / (own_strength + enemy_strength)
        } else {
            0.5 // No units on either side — neutral
        };

        // Economy health: 30%
        let econ_score = if pid < state.economies.len() {
            let econ = &state.economies[pid];
            let bank_score = (econ.energy_bank as f64 / 1000.0).min(1.0);
            let income_score = (econ.income as f64 / 10.0).min(1.0);
            let strain_penalty = (econ.strain as f64 / 100.0).min(1.0);
            (bank_score * 0.4 + income_score * 0.4) * (1.0 - strain_penalty * 0.5) + 0.2 * (1.0 - strain_penalty)
        } else {
            0.0
        };

        // Weighted combination
        army_score * 0.5 + econ_score * 0.3 + base_score * 0.2
    }

    /// Choose the best strategic action using MCTS.
    pub fn choose_action(
        &mut self,
        state: &MctsState,
        player_id: u8,
        iterations: u32,
    ) -> StrategicAction {
        let legal_actions = Self::get_legal_actions(state, player_id);

        // If only DoNothing is available, return it immediately
        if legal_actions.len() <= 1 {
            return StrategicAction::DoNothing;
        }

        // Initialize tree with root node
        let mut nodes = Vec::new();
        let mut root = MctsNode::new(StrategicAction::DoNothing, None);
        root.untried_actions = legal_actions;
        nodes.push(root);

        for _ in 0..iterations {
            let mut node_idx = 0;

            // 1. Selection — navigate to a promising leaf
            while nodes[node_idx].untried_actions.is_empty() && !nodes[node_idx].children.is_empty() {
                let parent_visits = nodes[node_idx].visits;
                let best_child = nodes[node_idx]
                    .children
                    .iter()
                    .max_by(|&&a, &&b| {
                        nodes[a].ucb1(parent_visits)
                            .partial_cmp(&nodes[b].ucb1(parent_visits))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
                    .unwrap();
                node_idx = best_child;
            }

            // 2. Expansion — add a child for an untried action
            if !nodes[node_idx].untried_actions.is_empty() {
                let action_idx = self.rng.gen_range(0..nodes[node_idx].untried_actions.len());
                let action = nodes[node_idx].untried_actions.remove(action_idx);

                let child_idx = nodes.len();
                let mut child = MctsNode::new(action.clone(), Some(node_idx));

                // Apply the action to get a new state, then get legal actions for that state
                let new_state = self.simulate_action(state, player_id, &action);
                child.untried_actions = Self::get_legal_actions(&new_state, player_id);

                nodes.push(child);
                nodes[node_idx].children.push(child_idx);
                node_idx = child_idx;
            }

            // 3. Rollout — random playout from this state
            let mut rollout_state = state.clone();
            // Apply the actions along the path to this node
            let path_actions = self.get_path_actions(node_idx, &nodes);
            for action in &path_actions {
                rollout_state = self.simulate_action(&rollout_state, player_id, action);
            }

            let value = self.rollout(&rollout_state, player_id);

            // 4. Backpropagation — update all ancestors
            let mut current = node_idx;
            loop {
                nodes[current].visits += 1;
                nodes[current].total_value += value;
                match nodes[current].parent {
                    Some(parent) => current = parent,
                    None => break,
                }
            }
        }

        // Select the best action (most visited child of root)
        let root_children = &nodes[0].children;
        if root_children.is_empty() {
            return StrategicAction::DoNothing;
        }

        let best_child_idx = root_children
            .iter()
            .max_by_key(|&&idx| nodes[idx].visits)
            .copied()
            .unwrap();

        nodes[best_child_idx].action.clone()
    }

    /// Get the sequence of actions from root to the given node.
    fn get_path_actions(&self, node_idx: usize, nodes: &[MctsNode]) -> Vec<StrategicAction> {
        let mut path = Vec::new();
        let mut current = node_idx;
        while let Some(parent) = nodes[current].parent {
            path.push(nodes[current].action.clone());
            current = parent;
        }
        path.reverse();
        path
    }

    /// Simulate a single action on the state, returning a new state.
    fn simulate_action(&self, state: &MctsState, player_id: u8, action: &StrategicAction) -> MctsState {
        let mut new_state = state.clone();
        let pid = player_id as usize;

        match action {
            StrategicAction::ProduceThrall => {
                let cost = blueprints::get_blueprint(SpriteId::Thrall).energy_cost as f32;
                if pid < new_state.economies.len() && new_state.economies[pid].energy_bank >= cost {
                    new_state.economies[pid].energy_bank -= cost;
                    // Add a thrall at the CP sector
                    if let Some(cp) = new_state.cp_sector[pid] {
                        new_state.sector_units[pid][cp as usize].thralls += 1;
                    }
                    new_state.infantry_line_busy[pid] = true;
                    // Strain increases for conscripts
                    new_state.economies[pid].strain += 5.0;
                }
            }
            StrategicAction::ProduceSentinel => {
                let cost = blueprints::get_blueprint(SpriteId::Sentinel).energy_cost as f32;
                if pid < new_state.economies.len() && new_state.economies[pid].energy_bank >= cost {
                    new_state.economies[pid].energy_bank -= cost;
                    if let Some(cp) = new_state.cp_sector[pid] {
                        new_state.sector_units[pid][cp as usize].sentinels += 1;
                    }
                    new_state.infantry_line_busy[pid] = true;
                }
            }
            StrategicAction::ProduceHoverTank => {
                let cost = blueprints::get_blueprint(SpriteId::HoverTank).energy_cost as f32;
                if pid < new_state.economies.len() && new_state.economies[pid].energy_bank >= cost {
                    new_state.economies[pid].energy_bank -= cost;
                    if let Some(cp) = new_state.cp_sector[pid] {
                        new_state.sector_units[pid][cp as usize].hover_tanks += 1;
                    }
                    new_state.armor_line_busy[pid] = true;
                }
            }
            StrategicAction::AttackSector(target_sector) => {
                // Simplified: move all units toward the target sector (proportional combat)
                let target = *target_sector as usize;
                // Gather all combat units and move them to the target sector
                let mut total_thralls = 0u16;
                let mut total_sentinels = 0u16;
                let mut total_tanks = 0u16;
                for sector in new_state.sector_units[pid].iter_mut() {
                    total_thralls += sector.thralls;
                    total_sentinels += sector.sentinels;
                    total_tanks += sector.hover_tanks;
                    sector.thralls = 0;
                    sector.sentinels = 0;
                    sector.hover_tanks = 0;
                }
                if target < TOTAL_SECTORS {
                    new_state.sector_units[pid][target].thralls = total_thralls;
                    new_state.sector_units[pid][target].sentinels = total_sentinels;
                    new_state.sector_units[pid][target].hover_tanks = total_tanks;

                    // Simulate combat in the target sector
                    self.simulate_sector_combat(&mut new_state, pid, target);
                }
            }
            StrategicAction::DefendSector(sector) => {
                // Move all units to the defense sector
                let target = *sector as usize;
                let mut total_thralls = 0u16;
                let mut total_sentinels = 0u16;
                let mut total_tanks = 0u16;
                for s in new_state.sector_units[pid].iter_mut() {
                    total_thralls += s.thralls;
                    total_sentinels += s.sentinels;
                    total_tanks += s.hover_tanks;
                    s.thralls = 0;
                    s.sentinels = 0;
                    s.hover_tanks = 0;
                }
                if target < TOTAL_SECTORS {
                    new_state.sector_units[pid][target].thralls = total_thralls;
                    new_state.sector_units[pid][target].sentinels = total_sentinels;
                    new_state.sector_units[pid][target].hover_tanks = total_tanks;
                }
            }
            StrategicAction::Retreat => {
                // Move all units to CP sector
                if let Some(cp) = new_state.cp_sector[pid] {
                    let target = cp as usize;
                    let mut total_thralls = 0u16;
                    let mut total_sentinels = 0u16;
                    let mut total_tanks = 0u16;
                    for s in new_state.sector_units[pid].iter_mut() {
                        total_thralls += s.thralls;
                        total_sentinels += s.sentinels;
                        total_tanks += s.hover_tanks;
                        s.thralls = 0;
                        s.sentinels = 0;
                        s.hover_tanks = 0;
                    }
                    new_state.sector_units[pid][target].thralls = total_thralls;
                    new_state.sector_units[pid][target].sentinels = total_sentinels;
                    new_state.sector_units[pid][target].hover_tanks = total_tanks;
                }
            }
            StrategicAction::DoNothing => {}
        }

        // Simulate one economy tick for all players
        for p in 0..new_state.player_count as usize {
            if p < new_state.economies.len() {
                let net = new_state.economies[p].income - new_state.economies[p].upkeep;
                new_state.economies[p].energy_bank = (new_state.economies[p].energy_bank + net).max(0.0);
                // Strain decay
                new_state.economies[p].strain = (new_state.economies[p].strain * 0.98).max(0.0);
            }
        }

        new_state
    }

    /// Simulate combat proportionally in a sector.
    /// Both sides lose units proportional to the other's DPS.
    fn simulate_sector_combat(&self, state: &mut MctsState, attacker_pid: usize, sector: usize) {
        let attacker_strength = state.sector_units[attacker_pid][sector].strength();

        // Calculate total defender strength in this sector
        let mut total_defender_strength = 0.0f32;
        for pid in 0..state.player_count as usize {
            if pid == attacker_pid {
                continue;
            }
            total_defender_strength += state.sector_units[pid][sector].strength();
        }

        if total_defender_strength <= 0.0 || attacker_strength <= 0.0 {
            return;
        }

        // Proportional losses: each side loses strength proportional to the enemy's ratio
        let attacker_ratio = attacker_strength / (attacker_strength + total_defender_strength);
        let defender_loss_ratio = attacker_ratio * 0.3; // 30% of defender strength lost per engagement
        let attacker_loss_ratio = (1.0 - attacker_ratio) * 0.3;

        // Apply losses to attacker
        {
            let units = &mut state.sector_units[attacker_pid][sector];
            let loss = (units.total() as f32 * attacker_loss_ratio).ceil() as u16;
            self.apply_losses(units, loss);
        }

        // Apply losses to each defender in this sector
        for pid in 0..state.player_count as usize {
            if pid == attacker_pid {
                continue;
            }
            let defender_strength_in_sector = state.sector_units[pid][sector].strength();
            if defender_strength_in_sector <= 0.0 {
                continue;
            }
            let share = defender_strength_in_sector / total_defender_strength;
            let loss = (state.sector_units[pid][sector].total() as f32 * defender_loss_ratio * share).ceil() as u16;
            let units = &mut state.sector_units[pid][sector];
            self.apply_losses(units, loss);
        }
    }

    /// Apply unit losses, removing cheapest units first (thralls, then sentinels, then tanks).
    fn apply_losses(&self, units: &mut SectorUnitCounts, mut losses: u16) {
        // Remove thralls first (cheapest)
        let thrall_lost = losses.min(units.thralls);
        units.thralls -= thrall_lost;
        losses -= thrall_lost;

        // Then sentinels
        let sentinel_lost = losses.min(units.sentinels);
        units.sentinels -= sentinel_lost;
        losses -= sentinel_lost;

        // Then hover tanks
        let tank_lost = losses.min(units.hover_tanks);
        units.hover_tanks -= tank_lost;
    }

    /// Perform a random rollout from the given state and return an evaluation score.
    fn rollout(&mut self, state: &MctsState, player_id: u8) -> f64 {
        let mut rollout_state = state.clone();

        for _ in 0..MCTS_ROLLOUT_DEPTH {
            // Check if game is over (any node destroyed)
            let player_alive = rollout_state.node_alive[player_id as usize];
            if !player_alive {
                return 0.0;
            }

            let all_enemies_dead = (0..rollout_state.player_count)
                .filter(|&p| p != player_id)
                .all(|p| !rollout_state.node_alive[p as usize]);
            if all_enemies_dead {
                return 1.0;
            }

            // Each player takes a random action
            for pid in 0..rollout_state.player_count {
                let actions = Self::get_legal_actions(&rollout_state, pid);
                if actions.is_empty() {
                    continue;
                }
                let action_idx = self.rng.gen_range(0..actions.len());
                let action = actions[action_idx].clone();
                rollout_state = self.simulate_action(&rollout_state, pid, &action);
            }
        }

        Self::evaluate_state(&rollout_state, player_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, GameConfig};

    fn test_game() -> Game {
        Game::new(GameConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
        })
    }

    #[test]
    fn test_mcts_state_extraction() {
        let mut game = test_game();

        // Spawn units for both players
        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_thrall(9.5, 8.5, 0);
        game.spawn_unit(SpriteId::Sentinel, 10.5, 8.5, 0);
        game.spawn_thrall(55.5, 55.5, 1);

        // Spawn nodes and CPs
        game.spawn_node(5.0, 5.0, 0);
        game.spawn_node(60.0, 60.0, 1);
        game.spawn_command_post(8.0, 8.0, 0);
        game.spawn_command_post(55.0, 55.0, 1);

        game.tick(50.0);

        let state = MctsPlanner::extract_state(&game.world, 2, 64, 64);

        assert_eq!(state.player_count, 2);
        assert_eq!(state.map_width, 64);
        assert_eq!(state.map_height, 64);

        // Player 0 should have 2 thralls + 1 sentinel
        assert_eq!(state.total_units(0), 3);
        // Player 1 should have 1 thrall
        assert_eq!(state.total_units(1), 1);

        // Both nodes should be alive
        assert!(state.node_alive[0]);
        assert!(state.node_alive[1]);

        // CP sectors should be set
        assert!(state.cp_sector[0].is_some());
        assert!(state.cp_sector[1].is_some());
    }

    #[test]
    fn test_mcts_legal_actions() {
        let mut game = test_game();
        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_thrall(55.5, 55.5, 1);
        game.spawn_node(5.0, 5.0, 0);
        game.spawn_node(60.0, 60.0, 1);
        game.spawn_command_post(8.0, 8.0, 0);
        game.spawn_command_post(55.0, 55.0, 1);

        game.tick(50.0);

        let state = MctsPlanner::extract_state(&game.world, 2, 64, 64);
        let actions = MctsPlanner::get_legal_actions(&state, 0);

        // Should have at minimum: DoNothing, ProduceThrall, ProduceSentinel, Retreat,
        // AttackSector (where player 1's unit is), DefendSector (CP sector)
        assert!(actions.contains(&StrategicAction::DoNothing));
        assert!(actions.contains(&StrategicAction::ProduceThrall));
        assert!(actions.contains(&StrategicAction::ProduceSentinel));
        assert!(actions.contains(&StrategicAction::Retreat));
        assert!(actions.len() >= 5, "Should have at least 5 legal actions, got {}", actions.len());
    }

    #[test]
    fn test_mcts_no_production_when_broke() {
        let mut game = test_game();
        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_node(5.0, 5.0, 0);
        game.spawn_command_post(8.0, 8.0, 0);

        game.tick(50.0);

        let mut state = MctsPlanner::extract_state(&game.world, 2, 64, 64);

        // Drain all energy
        state.economies[0].energy_bank = 0.0;

        let actions = MctsPlanner::get_legal_actions(&state, 0);

        // Should NOT have production actions
        assert!(!actions.contains(&StrategicAction::ProduceThrall));
        assert!(!actions.contains(&StrategicAction::ProduceSentinel));
        assert!(!actions.contains(&StrategicAction::ProduceHoverTank));
    }

    #[test]
    fn test_mcts_evaluation_winning() {
        let mut state = MctsState {
            player_count: 2,
            map_width: 64,
            map_height: 64,
            sector_units: vec![
                (0..TOTAL_SECTORS).map(|_| SectorUnitCounts::default()).collect(),
                (0..TOTAL_SECTORS).map(|_| SectorUnitCounts::default()).collect(),
            ],
            economies: vec![
                MctsEconomy { energy_bank: 500.0, income: 5.0, upkeep: 1.0, strain: 0.0 },
                MctsEconomy { energy_bank: 100.0, income: 2.0, upkeep: 1.0, strain: 50.0 },
            ],
            node_alive: vec![true, true],
            cp_sector: vec![Some(0), Some(63)],
            infantry_line_busy: vec![false, false],
            armor_line_busy: vec![false, false],
        };

        // Player 0 has 10 thralls, player 1 has 2
        state.sector_units[0][0].thralls = 10;
        state.sector_units[1][63].thralls = 2;

        let p0_score = MctsPlanner::evaluate_state(&state, 0);
        let p1_score = MctsPlanner::evaluate_state(&state, 1);

        // Player 0 should score higher (more army, better economy)
        assert!(p0_score > p1_score,
            "Player 0 (winning) should score higher: {} vs {}", p0_score, p1_score);
        assert!(p0_score > 0.5, "Winning player should score above 0.5: {}", p0_score);
    }

    #[test]
    fn test_mcts_evaluation_dead() {
        let state = MctsState {
            player_count: 2,
            map_width: 64,
            map_height: 64,
            sector_units: vec![
                (0..TOTAL_SECTORS).map(|_| SectorUnitCounts::default()).collect(),
                (0..TOTAL_SECTORS).map(|_| SectorUnitCounts::default()).collect(),
            ],
            economies: vec![
                MctsEconomy { energy_bank: 500.0, income: 5.0, upkeep: 1.0, strain: 0.0 },
                MctsEconomy { energy_bank: 500.0, income: 5.0, upkeep: 1.0, strain: 0.0 },
            ],
            node_alive: vec![false, true], // Player 0's node is dead
            cp_sector: vec![Some(0), Some(63)],
            infantry_line_busy: vec![false, false],
            armor_line_busy: vec![false, false],
        };

        let p0_score = MctsPlanner::evaluate_state(&state, 0);
        assert_eq!(p0_score, 0.0, "Dead player should score 0.0");
    }

    #[test]
    fn test_mcts_choose_action() {
        let mut game = test_game();
        game.spawn_thrall(8.5, 8.5, 0);
        game.spawn_thrall(9.5, 8.5, 0);
        game.spawn_thrall(55.5, 55.5, 1);
        game.spawn_node(5.0, 5.0, 0);
        game.spawn_node(60.0, 60.0, 1);
        game.spawn_command_post(8.0, 8.0, 0);
        game.spawn_command_post(55.0, 55.0, 1);

        game.tick(50.0);

        let state = MctsPlanner::extract_state(&game.world, 2, 64, 64);
        let mut planner = MctsPlanner::new(42);

        let action = planner.choose_action(&state, 0, 50); // Fewer iterations for test speed

        // Should return a valid action (not just DoNothing with good economy and units)
        let legal = MctsPlanner::get_legal_actions(&state, 0);
        assert!(legal.contains(&action),
            "Chosen action {:?} should be legal", action);
    }

    #[test]
    fn test_mcts_prefers_production() {
        // Test that MCTS tends to prefer production when energy is high and no immediate threat
        let state = MctsState {
            player_count: 2,
            map_width: 64,
            map_height: 64,
            sector_units: vec![
                (0..TOTAL_SECTORS).map(|_| SectorUnitCounts::default()).collect(),
                (0..TOTAL_SECTORS).map(|_| SectorUnitCounts::default()).collect(),
            ],
            economies: vec![
                MctsEconomy { energy_bank: 500.0, income: 10.0, upkeep: 0.0, strain: 0.0 },
                MctsEconomy { energy_bank: 500.0, income: 10.0, upkeep: 0.0, strain: 0.0 },
            ],
            node_alive: vec![true, true],
            cp_sector: vec![Some(0), Some(63)],
            infantry_line_busy: vec![false, false],
            armor_line_busy: vec![false, false],
        };

        // No units on either side
        // Run MCTS multiple times and count production actions
        let mut production_count = 0;
        let total_runs = 20;
        for i in 0..total_runs {
            let mut planner = MctsPlanner::new(i as u64);
            let action = planner.choose_action(&state, 0, 100);
            match action {
                StrategicAction::ProduceThrall |
                StrategicAction::ProduceSentinel |
                StrategicAction::ProduceHoverTank => production_count += 1,
                _ => {}
            }
        }

        // With no units and plenty of energy, MCTS should mostly choose production
        assert!(production_count > total_runs / 3,
            "MCTS should prefer production when economy is strong: {} out of {} runs",
            production_count, total_runs);
    }

    #[test]
    fn test_mcts_ucb1_exploration() {
        // Test UCB1 formula correctness
        let mut node = MctsNode::new(StrategicAction::DoNothing, None);
        node.visits = 10;
        node.total_value = 7.0; // 70% win rate

        let ucb1 = node.ucb1(100);

        // exploitation = 7/10 = 0.7
        // exploration = sqrt(2) * sqrt(ln(100)/10) ≈ 1.414 * sqrt(4.605/10) ≈ 1.414 * 0.679 ≈ 0.96
        assert!(ucb1 > 0.7, "UCB1 should be > exploitation value: {}", ucb1);
        assert!(ucb1 < 2.0, "UCB1 should be reasonable: {}", ucb1);

        // Unvisited node should have MAX UCB1
        let unvisited = MctsNode::new(StrategicAction::DoNothing, None);
        assert_eq!(unvisited.ucb1(100), f64::MAX);
    }

    #[test]
    fn test_sector_index() {
        let state = MctsState {
            player_count: 2,
            map_width: 64,
            map_height: 64,
            sector_units: vec![],
            economies: vec![],
            node_alive: vec![],
            cp_sector: vec![],
            infantry_line_busy: vec![],
            armor_line_busy: vec![],
        };

        // Top-left corner
        assert_eq!(state.sector_index(0.0, 0.0), 0);
        // Bottom-right corner
        assert_eq!(state.sector_index(63.0, 63.0), 63);
        // Center of map
        assert_eq!(state.sector_index(32.0, 32.0), 36); // sector (4, 4) = 4*8+4 = 36
    }

    #[test]
    fn test_sector_center() {
        let state = MctsState {
            player_count: 2,
            map_width: 64,
            map_height: 64,
            sector_units: vec![],
            economies: vec![],
            node_alive: vec![],
            cp_sector: vec![],
            infantry_line_busy: vec![],
            armor_line_busy: vec![],
        };

        // Sector 0 (top-left): center should be at (4.0, 4.0)
        let (cx, cy) = state.sector_center(0);
        assert!((cx - 4.0).abs() < 0.01);
        assert!((cy - 4.0).abs() < 0.01);

        // Sector 63 (bottom-right): center should be at (60.0, 60.0)
        let (cx, cy) = state.sector_center(63);
        assert!((cx - 60.0).abs() < 0.01);
        assert!((cy - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_highest_enemy_sector() {
        let mut state = MctsState {
            player_count: 2,
            map_width: 64,
            map_height: 64,
            sector_units: vec![
                (0..TOTAL_SECTORS).map(|_| SectorUnitCounts::default()).collect(),
                (0..TOTAL_SECTORS).map(|_| SectorUnitCounts::default()).collect(),
            ],
            economies: vec![],
            node_alive: vec![true, true],
            cp_sector: vec![Some(0), Some(63)],
            infantry_line_busy: vec![false, false],
            armor_line_busy: vec![false, false],
        };

        // Place enemy units in sector 30
        state.sector_units[1][30].thralls = 5;
        state.sector_units[1][30].sentinels = 2;

        let highest = state.highest_enemy_sector(0);
        assert_eq!(highest, Some(30));
    }
}

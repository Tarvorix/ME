/// Commands from the player (JS or AI) to the game simulation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Command {
    /// Move units to a target position via A* pathfinding.
    Move {
        unit_ids: Vec<u32>,
        target_x: f32,
        target_y: f32,
    },
    /// Stop units (clear path and target).
    Stop {
        unit_ids: Vec<u32>,
    },
    /// Attack a specific target entity.
    Attack {
        unit_ids: Vec<u32>,
        target_id: u32,
    },
    /// Move to a position, engaging enemies encountered along the way.
    AttackMove {
        unit_ids: Vec<u32>,
        target_x: f32,
        target_y: f32,
    },
    /// Place a building at a tile position.
    Build {
        player: u8,
        building_type: u16,
        tile_x: u32,
        tile_y: u32,
    },
    /// Queue unit production at the player's Node.
    Produce {
        player: u8,
        unit_type: u16,
    },
    /// Cancel an active production job on a specific line.
    CancelProduction {
        player: u8,
        line_index: u8,
    },
    /// Set the rally point for newly produced units.
    SetRally {
        player: u8,
        x: f32,
        y: f32,
    },
    /// Place Command Post during deployment phase.
    Deploy {
        player: u8,
        cp_x: f32,
        cp_y: f32,
    },
    /// Confirm deployment is ready. Battle starts when all players confirm.
    ConfirmDeployment {
        player: u8,
    },
    /// Upgrade a Node production line.
    UpgradeNode {
        player: u8,
        upgrade: u8,
    },
    /// Start campaign research.
    CampaignResearch {
        player: u8,
        tech_id: u8,
    },
    /// Dispatch forces between campaign sites.
    CampaignDispatch {
        player: u8,
        source_site: u32,
        target_site: u32,
        /// Encoded unit counts (unit_type:count pairs).
        units: Vec<(u16, u32)>,
    },
    /// Withdraw garrison from a campaign site.
    CampaignWithdraw {
        player: u8,
        site_id: u32,
    },
}

/// Resource holding pending commands to be processed next tick.
pub struct PendingCommands(pub Vec<Command>);

impl PendingCommands {
    pub fn new() -> Self {
        PendingCommands(Vec::new())
    }

    pub fn push(&mut self, cmd: Command) {
        self.0.push(cmd);
    }

    pub fn drain(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.0)
    }
}

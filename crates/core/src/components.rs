use crate::ecs::entity::Entity;
use crate::types::{AnimState, SpriteId};

/// Position in tile-space (floating point for sub-tile precision).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// Previous tick position for render interpolation.
pub struct PreviousPosition {
    pub x: f32,
    pub y: f32,
}

/// Unit type and ownership.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UnitType {
    pub kind: SpriteId,
    pub owner: u8,
}

/// Pathfinding state for a unit following a path.
pub struct PathState {
    pub path: Vec<(u32, u32)>,
    pub current_index: usize,
    pub speed: f32, // tiles per second
}

impl PathState {
    pub fn empty(speed: f32) -> Self {
        PathState {
            path: Vec::new(),
            current_index: 0,
            speed,
        }
    }

    pub fn has_path(&self) -> bool {
        !self.path.is_empty() && self.current_index < self.path.len()
    }

    pub fn clear(&mut self) {
        self.path.clear();
        self.current_index = 0;
    }
}

/// Render state mirrors what gets written to the shared render buffer.
pub struct RenderState {
    pub sprite_id: u16,
    pub frame: u16,
    pub facing: u8,
    pub health_pct: u8,
    pub flags: u8,      // bit 0 = selected, bit 1 = constructing, bits 2-3 = anim_state
    pub scale: f32,
    pub anim_state: AnimState,
    pub anim_timer: f32,
}

impl RenderState {
    pub fn new(sprite_id: SpriteId, scale: f32) -> Self {
        RenderState {
            sprite_id: sprite_id as u16,
            frame: 0,
            facing: 0,
            health_pct: 100,
            flags: 0,
            scale,
            anim_state: AnimState::Idle,
            anim_timer: 0.0,
        }
    }

    pub fn set_selected(&mut self, selected: bool) {
        if selected {
            self.flags |= 1;
        } else {
            self.flags &= !1;
        }
    }

    pub fn is_selected(&self) -> bool {
        self.flags & 1 != 0
    }

    /// Pack anim_state into flags bits 2-3.
    pub fn pack_flags(&mut self) {
        self.flags = (self.flags & 0b0000_0011) | ((self.anim_state as u8) << 2);
    }
}

/// Tag component marking an entity as selected.
pub struct Selected;

/// Health pool for a unit or building.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Health { current: max, max }
    }

    /// Returns health as a percentage (0–100).
    pub fn percent(&self) -> u8 {
        if self.max <= 0.0 {
            return 0;
        }
        ((self.current / self.max) * 100.0).clamp(0.0, 100.0) as u8
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }
}

/// Combat state tracking for units that can attack.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CombatState {
    pub target: Option<Entity>,
    pub attack_cooldown: f32,
    pub in_range: bool,
}

impl CombatState {
    pub fn new() -> Self {
        CombatState {
            target: None,
            attack_cooldown: 0.0,
            in_range: false,
        }
    }
}

/// Vision radius for fog of war computation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VisionRange(pub f32);

/// Whether a unit is deployed in an RTS battle (true) or garrisoned at home (false).
/// Affects upkeep cost calculation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Deployed(pub bool);

/// Timer counting down to entity despawn after death animation.
pub struct DeathTimer(pub f32);

/// Target position for attack-move commands.
/// Unit moves toward this position while engaging enemies in vision range.
pub struct AttackMoveTarget {
    pub x: f32,
    pub y: f32,
}

/// Capture point state for objective control mechanics.
/// Attached to CapturePoint entities on the battle map.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CapturePointState {
    /// Capture radius in tiles — units within this distance contribute to capture.
    pub capture_radius: f32,
    /// Base capture speed in progress points per second per unit.
    pub capture_speed: f32,
    /// Current owner player_id, or 255 for neutral/unowned.
    pub owner: u8,
    /// Capture progress 0.0 to 100.0. At 100.0 the point flips owner.
    pub progress: f32,
    /// Which player is currently capturing (255 = none).
    pub capturing_player: u8,
    /// Whether the point is contested (multiple players have units nearby).
    pub contested: bool,
    /// Index of this capture point (for deterministic ordering).
    pub point_index: u8,
}

impl CapturePointState {
    pub fn new(point_index: u8) -> Self {
        CapturePointState {
            capture_radius: 3.0,
            capture_speed: 5.0,
            owner: 255, // neutral
            progress: 0.0,
            capturing_player: 255,
            contested: false,
            point_index,
        }
    }

    /// Returns true if the capture point is owned by any player (not neutral).
    pub fn is_owned(&self) -> bool {
        self.owner != 255
    }

    /// Returns true if owned by the specified player.
    pub fn is_owned_by(&self, player: u8) -> bool {
        self.owner == player
    }
}

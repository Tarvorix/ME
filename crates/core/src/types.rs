/// 8-directional facing. Values match the render buffer `facing` byte
/// and the atlas direction order in manifest.json: S, SW, W, NW, N, NE, E, SE.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Direction {
    S = 0,
    SW = 1,
    W = 2,
    NW = 3,
    N = 4,
    NE = 5,
    E = 6,
    SE = 7,
}

impl Direction {
    /// Determine facing direction from a movement delta (dx, dy) in battle tile space.
    /// RTS battles now render on an orthogonal square grid, so facings should follow
    /// the direct world axes instead of the old isometric screen projection.
    pub fn from_delta(dx: f32, dy: f32) -> Self {
        if dx == 0.0 && dy == 0.0 {
            return Direction::S;
        }

        let angle = dy.atan2(dx);
        let angle = if angle < 0.0 { angle + std::f32::consts::TAU } else { angle };
        let sector = ((angle + std::f32::consts::FRAC_PI_8) / std::f32::consts::FRAC_PI_4) as u32 % 8;

        match sector {
            0 => Direction::E,
            1 => Direction::SE,
            2 => Direction::S,
            3 => Direction::SW,
            4 => Direction::W,
            5 => Direction::NW,
            6 => Direction::N,
            7 => Direction::NE,
            _ => Direction::S,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Direction::S => "S",
            Direction::SW => "SW",
            Direction::W => "W",
            Direction::NW => "NW",
            Direction::N => "N",
            Direction::NE => "NE",
            Direction::E => "E",
            Direction::SE => "SE",
        }
    }
}

/// Sprite type identifier. Maps to sprite atlases.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum SpriteId {
    Thrall = 0,
    Sentinel = 1,
    HoverTank = 2,
    CommandPost = 3,
    Node = 4,
    CapturePoint = 5,
}

impl SpriteId {
    pub fn from_u16(v: u16) -> Option<SpriteId> {
        match v {
            0 => Some(SpriteId::Thrall),
            1 => Some(SpriteId::Sentinel),
            2 => Some(SpriteId::HoverTank),
            3 => Some(SpriteId::CommandPost),
            4 => Some(SpriteId::Node),
            5 => Some(SpriteId::CapturePoint),
            _ => None,
        }
    }

    pub fn to_le_bytes(self) -> [u8; 2] {
        (self as u16).to_le_bytes()
    }
}

/// Animation state. Determines which animation set to play.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum AnimState {
    Idle = 0,
    Move = 1,
    Attack = 2,
    Death = 3,
}

impl AnimState {
    /// Frame duration in seconds for this animation state (from Art Bible).
    pub fn frame_duration(self) -> f32 {
        match self {
            AnimState::Idle => 0.2,
            AnimState::Move => 0.1,
            AnimState::Attack => 0.08,
            AnimState::Death => 0.12,
        }
    }

    /// Whether this animation loops.
    pub fn loops(self) -> bool {
        match self {
            AnimState::Idle | AnimState::Move => true,
            AnimState::Attack | AnimState::Death => false,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AnimState::Idle => "Idle",
            AnimState::Move => "Move",
            AnimState::Attack => "Shoot",
            AnimState::Death => "Death",
        }
    }
}

/// Returns the number of animation frames per direction for a given sprite + animation.
/// Based on manifest.json data.
pub fn get_frame_count(sprite_id: SpriteId, anim_state: AnimState) -> u16 {
    match (sprite_id, anim_state) {
        (SpriteId::Thrall, AnimState::Idle) => 4,
        (SpriteId::Thrall, AnimState::Move) => 6,
        (SpriteId::Thrall, AnimState::Attack) => 4,
        (SpriteId::Thrall, AnimState::Death) => 6,

        (SpriteId::Sentinel, AnimState::Idle) => 4,
        (SpriteId::Sentinel, AnimState::Move) => 8,
        (SpriteId::Sentinel, AnimState::Attack) => 6,
        (SpriteId::Sentinel, AnimState::Death) => 7,

        // Hover Tank, Command Post, Node, CapturePoint: static (1 frame per direction)
        (SpriteId::HoverTank, _) => 1,
        (SpriteId::CommandPost, _) => 1,
        (SpriteId::Node, _) => 1,
        (SpriteId::CapturePoint, _) => 1,
    }
}

/// Event types written to the event buffer for client-side feedback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum EventType {
    Shot = 0,
    Death = 1,
    UnitSpawned = 2,
    BuildingPlaced = 3,
    ProductionComplete = 4,
    CaptureProgress = 5,
    CaptureComplete = 6,
    BattleEnd = 7,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direction_from_delta() {
        // Orthogonal square-grid facings.
        assert_eq!(Direction::from_delta(1.0, 0.0), Direction::E);
        assert_eq!(Direction::from_delta(0.0, 1.0), Direction::S);
        assert_eq!(Direction::from_delta(-1.0, 0.0), Direction::W);
        assert_eq!(Direction::from_delta(0.0, -1.0), Direction::N);
        assert_eq!(Direction::from_delta(1.0, 1.0), Direction::SE);
        assert_eq!(Direction::from_delta(1.0, -1.0), Direction::NE);
        assert_eq!(Direction::from_delta(-1.0, 1.0), Direction::SW);
        assert_eq!(Direction::from_delta(-1.0, -1.0), Direction::NW);
    }
}

use crate::types::SpriteId;

/// Which production line a unit type uses at the Node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProductionLine {
    Infantry,
    Armor,
}

/// Static stat data for a unit or building type.
/// All values are from the Game Design Document.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UnitBlueprint {
    pub max_hp: f32,
    pub damage: f32,
    pub attack_range: f32,
    pub speed: f32,
    pub vision_range: f32,
    pub energy_cost: u32,
    pub build_time_secs: f32,
    pub garrisoned_upkeep: f32,
    pub deployed_upkeep: f32,
    pub is_conscript: bool,
    pub attack_cooldown: f32,
    pub scale: f32,
}

/// Returns the static blueprint for the given entity type.
pub fn get_blueprint(sprite_id: SpriteId) -> &'static UnitBlueprint {
    match sprite_id {
        SpriteId::Thrall => &THRALL_BLUEPRINT,
        SpriteId::Sentinel => &SENTINEL_BLUEPRINT,
        SpriteId::HoverTank => &HOVER_TANK_BLUEPRINT,
        SpriteId::CommandPost => &COMMAND_POST_BLUEPRINT,
        SpriteId::Node => &NODE_BLUEPRINT,
        SpriteId::CapturePoint => &CAPTURE_POINT_BLUEPRINT,
    }
}

/// Returns which production line the given unit type uses, or None for buildings.
pub fn production_line(sprite_id: SpriteId) -> Option<ProductionLine> {
    match sprite_id {
        SpriteId::Thrall | SpriteId::Sentinel => Some(ProductionLine::Infantry),
        SpriteId::HoverTank => Some(ProductionLine::Armor),
        SpriteId::CommandPost | SpriteId::Node | SpriteId::CapturePoint => None,
    }
}

// ── Thrall ──────────────────────────────────────────────────────────────────
// Conscripted infantry. Cheap, fast, causes Conscription Strain.
static THRALL_BLUEPRINT: UnitBlueprint = UnitBlueprint {
    max_hp: 80.0,
    damage: 8.0,
    attack_range: 5.0,
    speed: 3.0,
    vision_range: 8.0,
    energy_cost: 30,
    build_time_secs: 15.0,
    garrisoned_upkeep: 0.1,
    deployed_upkeep: 0.3,
    is_conscript: true,
    attack_cooldown: 0.5,
    scale: 48.0 / 512.0,
};

// ── Sentinel ────────────────────────────────────────────────────────────────
// Elite cyborg infantry. Expensive, tough, no strain.
static SENTINEL_BLUEPRINT: UnitBlueprint = UnitBlueprint {
    max_hp: 200.0,
    damage: 25.0,
    attack_range: 5.0,
    speed: 2.0,
    vision_range: 8.0,
    energy_cost: 120,
    build_time_secs: 45.0,
    garrisoned_upkeep: 0.3,
    deployed_upkeep: 0.8,
    is_conscript: false,
    attack_cooldown: 0.8,
    scale: 56.0 / 512.0,
};

// ── Hover Tank ──────────────────────────────────────────────────────────────
// Heavy armored vehicle. Very expensive, ignores terrain penalties.
static HOVER_TANK_BLUEPRINT: UnitBlueprint = UnitBlueprint {
    max_hp: 500.0,
    damage: 60.0,
    attack_range: 8.0,
    speed: 2.5,
    vision_range: 10.0,
    energy_cost: 300,
    build_time_secs: 90.0,
    garrisoned_upkeep: 0.8,
    deployed_upkeep: 2.0,
    is_conscript: false,
    attack_cooldown: 1.5,
    scale: 72.0 / 512.0,
};

// ── Command Post ────────────────────────────────────────────────────────────
// RTS battle building. Reinforcement beacon. One per player per battle.
static COMMAND_POST_BLUEPRINT: UnitBlueprint = UnitBlueprint {
    max_hp: 800.0,
    damage: 0.0,
    attack_range: 0.0,
    speed: 0.0,
    vision_range: 14.0,
    energy_cost: 0,
    build_time_secs: 0.0,
    garrisoned_upkeep: 0.0,
    deployed_upkeep: 0.0,
    is_conscript: false,
    attack_cooldown: 0.0,
    scale: 240.0 / 512.0,
};

// ── Node ────────────────────────────────────────────────────────────────────
// Campaign home base. Production hub. If destroyed = eliminated.
static NODE_BLUEPRINT: UnitBlueprint = UnitBlueprint {
    max_hp: 2000.0,
    damage: 0.0,
    attack_range: 0.0,
    speed: 0.0,
    vision_range: 0.0,
    energy_cost: 0,
    build_time_secs: 0.0,
    garrisoned_upkeep: 0.0,
    deployed_upkeep: 0.0,
    is_conscript: false,
    attack_cooldown: 0.0,
    scale: 240.0 / 512.0,
};

// ── Capture Point ──────────────────────────────────────────────────────────
// Neutral objective on the battle map. Cannot attack or move. Provides vision.
static CAPTURE_POINT_BLUEPRINT: UnitBlueprint = UnitBlueprint {
    max_hp: 1000.0,
    damage: 0.0,
    attack_range: 0.0,
    speed: 0.0,
    vision_range: 6.0,
    energy_cost: 0,
    build_time_secs: 0.0,
    garrisoned_upkeep: 0.0,
    deployed_upkeep: 0.0,
    is_conscript: false,
    attack_cooldown: 0.0,
    scale: 160.0 / 512.0,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blueprint_values_sane() {
        for sprite_id in [
            SpriteId::Thrall,
            SpriteId::Sentinel,
            SpriteId::HoverTank,
            SpriteId::CommandPost,
            SpriteId::Node,
            SpriteId::CapturePoint,
        ] {
            let bp = get_blueprint(sprite_id);
            assert!(bp.max_hp > 0.0, "{:?} should have positive HP", sprite_id);
            assert!(bp.scale > 0.0, "{:?} should have positive scale", sprite_id);
        }
    }

    #[test]
    fn test_thrall_blueprint() {
        let bp = get_blueprint(SpriteId::Thrall);
        assert_eq!(bp.max_hp, 80.0);
        assert_eq!(bp.damage, 8.0);
        assert_eq!(bp.attack_range, 5.0);
        assert_eq!(bp.speed, 3.0);
        assert_eq!(bp.energy_cost, 30);
        assert_eq!(bp.build_time_secs, 15.0);
        assert!(bp.is_conscript);
    }

    #[test]
    fn test_sentinel_blueprint() {
        let bp = get_blueprint(SpriteId::Sentinel);
        assert_eq!(bp.max_hp, 200.0);
        assert_eq!(bp.damage, 25.0);
        assert_eq!(bp.speed, 2.0);
        assert_eq!(bp.energy_cost, 120);
        assert!(!bp.is_conscript);
    }

    #[test]
    fn test_hover_tank_blueprint() {
        let bp = get_blueprint(SpriteId::HoverTank);
        assert_eq!(bp.max_hp, 500.0);
        assert_eq!(bp.damage, 60.0);
        assert_eq!(bp.attack_range, 8.0);
        assert_eq!(bp.speed, 2.5);
        assert_eq!(bp.energy_cost, 300);
    }

    #[test]
    fn test_command_post_blueprint() {
        let bp = get_blueprint(SpriteId::CommandPost);
        assert_eq!(bp.max_hp, 800.0);
        assert_eq!(bp.damage, 0.0);
        assert_eq!(bp.speed, 0.0);
        assert_eq!(bp.vision_range, 14.0);
    }

    #[test]
    fn test_node_blueprint() {
        let bp = get_blueprint(SpriteId::Node);
        assert_eq!(bp.max_hp, 2000.0);
        assert_eq!(bp.damage, 0.0);
        assert_eq!(bp.vision_range, 0.0);
    }

    #[test]
    fn test_production_lines() {
        assert_eq!(production_line(SpriteId::Thrall), Some(ProductionLine::Infantry));
        assert_eq!(production_line(SpriteId::Sentinel), Some(ProductionLine::Infantry));
        assert_eq!(production_line(SpriteId::HoverTank), Some(ProductionLine::Armor));
        assert_eq!(production_line(SpriteId::CommandPost), None);
        assert_eq!(production_line(SpriteId::Node), None);
    }
}

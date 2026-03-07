use crate::components::{Health, UnitType};
use crate::ecs::World;
use crate::ecs::entity::Entity;
use crate::types::SpriteId;

pub fn is_entity_alive(world: &World, entity: Entity) -> bool {
    if !world.is_alive(entity) {
        return false;
    }

    match world.get_component::<Health>(entity) {
        Some(health) => !health.is_dead(),
        None => true,
    }
}

pub fn is_attackable_kind(kind: SpriteId) -> bool {
    !matches!(kind, SpriteId::CapturePoint)
}

pub fn is_entity_attackable(world: &World, target: Entity) -> bool {
    if !is_entity_alive(world, target) {
        return false;
    }

    let unit_type = match world.get_component::<UnitType>(target) {
        Some(unit_type) => unit_type,
        None => return false,
    };

    if unit_type.owner == 255 || !is_attackable_kind(unit_type.kind) {
        return false;
    }

    match world.get_component::<Health>(target) {
        Some(health) => !health.is_dead(),
        None => false,
    }
}

pub fn is_hostile_attack_target(world: &World, attacker_owner: u8, target: Entity) -> bool {
    if !is_entity_attackable(world, target) {
        return false;
    }

    world.get_component::<UnitType>(target)
        .map(|unit_type| unit_type.owner != attacker_owner)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, GameConfig};
    use crate::types::SpriteId;

    fn test_game() -> Game {
        Game::new(GameConfig {
            map_width: 32,
            map_height: 32,
            player_count: 2,
            seed: 42,
        })
    }

    #[test]
    fn neutral_capture_points_are_not_attackable() {
        let mut game = test_game();
        let capture_point = game.spawn_unit(SpriteId::CapturePoint, 10.5, 10.5, 255);

        assert!(!is_entity_attackable(&game.world, capture_point));
        assert!(!is_hostile_attack_target(&game.world, 0, capture_point));
    }

    #[test]
    fn dead_entities_are_not_attackable() {
        let mut game = test_game();
        let enemy = game.spawn_thrall(10.5, 10.5, 1);
        if let Some(health) = game.world.get_component_mut::<Health>(enemy) {
            health.current = 0.0;
        }

        assert!(!is_entity_alive(&game.world, enemy));
        assert!(!is_entity_attackable(&game.world, enemy));
        assert!(!is_hostile_attack_target(&game.world, 0, enemy));
    }
}

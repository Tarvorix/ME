use crate::command::Command;
use crate::ecs::entity::Entity;

/// Result of evaluating a behavior tree node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BtStatus {
    Success,
    Failure,
    Running,
}

/// A behavior tree node. Nodes form a tree structure via indices into a flat array.
#[derive(Clone, Debug)]
pub enum BtNode {
    /// Executes children in order. Fails if any child fails. Succeeds when all succeed.
    Sequence { children: Vec<usize> },
    /// Tries children in order. Succeeds on first success. Fails if all children fail.
    Selector { children: Vec<usize> },
    /// Inverts the child's result: Success <-> Failure, Running stays Running.
    Inverter { child: usize },
    /// Always returns Success regardless of child result (except Running).
    Succeeder { child: usize },
    /// Evaluates a condition and returns Success or Failure.
    Condition { condition_id: ConditionId },
    /// Executes an action and returns Success, Failure, or Running.
    Action { action_id: ActionId },
}

/// Condition identifiers for leaf nodes that query game state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionId {
    /// Unit currently has an assigned target.
    HasTarget,
    /// Current target is within attack range.
    TargetInRange,
    /// Current target entity is still alive.
    TargetAlive,
    /// Unit's health is below a percentage threshold.
    HealthBelowPercent(u8),
    /// There is a visible enemy within vision range.
    EnemyInVisionRange,
    /// Friendly influence exceeds enemy influence at current position.
    FriendlyStrengthAdvantage,
    /// Unit is in a tile with high enemy threat (vulnerability > threshold).
    InDangerZone,
    /// Unit has more than half health.
    HealthAboveHalf,
}

/// Action identifiers for leaf nodes that produce game commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionId {
    /// Find the nearest visible enemy and set it as target.
    AcquireNearestTarget,
    /// Attack the current target (issues Attack command).
    AttackTarget,
    /// Move toward the current target to get in range.
    ChaseTarget,
    /// Move away from enemies toward the safest nearby tile.
    Retreat,
    /// Move toward the Command Post.
    FleeToCommandPost,
    /// Stay in place and scan for enemies.
    IdleGuard,
    /// Hold current position (do nothing).
    HoldPosition,
    /// Move to assigned position if any.
    MoveToAssignedPosition,
}

/// A behavior tree stored as a flat array of nodes for cache-friendly access.
#[derive(Clone, Debug)]
pub struct BehaviorTree {
    /// All nodes in the tree. Children reference indices into this array.
    pub nodes: Vec<BtNode>,
    /// Index of the root node.
    pub root: usize,
}

/// Per-entity behavior tree execution state.
#[derive(Clone, Debug)]
pub struct BtState {
    /// Index of the currently running node (if any).
    pub running_node: Option<usize>,
}

impl BtState {
    pub fn new() -> Self {
        BtState {
            running_node: None,
        }
    }
}

/// Context passed to behavior tree evaluation.
/// Contains all the information needed for conditions and actions.
pub struct BtContext {
    /// The entity being evaluated.
    pub entity: Entity,
    /// Entity's position.
    pub pos_x: f32,
    pub pos_y: f32,
    /// Entity's current health percentage (0-100).
    pub health_pct: u8,
    /// Entity's owner player ID.
    pub owner: u8,
    /// Entity's attack range.
    pub attack_range: f32,
    /// Entity's vision range.
    pub vision_range: f32,
    /// Current target entity (if any).
    pub target: Option<Entity>,
    /// Target's position (if target exists).
    pub target_x: f32,
    pub target_y: f32,
    /// Whether the target is alive.
    pub target_alive: bool,
    /// Distance to current target.
    pub target_distance: f32,
    /// Nearest visible enemy entity (if any).
    pub nearest_enemy: Option<Entity>,
    /// Nearest enemy's position.
    pub nearest_enemy_x: f32,
    pub nearest_enemy_y: f32,
    /// Nearest enemy distance.
    pub nearest_enemy_distance: f32,
    /// Friendly influence at current tile.
    pub friendly_influence: f32,
    /// Threat influence at current tile.
    pub threat_influence: f32,
    /// Safe retreat position.
    pub safe_x: f32,
    pub safe_y: f32,
    /// Command Post position for this player.
    pub cp_x: f32,
    pub cp_y: f32,
    /// Assigned position (from strategic AI).
    pub assigned_x: Option<f32>,
    pub assigned_y: Option<f32>,
    /// Commands generated during evaluation.
    pub commands: Vec<Command>,
}

impl BtContext {
    pub fn new(entity: Entity) -> Self {
        BtContext {
            entity,
            pos_x: 0.0,
            pos_y: 0.0,
            health_pct: 100,
            owner: 0,
            attack_range: 0.0,
            vision_range: 0.0,
            target: None,
            target_x: 0.0,
            target_y: 0.0,
            target_alive: false,
            target_distance: f32::MAX,
            nearest_enemy: None,
            nearest_enemy_x: 0.0,
            nearest_enemy_y: 0.0,
            nearest_enemy_distance: f32::MAX,
            friendly_influence: 0.0,
            threat_influence: 0.0,
            safe_x: 0.0,
            safe_y: 0.0,
            cp_x: 0.0,
            cp_y: 0.0,
            assigned_x: None,
            assigned_y: None,
            commands: Vec::new(),
        }
    }
}

/// Evaluate a behavior tree node recursively.
pub fn evaluate(tree: &BehaviorTree, node_index: usize, state: &mut BtState, ctx: &mut BtContext) -> BtStatus {
    if node_index >= tree.nodes.len() {
        return BtStatus::Failure;
    }

    match &tree.nodes[node_index] {
        BtNode::Sequence { children } => {
            for &child in children {
                let result = evaluate(tree, child, state, ctx);
                match result {
                    BtStatus::Failure => return BtStatus::Failure,
                    BtStatus::Running => {
                        state.running_node = Some(child);
                        return BtStatus::Running;
                    }
                    BtStatus::Success => continue,
                }
            }
            BtStatus::Success
        }

        BtNode::Selector { children } => {
            for &child in children {
                let result = evaluate(tree, child, state, ctx);
                match result {
                    BtStatus::Success => return BtStatus::Success,
                    BtStatus::Running => {
                        state.running_node = Some(child);
                        return BtStatus::Running;
                    }
                    BtStatus::Failure => continue,
                }
            }
            BtStatus::Failure
        }

        BtNode::Inverter { child } => {
            let result = evaluate(tree, *child, state, ctx);
            match result {
                BtStatus::Success => BtStatus::Failure,
                BtStatus::Failure => BtStatus::Success,
                BtStatus::Running => BtStatus::Running,
            }
        }

        BtNode::Succeeder { child } => {
            let result = evaluate(tree, *child, state, ctx);
            match result {
                BtStatus::Running => BtStatus::Running,
                _ => BtStatus::Success,
            }
        }

        BtNode::Condition { condition_id } => {
            evaluate_condition(*condition_id, ctx)
        }

        BtNode::Action { action_id } => {
            evaluate_action(*action_id, ctx)
        }
    }
}

/// Evaluate a condition node against the context.
fn evaluate_condition(condition: ConditionId, ctx: &BtContext) -> BtStatus {
    let result = match condition {
        ConditionId::HasTarget => ctx.target.is_some(),

        ConditionId::TargetInRange => {
            ctx.target.is_some() && ctx.target_distance <= ctx.attack_range
        }

        ConditionId::TargetAlive => ctx.target.is_some() && ctx.target_alive,

        ConditionId::HealthBelowPercent(threshold) => ctx.health_pct < threshold,

        ConditionId::EnemyInVisionRange => {
            ctx.nearest_enemy.is_some() && ctx.nearest_enemy_distance <= ctx.vision_range
        }

        ConditionId::FriendlyStrengthAdvantage => {
            ctx.friendly_influence > ctx.threat_influence
        }

        ConditionId::InDangerZone => {
            // Consider danger if threat is significantly higher than friendly
            ctx.threat_influence > ctx.friendly_influence * 1.5 && ctx.threat_influence > 5.0
        }

        ConditionId::HealthAboveHalf => ctx.health_pct > 50,
    };

    if result { BtStatus::Success } else { BtStatus::Failure }
}

/// Execute an action and push commands to the context.
fn evaluate_action(action: ActionId, ctx: &mut BtContext) -> BtStatus {
    match action {
        ActionId::AcquireNearestTarget => {
            if let Some(enemy) = ctx.nearest_enemy {
                ctx.target = Some(enemy);
                ctx.target_x = ctx.nearest_enemy_x;
                ctx.target_y = ctx.nearest_enemy_y;
                ctx.target_distance = ctx.nearest_enemy_distance;
                ctx.target_alive = true;
                BtStatus::Success
            } else {
                BtStatus::Failure
            }
        }

        ActionId::AttackTarget => {
            if let Some(target) = ctx.target {
                ctx.commands.push(Command::Attack {
                    unit_ids: vec![ctx.entity.raw()],
                    target_id: target.raw(),
                });
                BtStatus::Success
            } else {
                BtStatus::Failure
            }
        }

        ActionId::ChaseTarget => {
            if ctx.target.is_some() {
                ctx.commands.push(Command::Move {
                    unit_ids: vec![ctx.entity.raw()],
                    target_x: ctx.target_x,
                    target_y: ctx.target_y,
                });
                BtStatus::Running
            } else {
                BtStatus::Failure
            }
        }

        ActionId::Retreat => {
            ctx.commands.push(Command::Move {
                unit_ids: vec![ctx.entity.raw()],
                target_x: ctx.safe_x,
                target_y: ctx.safe_y,
            });
            BtStatus::Running
        }

        ActionId::FleeToCommandPost => {
            ctx.commands.push(Command::Move {
                unit_ids: vec![ctx.entity.raw()],
                target_x: ctx.cp_x,
                target_y: ctx.cp_y,
            });
            BtStatus::Running
        }

        ActionId::IdleGuard => {
            // Do nothing actively — just succeed to indicate we're in idle state
            BtStatus::Success
        }

        ActionId::HoldPosition => {
            ctx.commands.push(Command::Stop {
                unit_ids: vec![ctx.entity.raw()],
            });
            BtStatus::Success
        }

        ActionId::MoveToAssignedPosition => {
            if let (Some(ax), Some(ay)) = (ctx.assigned_x, ctx.assigned_y) {
                let dist = ((ctx.pos_x - ax).powi(2) + (ctx.pos_y - ay).powi(2)).sqrt();
                if dist > 2.0 {
                    ctx.commands.push(Command::Move {
                        unit_ids: vec![ctx.entity.raw()],
                        target_x: ax,
                        target_y: ay,
                    });
                    BtStatus::Running
                } else {
                    BtStatus::Success
                }
            } else {
                BtStatus::Failure
            }
        }
    }
}

/// Build the standard combat behavior tree used by most AI-controlled combat units.
///
/// Tree structure:
/// ```text
/// Selector (root)
/// ├── Sequence: Retreat if critically low health
/// │   ├── Condition: HealthBelowPercent(25)
/// │   └── Action: FleeToCommandPost
/// ├── Sequence: Retreat if in danger zone and low health
/// │   ├── Condition: InDangerZone
/// │   ├── Condition: HealthBelowPercent(50)
/// │   └── Action: Retreat
/// ├── Sequence: Attack current target if alive and in range
/// │   ├── Condition: HasTarget
/// │   ├── Condition: TargetAlive
/// │   ├── Condition: TargetInRange
/// │   └── Action: AttackTarget
/// ├── Sequence: Chase current target if alive but out of range
/// │   ├── Condition: HasTarget
/// │   ├── Condition: TargetAlive
/// │   └── Action: ChaseTarget
/// ├── Sequence: Acquire and attack new target
/// │   ├── Condition: EnemyInVisionRange
/// │   ├── Action: AcquireNearestTarget
/// │   └── Action: AttackTarget
/// ├── Sequence: Move to assigned position
/// │   └── Action: MoveToAssignedPosition
/// └── Action: IdleGuard
/// ```
pub fn build_combat_bt() -> BehaviorTree {
    let mut nodes: Vec<BtNode> = Vec::new();

    // === Leaf nodes (indices 0-15) ===

    // 0: Condition - Health below 25%
    nodes.push(BtNode::Condition { condition_id: ConditionId::HealthBelowPercent(25) });
    // 1: Action - Flee to Command Post
    nodes.push(BtNode::Action { action_id: ActionId::FleeToCommandPost });
    // 2: Condition - In danger zone
    nodes.push(BtNode::Condition { condition_id: ConditionId::InDangerZone });
    // 3: Condition - Health below 50%
    nodes.push(BtNode::Condition { condition_id: ConditionId::HealthBelowPercent(50) });
    // 4: Action - Retreat
    nodes.push(BtNode::Action { action_id: ActionId::Retreat });
    // 5: Condition - Has target
    nodes.push(BtNode::Condition { condition_id: ConditionId::HasTarget });
    // 6: Condition - Target alive
    nodes.push(BtNode::Condition { condition_id: ConditionId::TargetAlive });
    // 7: Condition - Target in range
    nodes.push(BtNode::Condition { condition_id: ConditionId::TargetInRange });
    // 8: Action - Attack target
    nodes.push(BtNode::Action { action_id: ActionId::AttackTarget });
    // 9: Condition - Has target (duplicate for chase sequence)
    nodes.push(BtNode::Condition { condition_id: ConditionId::HasTarget });
    // 10: Condition - Target alive (duplicate for chase sequence)
    nodes.push(BtNode::Condition { condition_id: ConditionId::TargetAlive });
    // 11: Action - Chase target
    nodes.push(BtNode::Action { action_id: ActionId::ChaseTarget });
    // 12: Condition - Enemy in vision range
    nodes.push(BtNode::Condition { condition_id: ConditionId::EnemyInVisionRange });
    // 13: Action - Acquire nearest target
    nodes.push(BtNode::Action { action_id: ActionId::AcquireNearestTarget });
    // 14: Action - Attack target (after acquire)
    nodes.push(BtNode::Action { action_id: ActionId::AttackTarget });
    // 15: Action - Move to assigned position
    nodes.push(BtNode::Action { action_id: ActionId::MoveToAssignedPosition });
    // 16: Action - Idle guard
    nodes.push(BtNode::Action { action_id: ActionId::IdleGuard });

    // === Composite nodes (indices 17-23) ===

    // 17: Sequence - Critical retreat (health < 25% → flee to CP)
    nodes.push(BtNode::Sequence { children: vec![0, 1] });
    // 18: Sequence - Danger zone retreat (in danger + health < 50% → retreat)
    nodes.push(BtNode::Sequence { children: vec![2, 3, 4] });
    // 19: Sequence - Attack in range (has target + alive + in range → attack)
    nodes.push(BtNode::Sequence { children: vec![5, 6, 7, 8] });
    // 20: Sequence - Chase target (has target + alive → chase)
    nodes.push(BtNode::Sequence { children: vec![9, 10, 11] });
    // 21: Sequence - Acquire and attack (enemy visible → acquire → attack)
    nodes.push(BtNode::Sequence { children: vec![12, 13, 14] });

    // 22: Root Selector
    nodes.push(BtNode::Selector {
        children: vec![17, 18, 19, 20, 21, 15, 16],
    });

    BehaviorTree {
        root: 22,
        nodes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::entity::Entity;

    fn test_entity() -> Entity {
        Entity::new(1, 0)
    }

    fn base_context() -> BtContext {
        let mut ctx = BtContext::new(test_entity());
        ctx.pos_x = 10.0;
        ctx.pos_y = 10.0;
        ctx.health_pct = 100;
        ctx.owner = 0;
        ctx.attack_range = 5.0;
        ctx.vision_range = 8.0;
        ctx.cp_x = 5.0;
        ctx.cp_y = 5.0;
        ctx.safe_x = 3.0;
        ctx.safe_y = 3.0;
        ctx
    }

    // ---- Basic Node Tests ----

    #[test]
    fn test_sequence_all_success() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Condition { condition_id: ConditionId::HealthAboveHalf },    // 0
                BtNode::Action { action_id: ActionId::IdleGuard },                    // 1
                BtNode::Sequence { children: vec![0, 1] },                            // 2
            ],
            root: 2,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();
        ctx.health_pct = 80; // above half

        let result = evaluate(&tree, tree.root, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Success);
    }

    #[test]
    fn test_sequence_failure_short_circuits() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Condition { condition_id: ConditionId::HasTarget },            // 0 - will fail (no target)
                BtNode::Action { action_id: ActionId::AttackTarget },                  // 1 - should not execute
                BtNode::Sequence { children: vec![0, 1] },                             // 2
            ],
            root: 2,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();
        // No target set

        let result = evaluate(&tree, tree.root, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Failure);
        assert!(ctx.commands.is_empty(), "Attack should not have executed");
    }

    #[test]
    fn test_selector_first_success() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Action { action_id: ActionId::IdleGuard },                    // 0 - succeeds
                BtNode::Action { action_id: ActionId::HoldPosition },                 // 1 - should not execute
                BtNode::Selector { children: vec![0, 1] },                            // 2
            ],
            root: 2,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();

        let result = evaluate(&tree, tree.root, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Success);
        // Only IdleGuard executed (no commands), HoldPosition would have pushed a Stop command
        assert!(ctx.commands.is_empty(), "Only idle should have executed, no commands");
    }

    #[test]
    fn test_selector_all_failure() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Condition { condition_id: ConditionId::HasTarget },            // 0 - fails
                BtNode::Condition { condition_id: ConditionId::EnemyInVisionRange },   // 1 - fails
                BtNode::Selector { children: vec![0, 1] },                            // 2
            ],
            root: 2,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();

        let result = evaluate(&tree, tree.root, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Failure);
    }

    #[test]
    fn test_inverter() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Condition { condition_id: ConditionId::HasTarget },            // 0 - fails (no target)
                BtNode::Inverter { child: 0 },                                        // 1
            ],
            root: 1,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();

        let result = evaluate(&tree, tree.root, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Success, "Inverter should turn Failure into Success");
    }

    #[test]
    fn test_succeeder() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Condition { condition_id: ConditionId::HasTarget },            // 0 - fails
                BtNode::Succeeder { child: 0 },                                       // 1
            ],
            root: 1,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();

        let result = evaluate(&tree, tree.root, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Success, "Succeeder should turn Failure into Success");
    }

    // ---- Condition Tests ----

    #[test]
    fn test_condition_has_target() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Condition { condition_id: ConditionId::HasTarget },
            ],
            root: 0,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();

        // No target
        assert_eq!(evaluate(&tree, 0, &mut state, &mut ctx), BtStatus::Failure);

        // With target
        ctx.target = Some(Entity::new(5, 0));
        assert_eq!(evaluate(&tree, 0, &mut state, &mut ctx), BtStatus::Success);
    }

    #[test]
    fn test_condition_target_in_range() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Condition { condition_id: ConditionId::TargetInRange },
            ],
            root: 0,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();
        ctx.target = Some(Entity::new(5, 0));
        ctx.attack_range = 5.0;

        // In range
        ctx.target_distance = 3.0;
        assert_eq!(evaluate(&tree, 0, &mut state, &mut ctx), BtStatus::Success);

        // Out of range
        ctx.target_distance = 7.0;
        assert_eq!(evaluate(&tree, 0, &mut state, &mut ctx), BtStatus::Failure);
    }

    #[test]
    fn test_condition_health_below() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Condition { condition_id: ConditionId::HealthBelowPercent(50) },
            ],
            root: 0,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();

        ctx.health_pct = 80;
        assert_eq!(evaluate(&tree, 0, &mut state, &mut ctx), BtStatus::Failure);

        ctx.health_pct = 30;
        assert_eq!(evaluate(&tree, 0, &mut state, &mut ctx), BtStatus::Success);
    }

    // ---- Action Tests ----

    #[test]
    fn test_action_acquire_target() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Action { action_id: ActionId::AcquireNearestTarget },
            ],
            root: 0,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();

        // No enemy visible
        assert_eq!(evaluate(&tree, 0, &mut state, &mut ctx), BtStatus::Failure);

        // Enemy visible
        let enemy = Entity::new(10, 0);
        ctx.nearest_enemy = Some(enemy);
        ctx.nearest_enemy_x = 15.0;
        ctx.nearest_enemy_y = 10.0;
        ctx.nearest_enemy_distance = 5.0;

        let result = evaluate(&tree, 0, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Success);
        assert_eq!(ctx.target, Some(enemy));
        assert_eq!(ctx.target_x, 15.0);
    }

    #[test]
    fn test_action_attack_target() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Action { action_id: ActionId::AttackTarget },
            ],
            root: 0,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();

        let target = Entity::new(5, 0);
        ctx.target = Some(target);

        let result = evaluate(&tree, 0, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Success);
        assert_eq!(ctx.commands.len(), 1);
        match &ctx.commands[0] {
            Command::Attack { unit_ids, target_id } => {
                assert_eq!(unit_ids, &vec![test_entity().raw()]);
                assert_eq!(*target_id, target.raw());
            }
            _ => panic!("Expected Attack command"),
        }
    }

    #[test]
    fn test_action_chase_target() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Action { action_id: ActionId::ChaseTarget },
            ],
            root: 0,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();

        ctx.target = Some(Entity::new(5, 0));
        ctx.target_x = 20.0;
        ctx.target_y = 15.0;

        let result = evaluate(&tree, 0, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Running);
        assert_eq!(ctx.commands.len(), 1);
        match &ctx.commands[0] {
            Command::Move { target_x, target_y, .. } => {
                assert_eq!(*target_x, 20.0);
                assert_eq!(*target_y, 15.0);
            }
            _ => panic!("Expected Move command"),
        }
    }

    #[test]
    fn test_action_retreat() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Action { action_id: ActionId::Retreat },
            ],
            root: 0,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();
        ctx.safe_x = 3.0;
        ctx.safe_y = 3.0;

        let result = evaluate(&tree, 0, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Running);
        match &ctx.commands[0] {
            Command::Move { target_x, target_y, .. } => {
                assert_eq!(*target_x, 3.0);
                assert_eq!(*target_y, 3.0);
            }
            _ => panic!("Expected Move command to safe position"),
        }
    }

    #[test]
    fn test_action_flee_to_cp() {
        let tree = BehaviorTree {
            nodes: vec![
                BtNode::Action { action_id: ActionId::FleeToCommandPost },
            ],
            root: 0,
        };

        let mut state = BtState::new();
        let mut ctx = base_context();
        ctx.cp_x = 5.0;
        ctx.cp_y = 5.0;

        let result = evaluate(&tree, 0, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Running);
        match &ctx.commands[0] {
            Command::Move { target_x, target_y, .. } => {
                assert_eq!(*target_x, 5.0);
                assert_eq!(*target_y, 5.0);
            }
            _ => panic!("Expected Move command to CP"),
        }
    }

    // ---- Combat BT Tests ----

    #[test]
    fn test_combat_bt_retreat_critical() {
        let bt = build_combat_bt();
        let mut state = BtState::new();
        let mut ctx = base_context();

        // Critical health (< 25%)
        ctx.health_pct = 15;

        let result = evaluate(&bt, bt.root, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Running);
        // Should flee to CP
        assert!(!ctx.commands.is_empty(), "Should generate flee command");
        match &ctx.commands[0] {
            Command::Move { target_x, target_y, .. } => {
                assert_eq!(*target_x, ctx.cp_x);
                assert_eq!(*target_y, ctx.cp_y);
            }
            _ => panic!("Expected Move to CP, got {:?}", ctx.commands[0]),
        }
    }

    #[test]
    fn test_combat_bt_attack_in_range() {
        let bt = build_combat_bt();
        let mut state = BtState::new();
        let mut ctx = base_context();

        let target = Entity::new(5, 0);
        ctx.target = Some(target);
        ctx.target_alive = true;
        ctx.target_distance = 3.0; // within range
        ctx.target_x = 13.0;
        ctx.target_y = 10.0;

        let result = evaluate(&bt, bt.root, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Success);
        assert_eq!(ctx.commands.len(), 1);
        match &ctx.commands[0] {
            Command::Attack { target_id, .. } => {
                assert_eq!(*target_id, target.raw());
            }
            _ => panic!("Expected Attack command"),
        }
    }

    #[test]
    fn test_combat_bt_chase() {
        let bt = build_combat_bt();
        let mut state = BtState::new();
        let mut ctx = base_context();

        let target = Entity::new(5, 0);
        ctx.target = Some(target);
        ctx.target_alive = true;
        ctx.target_distance = 8.0; // out of attack range (5.0)
        ctx.target_x = 18.0;
        ctx.target_y = 10.0;

        let result = evaluate(&bt, bt.root, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Running);
        match &ctx.commands[0] {
            Command::Move { target_x, target_y, .. } => {
                assert_eq!(*target_x, 18.0);
                assert_eq!(*target_y, 10.0);
            }
            _ => panic!("Expected Move (chase) command"),
        }
    }

    #[test]
    fn test_combat_bt_acquire_and_attack() {
        let bt = build_combat_bt();
        let mut state = BtState::new();
        let mut ctx = base_context();

        // No current target, but enemy in vision range
        let enemy = Entity::new(8, 0);
        ctx.nearest_enemy = Some(enemy);
        ctx.nearest_enemy_x = 14.0;
        ctx.nearest_enemy_y = 10.0;
        ctx.nearest_enemy_distance = 4.0;

        let result = evaluate(&bt, bt.root, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Success);
        // Should have acquired target and issued attack
        assert_eq!(ctx.target, Some(enemy));
        assert!(!ctx.commands.is_empty());
        match &ctx.commands[0] {
            Command::Attack { target_id, .. } => {
                assert_eq!(*target_id, enemy.raw());
            }
            _ => panic!("Expected Attack command after acquire"),
        }
    }

    #[test]
    fn test_combat_bt_idle() {
        let bt = build_combat_bt();
        let mut state = BtState::new();
        let mut ctx = base_context();

        // No target, no enemies, no assigned position
        let result = evaluate(&bt, bt.root, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Success);
        // Should idle — no commands generated (MoveToAssignedPosition fails, IdleGuard succeeds)
        assert!(ctx.commands.is_empty(), "Should have no commands in idle state");
    }

    #[test]
    fn test_combat_bt_move_to_assigned() {
        let bt = build_combat_bt();
        let mut state = BtState::new();
        let mut ctx = base_context();

        // No target, no enemies, but has assigned position far away
        ctx.assigned_x = Some(30.0);
        ctx.assigned_y = Some(30.0);

        let result = evaluate(&bt, bt.root, &mut state, &mut ctx);
        assert_eq!(result, BtStatus::Running);
        match &ctx.commands[0] {
            Command::Move { target_x, target_y, .. } => {
                assert_eq!(*target_x, 30.0);
                assert_eq!(*target_y, 30.0);
            }
            _ => panic!("Expected Move to assigned position"),
        }
    }
}

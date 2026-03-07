pub mod ecs;
pub mod types;
pub mod components;
pub mod blueprints;
pub mod map;
pub mod command;
pub mod pathfinding;
pub mod systems;
pub mod game;
pub mod protocol;
pub mod state_snapshot;
pub mod ai;
pub mod deployment;
pub mod campaign;
pub mod campaign_game;
pub mod replay;
pub mod targeting;

pub fn hello() -> &'static str {
    "Machine Empire Core"
}

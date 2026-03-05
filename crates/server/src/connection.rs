use tokio::sync::mpsc;
use uuid::Uuid;

use machine_empire_core::protocol::{MatchId, ServerMessage};
use machine_empire_core::command::Command;

/// Unique connection identifier.
pub type ConnectionId = Uuid;

/// Represents a connected client (human player or spectator).
pub struct Connection {
    /// Unique connection ID.
    pub id: ConnectionId,
    /// Player name (set on Join).
    pub player_name: Option<String>,
    /// Match this connection is participating in.
    pub match_id: Option<MatchId>,
    /// Player slot in the match (0-3).
    pub player_slot: Option<u8>,
    /// Channel to send messages to this client.
    pub tx: mpsc::UnboundedSender<ServerMessage>,
}

impl Connection {
    pub fn new(tx: mpsc::UnboundedSender<ServerMessage>) -> Self {
        Connection {
            id: Uuid::new_v4(),
            player_name: None,
            match_id: None,
            player_slot: None,
            tx,
        }
    }
}

/// Handle for sending commands to a match from a player connection.
#[allow(dead_code)]
pub struct PlayerCommandSender {
    pub player_slot: u8,
    pub cmd_tx: mpsc::UnboundedSender<Command>,
}

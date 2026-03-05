use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::info;

use machine_empire_core::protocol::{MatchId, MatchConfig, ServerMessage};
use machine_empire_core::command::Command;

use crate::connection::{Connection, ConnectionId};
use crate::lobby::{Lobby, LobbyStatus};
use crate::match_runner::{MatchRunner, PlayerHandle};

/// Handle to a running match (holds channels and task handle).
#[allow(dead_code)]
pub struct MatchHandle {
    /// Match identifier.
    pub match_id: MatchId,
    /// Per-player command senders (player_slot -> channel).
    pub player_cmd_txs: HashMap<u8, mpsc::UnboundedSender<Command>>,
    /// Per-player connection IDs (player_slot -> connection_id).
    pub player_connections: HashMap<u8, ConnectionId>,
    /// Task handle for the match runner.
    pub task_handle: JoinHandle<()>,
}

/// Shared server state, accessed by all connection handlers.
pub struct ServerState {
    /// Active WebSocket connections.
    pub connections: HashMap<ConnectionId, Connection>,
    /// Active match handles.
    pub matches: HashMap<MatchId, MatchHandle>,
    /// Active lobbies.
    pub lobbies: HashMap<MatchId, Lobby>,
}

impl ServerState {
    pub fn new() -> Self {
        ServerState {
            connections: HashMap::new(),
            matches: HashMap::new(),
            lobbies: HashMap::new(),
        }
    }

    pub fn add_connection(&mut self, conn: Connection) {
        self.connections.insert(conn.id, conn);
    }

    pub fn remove_connection(&mut self, id: &ConnectionId) {
        // Remove from any lobby they're in
        let lobby_ids: Vec<MatchId> = self.lobbies.keys().cloned().collect();
        for lobby_id in lobby_ids {
            if let Some(lobby) = self.lobbies.get_mut(&lobby_id) {
                lobby.remove_player(id);
                if lobby.player_count() == 0 && lobby.status == LobbyStatus::Waiting {
                    self.lobbies.remove(&lobby_id);
                }
            }
        }

        self.connections.remove(id);
    }

    pub fn get_connection(&self, id: &ConnectionId) -> Option<&Connection> {
        self.connections.get(id)
    }

    pub fn get_connection_mut(&mut self, id: &ConnectionId) -> Option<&mut Connection> {
        self.connections.get_mut(id)
    }

    /// Send a message to a specific connection.
    pub fn send_to_connection(&self, conn_id: &ConnectionId, msg: ServerMessage) -> bool {
        if let Some(conn) = self.connections.get(conn_id) {
            conn.tx.send(msg).is_ok()
        } else {
            false
        }
    }

    /// Send a command to a match for a specific player.
    pub fn send_command_to_match(
        &self,
        match_id: &MatchId,
        player_slot: u8,
        cmd: Command,
    ) -> bool {
        if let Some(match_handle) = self.matches.get(match_id) {
            if let Some(tx) = match_handle.player_cmd_txs.get(&player_slot) {
                return tx.send(cmd).is_ok();
            }
        }
        false
    }

    // ---- Lobby Management ----

    /// Create a new lobby with the given match ID and config.
    pub fn create_lobby(&mut self, match_id: MatchId, config: MatchConfig) -> &mut Lobby {
        let lobby = Lobby::new(match_id.clone(), config);
        self.lobbies.insert(match_id.clone(), lobby);
        self.lobbies.get_mut(&match_id).unwrap()
    }

    /// Get a reference to a lobby.
    pub fn get_lobby(&self, match_id: &MatchId) -> Option<&Lobby> {
        self.lobbies.get(match_id)
    }

    /// Get a mutable reference to a lobby.
    pub fn get_lobby_mut(&mut self, match_id: &MatchId) -> Option<&mut Lobby> {
        self.lobbies.get_mut(match_id)
    }

    /// Find which lobby a connection is in (if any).
    pub fn find_lobby_for_connection(&self, conn_id: &ConnectionId) -> Option<MatchId> {
        for (match_id, lobby) in &self.lobbies {
            for slot in &lobby.players {
                if let Some(ps) = slot {
                    if ps.connection_id.as_ref() == Some(conn_id) {
                        return Some(match_id.clone());
                    }
                }
            }
        }
        None
    }

    /// Broadcast lobby status to all players in the lobby.
    pub fn broadcast_lobby_status(&self, match_id: &MatchId) {
        if let Some(lobby) = self.lobbies.get(match_id) {
            let msg = ServerMessage::Lobby {
                players: lobby.player_info_list(),
                status: lobby.status_string(),
            };

            for slot in &lobby.players {
                if let Some(ps) = slot {
                    if let Some(conn_id) = &ps.connection_id {
                        self.send_to_connection(conn_id, msg.clone());
                    }
                }
            }
        }
    }

    /// Start a match from a lobby. Creates channels, spawns the match runner task,
    /// and transitions the lobby to InProgress.
    /// Returns the match ID on success.
    pub fn start_match_from_lobby(&mut self, match_id: &MatchId) -> Result<MatchId, String> {
        let lobby = self.lobbies.get_mut(match_id)
            .ok_or_else(|| "Lobby not found".to_string())?;

        if lobby.status != LobbyStatus::Waiting {
            return Err("Lobby is not in Waiting state".into());
        }

        if !lobby.all_ready() {
            return Err("Not all players are ready".into());
        }

        // Transition lobby to Starting
        lobby.status = LobbyStatus::Starting;

        let config = lobby.config.clone();
        let result_match_id = lobby.match_id.clone();

        // Build player handles and command channels
        let mut player_handles = Vec::new();
        let mut player_cmd_txs: HashMap<u8, mpsc::UnboundedSender<Command>> = HashMap::new();
        let mut player_connections: HashMap<u8, ConnectionId> = HashMap::new();

        for (slot_index, slot) in lobby.players.iter().enumerate() {
            if let Some(ps) = slot {
                let player_id = slot_index as u8;

                if ps.is_ai {
                    // AI player: no channels needed
                    player_handles.push(PlayerHandle {
                        player_id,
                        is_ai: true,
                        cmd_rx: None,
                        state_tx: None,
                    });
                } else {
                    // Human player: create command channel, use connection's tx for state
                    let conn_id = ps.connection_id
                        .ok_or_else(|| format!("Human player {} has no connection", slot_index))?;

                    let state_tx = self.connections.get(&conn_id)
                        .map(|conn| conn.tx.clone())
                        .ok_or_else(|| format!("Connection not found for player {}", slot_index))?;

                    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

                    player_cmd_txs.insert(player_id, cmd_tx);
                    player_connections.insert(player_id, conn_id);

                    player_handles.push(PlayerHandle {
                        player_id,
                        is_ai: false,
                        cmd_rx: Some(cmd_rx),
                        state_tx: Some(state_tx),
                    });

                    // Update connection with match info
                    if let Some(conn) = self.connections.get_mut(&conn_id) {
                        conn.match_id = Some(result_match_id.clone());
                        conn.player_slot = Some(player_id);
                    }
                }
            }
        }

        info!(
            "Starting match '{}' with {} players",
            result_match_id.0,
            player_handles.len()
        );

        // Create and spawn the match runner
        let runner = MatchRunner::new(result_match_id.clone(), config, player_handles);
        let task_handle = tokio::spawn(async move {
            runner.run().await;
        });

        // Store match handle
        self.matches.insert(result_match_id.clone(), MatchHandle {
            match_id: result_match_id.clone(),
            player_cmd_txs,
            player_connections,
            task_handle,
        });

        // Transition lobby to InProgress
        if let Some(lobby) = self.lobbies.get_mut(match_id) {
            lobby.status = LobbyStatus::InProgress;
        }

        Ok(result_match_id)
    }
}

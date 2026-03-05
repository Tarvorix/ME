use machine_empire_core::protocol::{MatchId, MatchConfig, PlayerInfo, PlayerId};
use crate::connection::ConnectionId;

/// Status of a lobby.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LobbyStatus {
    Waiting,
    Starting,
    InProgress,
    #[allow(dead_code)]
    Finished,
}

/// A player slot in a lobby.
#[derive(Clone, Debug)]
pub struct PlayerSlot {
    /// Connection ID (None for AI players).
    pub connection_id: Option<ConnectionId>,
    /// Player display name.
    pub name: String,
    /// Whether this is an AI player.
    pub is_ai: bool,
    /// Whether the player has signaled ready.
    pub is_ready: bool,
}

/// A game lobby waiting for players to join and ready up.
pub struct Lobby {
    /// Match identifier.
    pub match_id: MatchId,
    /// Lobby display name.
    pub name: String,
    /// Maximum number of players.
    pub max_players: u8,
    /// Match configuration.
    pub config: MatchConfig,
    /// Player slots (indexed by player_id 0-3).
    pub players: Vec<Option<PlayerSlot>>,
    /// Alias for players (HTTP API compatibility).
    pub slots: Vec<Option<HttpPlayerSlot>>,
    /// Current lobby status.
    pub status: LobbyStatus,
}

/// Simplified player slot for HTTP API.
#[derive(Clone, Debug)]
pub struct HttpPlayerSlot {
    pub name: String,
    pub is_ai: bool,
    pub ready: bool,
}

impl Lobby {
    /// Create a new lobby with the given config.
    pub fn new(match_id: MatchId, config: MatchConfig) -> Self {
        let player_count = config.player_count as usize;
        Lobby {
            match_id: match_id.clone(),
            name: match_id.0.clone(),
            max_players: config.player_count as u8,
            config,
            players: vec![None; player_count],
            slots: vec![None; player_count],
            status: LobbyStatus::Waiting,
        }
    }

    /// Create a lobby from HTTP API (name + max_players).
    pub fn from_http(name: String, max_players: u8) -> Self {
        let config = MatchConfig {
            map_width: 64,
            map_height: 64,
            player_count: max_players as u32,
            seed: 42,
            tick_rate_ms: 50,
        };
        let match_id = MatchId(name.clone());
        Lobby {
            match_id,
            name,
            max_players,
            config,
            players: vec![None; max_players as usize],
            slots: vec![None; max_players as usize],
            status: LobbyStatus::Waiting,
        }
    }

    /// Add a human player to the next available slot. Returns the assigned player_id.
    pub fn add_player(&mut self, conn_id: ConnectionId, name: String) -> Result<u8, String> {
        if self.status != LobbyStatus::Waiting {
            return Err("Lobby is not accepting players".into());
        }

        for (i, slot) in self.players.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(PlayerSlot {
                    connection_id: Some(conn_id),
                    name,
                    is_ai: false,
                    is_ready: false,
                });
                return Ok(i as u8);
            }
        }

        Err("Lobby is full".into())
    }

    /// Add an AI player to the next available slot. Returns the assigned player_id.
    #[allow(dead_code)]
    pub fn add_ai_player(&mut self, name: String) -> Result<u8, String> {
        if self.status != LobbyStatus::Waiting {
            return Err("Lobby is not accepting players".into());
        }

        for (i, slot) in self.players.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(PlayerSlot {
                    connection_id: None,
                    name,
                    is_ai: true,
                    is_ready: true, // AI is always ready
                });
                return Ok(i as u8);
            }
        }

        Err("Lobby is full".into())
    }

    /// Remove a player from the lobby.
    pub fn remove_player(&mut self, conn_id: &ConnectionId) {
        for slot in &mut self.players {
            if let Some(ref ps) = slot {
                if ps.connection_id.as_ref() == Some(conn_id) {
                    *slot = None;
                    return;
                }
            }
        }
    }

    /// Mark a player as ready.
    pub fn set_ready(&mut self, conn_id: &ConnectionId) -> bool {
        for slot in &mut self.players {
            if let Some(ref mut ps) = slot {
                if ps.connection_id.as_ref() == Some(conn_id) {
                    ps.is_ready = true;
                    return true;
                }
            }
        }
        false
    }

    /// Check if all slots are filled and all players are ready.
    pub fn all_ready(&self) -> bool {
        self.players.iter().all(|slot| {
            match slot {
                Some(ps) => ps.is_ready,
                None => false,
            }
        })
    }

    /// Count occupied player slots.
    pub fn player_count(&self) -> u8 {
        self.players.iter().filter(|s| s.is_some()).count() as u8
    }

    /// Build player info list for lobby status messages.
    pub fn player_info_list(&self) -> Vec<PlayerInfo> {
        self.players.iter().enumerate().filter_map(|(i, slot)| {
            slot.as_ref().map(|ps| PlayerInfo {
                id: PlayerId(i as u8),
                name: ps.name.clone(),
                is_ai: ps.is_ai,
                is_ready: ps.is_ready,
            })
        }).collect()
    }

    /// Get status as a string.
    pub fn status_string(&self) -> String {
        match self.status {
            LobbyStatus::Waiting => "Waiting".into(),
            LobbyStatus::Starting => "Starting".into(),
            LobbyStatus::InProgress => "InProgress".into(),
            LobbyStatus::Finished => "Finished".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_config() -> MatchConfig {
        MatchConfig {
            map_width: 64,
            map_height: 64,
            player_count: 2,
            seed: 42,
            tick_rate_ms: 50,
        }
    }

    #[test]
    fn test_lobby_creation() {
        let lobby = Lobby::new(MatchId("test".into()), test_config());
        assert_eq!(lobby.status, LobbyStatus::Waiting);
        assert_eq!(lobby.players.len(), 2);
        assert_eq!(lobby.player_count(), 0);
    }

    #[test]
    fn test_lobby_add_player() {
        let mut lobby = Lobby::new(MatchId("test".into()), test_config());
        let conn = Uuid::new_v4();
        let slot = lobby.add_player(conn, "Player1".into()).unwrap();
        assert_eq!(slot, 0);
        assert_eq!(lobby.player_count(), 1);
    }

    #[test]
    fn test_lobby_full() {
        let mut lobby = Lobby::new(MatchId("test".into()), test_config());
        lobby.add_player(Uuid::new_v4(), "P1".into()).unwrap();
        lobby.add_player(Uuid::new_v4(), "P2".into()).unwrap();
        assert!(lobby.add_player(Uuid::new_v4(), "P3".into()).is_err());
    }

    #[test]
    fn test_lobby_ready() {
        let mut lobby = Lobby::new(MatchId("test".into()), test_config());
        let c1 = Uuid::new_v4();
        let c2 = Uuid::new_v4();
        lobby.add_player(c1, "P1".into()).unwrap();
        lobby.add_player(c2, "P2".into()).unwrap();

        assert!(!lobby.all_ready());
        lobby.set_ready(&c1);
        assert!(!lobby.all_ready());
        lobby.set_ready(&c2);
        assert!(lobby.all_ready());
    }

    #[test]
    fn test_lobby_add_ai_player() {
        let mut lobby = Lobby::new(MatchId("test".into()), test_config());
        let c1 = Uuid::new_v4();
        lobby.add_player(c1, "Human".into()).unwrap();
        lobby.add_ai_player("AI Bot".into()).unwrap();

        // AI is always ready
        lobby.set_ready(&c1);
        assert!(lobby.all_ready());
    }

    #[test]
    fn test_lobby_remove_player() {
        let mut lobby = Lobby::new(MatchId("test".into()), test_config());
        let c1 = Uuid::new_v4();
        lobby.add_player(c1, "P1".into()).unwrap();
        assert_eq!(lobby.player_count(), 1);

        lobby.remove_player(&c1);
        assert_eq!(lobby.player_count(), 0);
    }

    #[test]
    fn test_lobby_player_info() {
        let mut lobby = Lobby::new(MatchId("test".into()), test_config());
        lobby.add_player(Uuid::new_v4(), "Alice".into()).unwrap();
        lobby.add_ai_player("Bot".into()).unwrap();

        let info = lobby.player_info_list();
        assert_eq!(info.len(), 2);
        assert_eq!(info[0].name, "Alice");
        assert!(!info[0].is_ai);
        assert_eq!(info[1].name, "Bot");
        assert!(info[1].is_ai);
    }
}

use std::sync::Arc;
use std::net::SocketAddr;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::tungstenite::Message;
use futures_util::{StreamExt, SinkExt};
use tracing::{info, warn, error};

use machine_empire_core::protocol::{ClientMessage, MatchConfig, MatchId, ServerMessage};

use crate::connection::Connection;
use crate::server_state::ServerState;

/// Start the WebSocket server on the specified port.
pub async fn run(state: Arc<Mutex<ServerState>>, port: u16) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind WebSocket listener on {}: {}", addr, e);
            return;
        }
    };

    info!("WebSocket listener bound to {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                info!("New TCP connection from {}", peer_addr);
                let state = state.clone();
                tokio::spawn(async move {
                    handle_connection(state, stream, peer_addr).await;
                });
            }
            Err(e) => {
                warn!("Failed to accept connection: {}", e);
            }
        }
    }
}

/// Handle a single WebSocket connection lifecycle.
async fn handle_connection(
    state: Arc<Mutex<ServerState>>,
    stream: TcpStream,
    peer_addr: SocketAddr,
) {
    // Upgrade TCP to WebSocket
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            warn!("WebSocket handshake failed for {}: {}", peer_addr, e);
            return;
        }
    };

    info!("WebSocket connection established with {}", peer_addr);

    let (mut ws_sink, mut ws_stream_reader) = ws_stream.split();

    // Create a channel for sending messages to this client
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Register connection
    let conn = Connection::new(tx);
    let conn_id = conn.id;
    {
        let mut server_state = state.lock().await;
        server_state.add_connection(conn);
    }

    // Spawn a task to forward outgoing messages from channel to WebSocket
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match encode_server_message(&msg) {
                Ok(bytes) => {
                    if ws_sink.send(Message::Binary(bytes.into())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    warn!("Failed to encode message: {}", e);
                }
            }
        }
    });

    // Read incoming messages
    while let Some(msg_result) = ws_stream_reader.next().await {
        match msg_result {
            Ok(Message::Binary(data)) => {
                match decode_client_message(&data) {
                    Ok(client_msg) => {
                        handle_client_message(&state, &conn_id, client_msg).await;
                    }
                    Err(e) => {
                        warn!("Failed to decode message from {}: {}", peer_addr, e);
                    }
                }
            }
            Ok(Message::Text(text)) => {
                // Try JSON fallback for debugging
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        handle_client_message(&state, &conn_id, client_msg).await;
                    }
                    Err(e) => {
                        warn!("Failed to parse JSON from {}: {}", peer_addr, e);
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                // Tungstenite handles pong automatically
                let _ = data;
            }
            Ok(Message::Close(_)) => {
                info!("Client {} sent close frame", peer_addr);
                break;
            }
            Err(e) => {
                warn!("WebSocket error from {}: {}", peer_addr, e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup: remove connection
    {
        let mut server_state = state.lock().await;
        server_state.remove_connection(&conn_id);
    }

    write_task.abort();
    info!("Connection closed for {}", peer_addr);
}

/// Handle a decoded client message.
async fn handle_client_message(
    state: &Arc<Mutex<ServerState>>,
    conn_id: &crate::connection::ConnectionId,
    msg: ClientMessage,
) {
    match msg {
        ClientMessage::Ping { seq } => {
            let server_state = state.lock().await;
            let pong = ServerMessage::Pong { seq, server_tick: 0 };
            server_state.send_to_connection(conn_id, pong);
        }
        ClientMessage::Cmd { cmd } => {
            let server_state = state.lock().await;
            if let Some(conn) = server_state.get_connection(conn_id) {
                if let (Some(match_id), Some(slot)) = (&conn.match_id, conn.player_slot) {
                    server_state.send_command_to_match(match_id, slot, cmd);
                }
            }
        }
        ClientMessage::Join { lobby_id, player_name } => {
            let mut server_state = state.lock().await;

            // Set player name on connection
            if let Some(conn) = server_state.get_connection_mut(conn_id) {
                conn.player_name = Some(player_name.clone());
            }

            let match_id = MatchId(lobby_id.clone());

            // Create lobby if it doesn't exist
            if server_state.get_lobby(&match_id).is_none() {
                let config = MatchConfig::default();
                server_state.create_lobby(match_id.clone(), config);
                info!("Created new lobby '{}'", lobby_id);
            }

            // Try to add the player to the lobby
            match server_state.get_lobby_mut(&match_id) {
                Some(lobby) => {
                    match lobby.add_player(*conn_id, player_name.clone()) {
                        Ok(slot) => {
                            info!("Player '{}' joined lobby '{}' as slot {}", player_name, lobby_id, slot);
                            // Broadcast updated lobby status to all players in lobby
                            server_state.broadcast_lobby_status(&match_id);
                        }
                        Err(e) => {
                            warn!("Failed to add player '{}' to lobby '{}': {}", player_name, lobby_id, e);
                            server_state.send_to_connection(conn_id, ServerMessage::Error {
                                message: e,
                            });
                        }
                    }
                }
                None => {
                    server_state.send_to_connection(conn_id, ServerMessage::Error {
                        message: "Lobby not found".into(),
                    });
                }
            }
        }
        ClientMessage::Ready => {
            let mut server_state = state.lock().await;

            // Find which lobby this connection is in
            let lobby_id = server_state.find_lobby_for_connection(conn_id);

            match lobby_id {
                Some(match_id) => {
                    // Mark player as ready
                    if let Some(lobby) = server_state.get_lobby_mut(&match_id) {
                        lobby.set_ready(conn_id);
                        info!("Player readied up in lobby '{}'", match_id.0);
                    }

                    // Broadcast updated lobby status
                    server_state.broadcast_lobby_status(&match_id);

                    // Check if all players are ready to start the match
                    let all_ready = server_state.get_lobby(&match_id)
                        .map(|l| l.all_ready())
                        .unwrap_or(false);

                    if all_ready {
                        info!("All players ready in lobby '{}', starting match", match_id.0);
                        match server_state.start_match_from_lobby(&match_id) {
                            Ok(mid) => {
                                info!("Match '{}' started successfully", mid.0);
                            }
                            Err(e) => {
                                warn!("Failed to start match from lobby '{}': {}", match_id.0, e);
                            }
                        }
                    }
                }
                None => {
                    server_state.send_to_connection(conn_id, ServerMessage::Error {
                        message: "Not in a lobby".into(),
                    });
                }
            }
        }
    }
}

/// Encode a ServerMessage to MessagePack bytes.
pub fn encode_server_message(msg: &ServerMessage) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec(msg).map_err(|e| e.to_string())
}

/// Decode a ClientMessage from MessagePack bytes.
pub fn decode_client_message(data: &[u8]) -> Result<ClientMessage, String> {
    rmp_serde::from_slice(data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use machine_empire_core::command::Command;

    #[test]
    fn test_message_encode_decode_ping() {
        let msg = ClientMessage::Ping { seq: 42 };
        let bytes = rmp_serde::to_vec(&msg).unwrap();
        let decoded = decode_client_message(&bytes).unwrap();
        match decoded {
            ClientMessage::Ping { seq } => assert_eq!(seq, 42),
            _ => panic!("Expected Ping"),
        }
    }

    #[test]
    fn test_message_encode_decode_cmd() {
        let msg = ClientMessage::Cmd {
            cmd: Command::Move {
                unit_ids: vec![1, 2, 3],
                target_x: 10.5,
                target_y: 20.5,
            },
        };
        let bytes = rmp_serde::to_vec(&msg).unwrap();
        let decoded = decode_client_message(&bytes).unwrap();
        match decoded {
            ClientMessage::Cmd { cmd } => match cmd {
                Command::Move { unit_ids, target_x, target_y } => {
                    assert_eq!(unit_ids, vec![1, 2, 3]);
                    assert!((target_x - 10.5).abs() < 0.001);
                    assert!((target_y - 20.5).abs() < 0.001);
                }
                _ => panic!("Expected Move command"),
            },
            _ => panic!("Expected Cmd"),
        }
    }

    #[test]
    fn test_server_message_encode() {
        let msg = ServerMessage::Pong { seq: 1, server_tick: 100 };
        let bytes = encode_server_message(&msg).unwrap();
        assert!(!bytes.is_empty());
        // Decode it back
        let decoded: ServerMessage = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            ServerMessage::Pong { seq, server_tick } => {
                assert_eq!(seq, 1);
                assert_eq!(server_tick, 100);
            }
            _ => panic!("Expected Pong"),
        }
    }

    #[test]
    fn test_encode_decode_join() {
        let msg = ClientMessage::Join {
            lobby_id: "lobby-123".into(),
            player_name: "TestPlayer".into(),
        };
        let bytes = rmp_serde::to_vec(&msg).unwrap();
        let decoded = decode_client_message(&bytes).unwrap();
        match decoded {
            ClientMessage::Join { lobby_id, player_name } => {
                assert_eq!(lobby_id, "lobby-123");
                assert_eq!(player_name, "TestPlayer");
            }
            _ => panic!("Expected Join"),
        }
    }

    #[test]
    fn test_encode_decode_error_message() {
        let msg = ServerMessage::Error { message: "Something went wrong".into() };
        let bytes = encode_server_message(&msg).unwrap();
        let decoded: ServerMessage = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            ServerMessage::Error { message } => {
                assert_eq!(message, "Something went wrong");
            }
            _ => panic!("Expected Error"),
        }
    }
}

use axum::{
    Router,
    routing::{get, post},
    extract::State,
    Json,
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::server_state::ServerState;

/// Lobby info returned by API.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LobbyInfo {
    pub id: String,
    pub name: String,
    pub player_count: u32,
    pub max_players: u32,
    pub status: String,
    pub players: Vec<LobbyPlayerInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LobbyPlayerInfo {
    pub slot: u32,
    pub name: String,
    pub is_ai: bool,
    pub ready: bool,
}

/// Request to create a new lobby.
#[derive(Debug, Deserialize)]
pub struct CreateLobbyRequest {
    pub name: String,
    pub max_players: Option<u32>,
}

/// Response for lobby creation.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateLobbyResponse {
    pub id: String,
    pub name: String,
}

/// Request to join a lobby.
#[derive(Debug, Deserialize)]
pub struct JoinLobbyRequest {
    pub player_name: String,
}

/// Request to add AI to a lobby.
#[derive(Debug, Deserialize)]
pub struct AddAiRequest {
    #[allow(dead_code)]
    pub difficulty: Option<String>,
}

/// Match info returned by API.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchInfo {
    pub id: String,
    pub tick: u32,
    pub player_count: u32,
    pub status: String,
}

/// Health check response.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub lobbies: usize,
    pub matches: usize,
}

/// Create the HTTP API router.
pub fn create_router(state: Arc<Mutex<ServerState>>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/lobbies", get(list_lobbies))
        .route("/lobbies", post(create_lobby))
        .route("/lobbies/:id/join", post(join_lobby))
        .route("/lobbies/:id/ai", post(add_ai))
        .route("/lobbies/:id/ready", post(ready_up))
        .route("/matches/:id", get(get_match))
        .with_state(state)
}

async fn health_handler(
    State(state): State<Arc<Mutex<ServerState>>>,
) -> Json<HealthResponse> {
    let server = state.lock().await;
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        lobbies: server.lobbies.len(),
        matches: server.matches.len(),
    })
}

async fn list_lobbies(
    State(state): State<Arc<Mutex<ServerState>>>,
) -> Json<Vec<LobbyInfo>> {
    let server = state.lock().await;
    let lobbies: Vec<LobbyInfo> = server.lobbies.iter().map(|(id, lobby)| {
        let players: Vec<LobbyPlayerInfo> = lobby.slots.iter().enumerate()
            .filter_map(|(i, slot)| {
                slot.as_ref().map(|s| LobbyPlayerInfo {
                    slot: i as u32,
                    name: s.name.clone(),
                    is_ai: s.is_ai,
                    ready: s.ready,
                })
            })
            .collect();

        LobbyInfo {
            id: id.0.clone(),
            name: lobby.name.clone(),
            player_count: players.len() as u32,
            max_players: lobby.max_players as u32,
            status: format!("{:?}", lobby.status),
            players,
        }
    }).collect();

    Json(lobbies)
}

async fn create_lobby(
    State(state): State<Arc<Mutex<ServerState>>>,
    Json(req): Json<CreateLobbyRequest>,
) -> Json<CreateLobbyResponse> {
    let mut server = state.lock().await;
    let max_players = req.max_players.unwrap_or(2).min(4).max(2);
    let lobby = crate::lobby::Lobby::from_http(req.name.clone(), max_players as u8);
    let id = uuid::Uuid::new_v4().to_string();
    let match_id = machine_empire_core::protocol::MatchId(id.clone());
    server.lobbies.insert(match_id, lobby);

    Json(CreateLobbyResponse {
        id,
        name: req.name,
    })
}

async fn join_lobby(
    State(state): State<Arc<Mutex<ServerState>>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<JoinLobbyRequest>,
) -> Result<Json<LobbyInfo>, axum::http::StatusCode> {
    let mut server = state.lock().await;
    let match_id = machine_empire_core::protocol::MatchId(id.clone());

    let lobby = match server.lobbies.get_mut(&match_id) {
        Some(l) => l,
        None => return Err(axum::http::StatusCode::NOT_FOUND),
    };

    // Add via HTTP slot (no connection ID needed for REST API)
    let slot_idx = lobby.slots.iter().position(|s| s.is_none());
    if let Some(idx) = slot_idx {
        lobby.slots[idx] = Some(crate::lobby::HttpPlayerSlot {
            name: req.player_name,
            is_ai: false,
            ready: false,
        });
    } else {
        return Err(axum::http::StatusCode::CONFLICT);
    }

    let players: Vec<LobbyPlayerInfo> = lobby.slots.iter().enumerate()
        .filter_map(|(i, slot)| {
            slot.as_ref().map(|s| LobbyPlayerInfo {
                slot: i as u32,
                name: s.name.clone(),
                is_ai: s.is_ai,
                ready: s.ready,
            })
        })
        .collect();

    Ok(Json(LobbyInfo {
        id,
        name: lobby.name.clone(),
        player_count: players.len() as u32,
        max_players: lobby.max_players as u32,
        status: format!("{:?}", lobby.status),
        players,
    }))
}

async fn add_ai(
    State(state): State<Arc<Mutex<ServerState>>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(_req): Json<AddAiRequest>,
) -> Result<Json<LobbyInfo>, axum::http::StatusCode> {
    let mut server = state.lock().await;
    let match_id = machine_empire_core::protocol::MatchId(id.clone());

    let lobby = match server.lobbies.get_mut(&match_id) {
        Some(l) => l,
        None => return Err(axum::http::StatusCode::NOT_FOUND),
    };

    let slot_idx = lobby.slots.iter().position(|s| s.is_none());
    if let Some(idx) = slot_idx {
        lobby.slots[idx] = Some(crate::lobby::HttpPlayerSlot {
            name: "AI".to_string(),
            is_ai: true,
            ready: true,
        });
    } else {
        return Err(axum::http::StatusCode::CONFLICT);
    }

    let players: Vec<LobbyPlayerInfo> = lobby.slots.iter().enumerate()
        .filter_map(|(i, slot)| {
            slot.as_ref().map(|s| LobbyPlayerInfo {
                slot: i as u32,
                name: s.name.clone(),
                is_ai: s.is_ai,
                ready: s.ready,
            })
        })
        .collect();

    Ok(Json(LobbyInfo {
        id,
        name: lobby.name.clone(),
        player_count: players.len() as u32,
        max_players: lobby.max_players as u32,
        status: format!("{:?}", lobby.status),
        players,
    }))
}

async fn ready_up(
    State(state): State<Arc<Mutex<ServerState>>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<LobbyInfo>, axum::http::StatusCode> {
    let mut server = state.lock().await;
    let match_id = machine_empire_core::protocol::MatchId(id.clone());

    let lobby = match server.lobbies.get_mut(&match_id) {
        Some(l) => l,
        None => return Err(axum::http::StatusCode::NOT_FOUND),
    };

    // Ready up the first non-ready human player
    let mut found = false;
    for slot in lobby.slots.iter_mut().flatten() {
        if !slot.is_ai && !slot.ready {
            slot.ready = true;
            found = true;
            break;
        }
    }

    if !found {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }

    let players: Vec<LobbyPlayerInfo> = lobby.slots.iter().enumerate()
        .filter_map(|(i, slot)| {
            slot.as_ref().map(|s| LobbyPlayerInfo {
                slot: i as u32,
                name: s.name.clone(),
                is_ai: s.is_ai,
                ready: s.ready,
            })
        })
        .collect();

    Ok(Json(LobbyInfo {
        id,
        name: lobby.name.clone(),
        player_count: players.len() as u32,
        max_players: lobby.max_players as u32,
        status: format!("{:?}", lobby.status),
        players,
    }))
}

async fn get_match(
    State(state): State<Arc<Mutex<ServerState>>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<MatchInfo>, axum::http::StatusCode> {
    let server = state.lock().await;
    let match_id = machine_empire_core::protocol::MatchId(id.clone());

    if server.matches.contains_key(&match_id) {
        Ok(Json(MatchInfo {
            id,
            tick: 0,
            player_count: 2,
            status: "running".to_string(),
        }))
    } else {
        Err(axum::http::StatusCode::NOT_FOUND)
    }
}

/// Start the HTTP API server.
pub async fn run(state: Arc<Mutex<ServerState>>, port: u16) {
    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("Failed to bind HTTP API");

    tracing::info!("HTTP API listening on http://0.0.0.0:{}", port);
    axum::serve(listener, app)
        .await
        .expect("HTTP API server error");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn test_state() -> Arc<Mutex<ServerState>> {
        Arc::new(Mutex::new(ServerState::new()))
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = test_state();
        let app = create_router(state);

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(health.status, "ok");
    }

    #[tokio::test]
    async fn test_create_lobby() {
        let state = test_state();
        let app = create_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/lobbies")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"Test Lobby"}"#))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let lobby: CreateLobbyResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(lobby.name, "Test Lobby");
        assert!(!lobby.id.is_empty());
    }

    #[tokio::test]
    async fn test_list_lobbies() {
        let state = test_state();

        // Create a lobby first
        {
            let mut s = state.lock().await;
            let lobby = crate::lobby::Lobby::from_http("Test".to_string(), 2);
            let match_id = machine_empire_core::protocol::MatchId("test-id".into());
            s.lobbies.insert(match_id, lobby);
        }

        let app = create_router(state);

        let req = Request::builder()
            .uri("/lobbies")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let lobbies: Vec<LobbyInfo> = serde_json::from_slice(&body).unwrap();
        assert_eq!(lobbies.len(), 1);
        assert_eq!(lobbies[0].name, "Test");
    }

    #[tokio::test]
    async fn test_join_lobby() {
        let state = test_state();

        {
            let mut s = state.lock().await;
            let lobby = crate::lobby::Lobby::from_http("Test".to_string(), 2);
            let match_id = machine_empire_core::protocol::MatchId("test-id".into());
            s.lobbies.insert(match_id, lobby);
        }

        let app = create_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/lobbies/test-id/join")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"player_name":"Player1"}"#))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let info: LobbyInfo = serde_json::from_slice(&body).unwrap();
        assert_eq!(info.player_count, 1);
    }

    #[tokio::test]
    async fn test_join_nonexistent_lobby() {
        let state = test_state();
        let app = create_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/lobbies/nonexistent/join")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"player_name":"Player1"}"#))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_ready_up() {
        let state = test_state();

        {
            let mut s = state.lock().await;
            let mut lobby = crate::lobby::Lobby::from_http("Test".to_string(), 2);
            lobby.slots[0] = Some(crate::lobby::HttpPlayerSlot {
                name: "Player1".to_string(),
                is_ai: false,
                ready: false,
            });
            let match_id = machine_empire_core::protocol::MatchId("test-id".into());
            s.lobbies.insert(match_id, lobby);
        }

        let app = create_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/lobbies/test-id/ready")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let info: LobbyInfo = serde_json::from_slice(&body).unwrap();
        assert!(info.players[0].ready);
    }

    #[tokio::test]
    async fn test_get_match_not_found() {
        let state = test_state();
        let app = create_router(state);

        let req = Request::builder()
            .uri("/matches/nonexistent")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

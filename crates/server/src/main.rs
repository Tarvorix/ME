mod ws_server;
mod connection;
mod server_state;
mod lobby;
mod match_runner;
mod state_broadcaster;
mod mcp;
mod http_api;

use std::sync::Arc;
use tokio::sync::Mutex;
use clap::Parser;
use tracing::info;

use server_state::ServerState;

#[derive(Parser, Debug)]
#[command(name = "machine-empire-server")]
#[command(about = "Machine Empire headless game server")]
struct Cli {
    /// WebSocket server port
    #[arg(long, default_value = "8080")]
    ws_port: u16,

    /// MCP server port (SSE + JSON-RPC)
    #[arg(long, default_value = "8081")]
    mcp_port: u16,

    /// HTTP REST API port
    #[arg(long, default_value = "8082")]
    http_port: u16,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    info!("Machine Empire Server starting...");

    let state = Arc::new(Mutex::new(ServerState::new()));

    // Start WebSocket server
    let ws_state = state.clone();
    let ws_handle = tokio::spawn(async move {
        ws_server::run(ws_state, cli.ws_port).await;
    });

    info!("WebSocket server listening on ws://0.0.0.0:{}", cli.ws_port);

    // Start MCP server
    let mcp_state = Arc::new(tokio::sync::Mutex::new(
        mcp::server::McpState::new(
            machine_empire_core::protocol::MatchConfig::default(),
            0,
        ),
    ));
    let mcp_handle = tokio::spawn(async move {
        mcp::server::run(mcp_state, cli.mcp_port).await;
    });

    info!("MCP server listening on http://0.0.0.0:{}", cli.mcp_port);

    // Start HTTP REST API
    let http_state = state.clone();
    let http_port = cli.http_port;
    let http_handle = tokio::spawn(async move {
        http_api::run(http_state, http_port).await;
    });

    info!("HTTP API listening on http://0.0.0.0:{}", cli.http_port);

    // Wait for server tasks
    tokio::select! {
        result = ws_handle => {
            if let Err(e) = result {
                tracing::error!("WebSocket server task failed: {}", e);
            }
        }
        result = mcp_handle => {
            if let Err(e) = result {
                tracing::error!("MCP server task failed: {}", e);
            }
        }
        result = http_handle => {
            if let Err(e) = result {
                tracing::error!("HTTP API server task failed: {}", e);
            }
        }
    }
}

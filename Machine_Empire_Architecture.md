# Machine Empire — Technical Architecture

**Version:** 0.2
**Date:** March 2, 2026
**Scope:** Tech stack, build pipeline, project structure, bridge layer, networking, and deployment. No game design, units, or balance.

---

## 1. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     RUST GAME CORE (lib)                        │
│                                                                 │
│  ECS Engine · Systems · Pathfinding · AI (MCTS/BT)             │
│  Fog of War · Command Processing · Game Loop                   │
│  Map · Resource Tracking · Replay Recording                    │
│                                                                 │
│  Compiles to:                                                   │
│    → wasm32-unknown-unknown  (browser client)                   │
│    → native x86_64/aarch64   (headless server)                  │
└────────┬─────────────────────────────────┬──────────────────────┘
         │                                 │
    ┌────▼────────────┐            ┌───────▼──────────────┐
    │  BROWSER CLIENT  │            │   HEADLESS SERVER    │
    │                  │            │                      │
    │  TypeScript      │  WebSocket │   Native Rust binary │
    │  PixiJS renderer │◄──────────►│   Tokio async        │
    │  Bridge layer    │            │   WebSocket server   │
    │  Touch + Mouse   │            │   MCP Server (SSE)   │
    │  Audio (Howler)  │            │   Match orchestrator  │
    │  UI overlay      │            │   AI player host     │
    └──────────────────┘            └──────────────────────┘
```

### Design Principles

- **One source of truth:** All game logic lives in Rust. The TypeScript layer is purely presentation and input.
- **Zero game logic in JS:** The client never decides if a unit can move, attack, or build. It asks Rust.
- **Minimal bridge crossings:** One WASM call per tick, shared memory buffers, no JSON serialization.
- **Same binary, two targets:** The Rust game core compiles to WASM (browser) and native (server) from identical source.

---

## 2. Tech Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **Game core** | Rust (stable) | ECS, simulation, AI, pathfinding, fog of war |
| **WASM toolchain** | wasm-bindgen + wasm-pack | Rust → WASM compilation and JS bindings |
| **Browser renderer** | PixiJS v8 | WebGL sprite rendering, tilemaps, particles |
| **Browser UI** | Preact + htm | Lightweight HUD overlay (resources, minimap, selection) |
| **Browser audio** | Howler.js | Cross-browser audio with iOS Safari support |
| **Browser bundler** | Vite | Dev server, HMR, WASM plugin |
| **Headless server** | Rust + Tokio | Async runtime for server binary |
| **WebSocket** | tokio-tungstenite (server) / native WebSocket (client) | Client-server communication |
| **MCP server** | Rust (rmcp or custom) | Model Context Protocol for AI agent connectivity |
| **Package manager** | pnpm (JS) + Cargo (Rust) | Monorepo workspace management |
| **CI/CD** | GitHub Actions | Build, test, deploy pipeline |
| **Deployment** | Docker (server) + static hosting (client) | Cloudflare Pages / Fly.io |

### Why Each Choice

**PixiJS v8** over alternatives: WebGL2 with WebGPU path landing, best iOS Safari track record, handles 10k+ sprites at 60fps, built-in sprite batching and tilemap support. Not a game framework — just a renderer.

**Preact + htm** over React: 3KB instead of 40KB, same API, no JSX build step needed. The UI layer is just health bars, resource counters, and menus — doesn't need React's weight.

**Howler.js** over Web Audio API directly: Handles iOS Safari's audio context restrictions, sprite audio sheets, spatial audio. One less thing to debug on mobile.

**Tokio** for server: Industry-standard async runtime for Rust. Handles thousands of concurrent WebSocket connections efficiently.

---

## 3. Project Structure

```
dominion/
├── Cargo.toml                      # Rust workspace root
├── package.json                    # pnpm workspace root
├── pnpm-workspace.yaml
│
├── crates/
│   ├── core/                       # Game logic library (NO rendering, NO I/O)
│   │   ├── Cargo.toml              #   [lib] — compiles to both WASM and native
│   │   └── src/
│   │       ├── lib.rs              #   Public API surface
│   │       ├── ecs/
│   │       │   ├── mod.rs
│   │       │   ├── world.rs        #   Sparse-set ECS world
│   │       │   ├── entity.rs       #   Entity ID allocator
│   │       │   ├── component.rs    #   Component storage traits
│   │       │   └── system.rs       #   System scheduling
│   │       ├── systems/
│   │       │   ├── mod.rs
│   │       │   ├── movement.rs     #   Pathfinding + position updates
│   │       │   ├── combat.rs       #   Damage, targeting, projectiles
│   │       │   ├── production.rs   #   Unit/building construction
│   │       │   ├── resource.rs     #   Economy ticks
│   │       │   ├── fog.rs          #   Fog of war computation
│   │       │   └── turret.rs       #   Auto-targeting turret AI
│   │       ├── ai/
│   │       │   ├── mod.rs
│   │       │   ├── mcts.rs         #   Monte Carlo Tree Search
│   │       │   ├── behavior_tree.rs
│   │       │   ├── influence_map.rs
│   │       │   └── planner.rs      #   Strategic decision layer
│   │       ├── pathfinding/
│   │       │   ├── mod.rs
│   │       │   ├── astar.rs
│   │       │   └── flow_field.rs   #   Optional: for large unit groups
│   │       ├── map.rs              #   Tile grid, terrain data, map gen
│   │       ├── map_gen.rs          #   Procedural map generator
│   │       ├── command.rs          #   Command enum + validation
│   │       ├── game.rs             #   Game state, tick loop, config
│   │       ├── replay.rs           #   Command recording/playback
│   │       └── types.rs            #   Shared type definitions
│   │
│   ├── wasm/                       # WASM bridge crate
│   │   ├── Cargo.toml              #   [lib] crate-type = ["cdylib"]
│   │   └── src/
│   │       ├── lib.rs              #   wasm_bindgen exports
│   │       ├── buffers.rs          #   Shared memory buffer management
│   │       └── commands.rs         #   JS → WASM command interface
│   │
│   └── server/                     # Headless server binary
│       ├── Cargo.toml              #   [bin]
│       └── src/
│           ├── main.rs             #   Entry point, CLI args
│           ├── server.rs           #   WebSocket server (Tokio)
│           ├── lobby.rs            #   Match creation, player slots
│           ├── match_runner.rs     #   Runs game instances headlessly
│           ├── protocol.rs         #   Client↔Server message format
│           └── mcp/
│               ├── mod.rs
│               ├── server.rs       #   MCP protocol handler (SSE transport)
│               ├── tools.rs        #   MCP tool definitions
│               └── resources.rs    #   MCP resource definitions
│
├── client/                         # Browser client (TypeScript + PixiJS)
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── index.html
│   └── src/
│       ├── main.ts                 #   Entry point, boot sequence
│       ├── bridge/
│       │   ├── GameBridge.ts       #   WASM ↔ JS bridge layer
│       │   ├── BufferReader.ts     #   Typed array view readers
│       │   ├── CommandWriter.ts    #   JS → WASM command encoding
│       │   └── types.ts           #   Shared type defs (mirrors Rust)
│       ├── render/
│       │   ├── GameRenderer.ts     #   Main PixiJS render loop
│       │   ├── SpritePool.ts       #   Object pool for sprites
│       │   ├── TilemapRenderer.ts  #   Map tile rendering
│       │   ├── FogRenderer.ts      #   Fog of war overlay
│       │   ├── MinimapRenderer.ts  #   Minimap display
│       │   └── ParticleManager.ts  #   Explosions, effects
│       ├── input/
│       │   ├── InputManager.ts     #   Unified input dispatcher
│       │   ├── MouseHandler.ts     #   Desktop mouse + keyboard
│       │   ├── TouchHandler.ts     #   Mobile tap, pinch, drag
│       │   ├── SelectionBox.ts     #   Drag-select rectangle
│       │   └── CameraController.ts #   Pan, zoom (mouse + touch)
│       ├── ui/
│       │   ├── HUD.ts             #   Resource bar, selection panel
│       │   ├── Minimap.ts         #   Clickable minimap
│       │   └── BuildMenu.ts       #   Building/unit production UI
│       ├── network/
│       │   ├── Socket.ts          #   WebSocket client wrapper
│       │   └── NetBridge.ts       #   Multiplayer state sync
│       ├── audio/
│       │   └── SoundManager.ts    #   Howler.js wrapper
│       └── config.ts              #   Client configuration
│
├── shared/                         # Shared constants (optional)
│   └── protocol.ts                 #   Message type definitions
│
├── assets/
│   ├── sprites/                    #   Sprite sheets (.png)
│   ├── tiles/                      #   Tileset images
│   ├── audio/                      #   Sound effects, music
│   └── ui/                         #   HUD elements
│
├── Dockerfile                      #   Server container
├── .github/
│   └── workflows/
│       ├── build.yml               #   CI: build + test
│       └── deploy.yml              #   CD: deploy client + server
└── README.md
```

---

## 4. Bridge Layer (WASM ↔ JavaScript)

### 4.1 Design Principles

1. **One WASM call per game tick** — `tick(delta_ms)` runs all systems, writes all output
2. **Shared memory buffers** — JS reads WASM linear memory via typed array views (zero copy)
3. **Fixed-size entries** — each entity is a fixed byte width; no variable-length encoding
4. **Batched events** — game events (death, shot, build complete) queued during tick, read after
5. **Commands are fire-and-forget** — JS sends small command calls, Rust validates internally

### 4.2 Memory Layout

```
WASM Linear Memory
┌────────────────────────────────────────────────────────────┐
│ Offset 0x0000                                              │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ RENDER BUFFER (read by JS every frame)                 │ │
│ │ Size: 32 bytes × 2048 entities = 65,536 bytes          │ │
│ │                                                        │ │
│ │ Per entity:                                            │ │
│ │   [0..3]   entity_id   : u32                           │ │
│ │   [4..7]   x           : f32  (world position)        │ │
│ │   [8..11]  y           : f32                           │ │
│ │   [12..13] sprite_id   : u16  (sprite sheet index)    │ │
│ │   [14..15] frame       : u16  (animation frame)       │ │
│ │   [16]     health_pct  : u8   (0-100)                 │ │
│ │   [17]     facing      : u8   (0-7 direction)         │ │
│ │   [18]     owner       : u8   (player ID)             │ │
│ │   [19]     flags       : u8   (selected, constructing)│ │
│ │   [20..23] scale       : f32                           │ │
│ │   [24..27] z_order     : f32  (render sort depth)     │ │
│ │   [28..31] reserved    : u32                           │ │
│ └────────────────────────────────────────────────────────┘ │
│                                                            │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ EVENT BUFFER (read by JS every frame)                  │ │
│ │ Size: 32 bytes × 256 events = 8,192 bytes              │ │
│ │                                                        │ │
│ │ Per event:                                             │ │
│ │   [0..1]   event_type  : u16                           │ │
│ │   [2..3]   reserved    : u16                           │ │
│ │   [4..7]   entity_id   : u32                           │ │
│ │   [8..11]  x           : f32                           │ │
│ │   [12..15] y           : f32                           │ │
│ │   [16..31] payload     : [u8; 16] (event-specific)    │ │
│ └────────────────────────────────────────────────────────┘ │
│                                                            │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ FOG GRID (read by JS every frame)                      │ │
│ │ Size: MAP_W × MAP_H bytes (e.g., 128×128 = 16,384)    │ │
│ │ Values: 0 = unexplored, 1 = explored, 2 = visible     │ │
│ └────────────────────────────────────────────────────────┘ │
│                                                            │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ UI STATE BUFFER (read by JS every frame)               │ │
│ │ Size: 256 bytes                                        │ │
│ │                                                        │ │
│ │   [0..3]   current_energy     : f32                    │ │
│ │   [4..7]   energy_income      : f32                    │ │
│ │   [8..11]  energy_expense     : f32                    │ │
│ │   [12..15] power_capacity     : f32                    │ │
│ │   [16..19] power_demand       : f32                    │ │
│ │   [20..23] game_tick          : u32                    │ │
│ │   [24..27] game_time_secs     : f32                    │ │
│ │   [28]     game_state         : u8  (playing/paused)  │ │
│ │   [29..31] padding                                    │ │
│ │   [32..63] selected_ids       : [u32; 8] (first 8)   │ │
│ │   [64..67] selected_count     : u32                    │ │
│ │   [68..195] production_queues : per-building data     │ │
│ │   [196..255] reserved                                 │ │
│ └────────────────────────────────────────────────────────┘ │
│                                                            │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ COMMAND INPUT BUFFER (written by JS, read by Rust)     │ │
│ │ Size: 64 bytes × 32 commands = 2,048 bytes             │ │
│ │                                                        │ │
│ │ Per command:                                           │ │
│ │   [0..1]   command_type : u16                          │ │
│ │   [2..3]   unit_count   : u16                          │ │
│ │   [4..7]   target_x     : f32                          │ │
│ │   [8..11]  target_y     : f32                          │ │
│ │   [12..15] target_id    : u32                          │ │
│ │   [16..19] param1       : u32  (building/unit type)   │ │
│ │   [20..63] unit_ids     : [u32; 11] (selected units)  │ │
│ └────────────────────────────────────────────────────────┘ │
│                                                            │
│ [Rest of WASM heap: ECS storage, AI state, pathfinding]   │
└────────────────────────────────────────────────────────────┘
```

### 4.3 WASM Exported Functions

```rust
// === LIFECYCLE ===
fn init_game(map_width: u32, map_height: u32, 
             player_count: u32, seed: u32)        // Initialize game state
fn tick(delta_ms: f32)                             // Advance simulation one tick
fn destroy()                                       // Free all memory

// === BUFFER POINTERS (for JS typed array views) ===
fn get_render_buffer_ptr() -> *const u8            // → RenderBuffer start
fn get_render_count() -> u32                       // → number of visible entities
fn get_event_buffer_ptr() -> *const u8             // → EventBuffer start
fn get_event_count() -> u32                        // → number of events this tick
fn get_fog_buffer_ptr() -> *const u8               // → FogGrid start
fn get_ui_state_ptr() -> *const u8                 // → UIState start
fn get_command_buffer_ptr() -> *mut u8             // → CommandBuffer start (writable)

// === COMMANDS (alternative to command buffer — direct calls) ===
fn cmd_move(unit_ids_ptr: *const u32, count: u32, 
            x: f32, y: f32)
fn cmd_attack(unit_ids_ptr: *const u32, count: u32, 
              target_id: u32)
fn cmd_attack_move(unit_ids_ptr: *const u32, count: u32, 
                   x: f32, y: f32)
fn cmd_build(building_type: u32, tile_x: u32, tile_y: u32)
fn cmd_produce(building_id: u32, unit_type: u32)
fn cmd_cancel_production(building_id: u32)
fn cmd_set_rally(building_id: u32, x: f32, y: f32)
fn cmd_stop(unit_ids_ptr: *const u32, count: u32)

// === QUERIES (for UI, selection, tooltips) ===
fn get_entity_info(entity_id: u32) -> u32          // → offset into info buffer
fn get_buildable_at(tile_x: u32, tile_y: u32) -> u32  // → bitmask of valid buildings
fn get_map_tile(tile_x: u32, tile_y: u32) -> u32  // → terrain type + metadata
fn get_map_width() -> u32
fn get_map_height() -> u32

// === AI CONTROL ===
fn set_ai_difficulty(player_id: u32, difficulty: u32)
fn ai_get_suggestion(player_id: u32) -> u32        // → offset into suggestion buffer

// === REPLAY ===
fn get_replay_data_ptr() -> *const u8
fn get_replay_data_len() -> u32
fn load_replay(data_ptr: *const u8, len: u32)
fn replay_tick() -> u32                             // → 0 = more ticks, 1 = done
```

### 4.4 Frame Flow

```
  60 FPS RENDER LOOP (client/src/main.ts)
  ══════════════════════════════════════════════════════════

  ┌─ INPUT PHASE ────────────────────────────────────┐
  │ InputManager polls mouse/touch/keyboard           │  ~0.1ms
  │ Converts to commands (move, attack, build)        │
  │ Writes commands via cmd_*() WASM calls            │  ~0.05ms
  └───────────────────────────────┬───────────────────┘
                                  │
  ┌─ SIMULATION PHASE ───────────▼───────────────────┐
  │ bridge.tick(delta)                                │
  │   → Rust runs ALL systems:                        │  ~3-8ms
  │     CommandProcessor → Movement → Combat →        │
  │     Production → Resource → FogOfWar → AI →       │
  │     RenderBufferWrite → EventBufferWrite          │
  └───────────────────────────────┬───────────────────┘
                                  │
  ┌─ READ PHASE ─────────────────▼───────────────────┐
  │ BufferReader reads render buffer (typed views)    │  ~0.1ms
  │ BufferReader reads event buffer                   │
  │ BufferReader reads fog grid                       │
  │ BufferReader reads UI state                       │
  └───────────────────────────────┬───────────────────┘
                                  │
  ┌─ RENDER PHASE ───────────────▼───────────────────┐
  │ SpritePool syncs PixiJS sprites to render buffer  │  ~2-4ms
  │ FogRenderer updates fog overlay texture           │
  │ ParticleManager spawns effects from events        │
  │ HUD updates resource display                      │
  │ PixiJS renders frame to WebGL                     │  ~2-4ms
  └───────────────────────────────┬───────────────────┘
                                  │
  ┌─ AUDIO PHASE ────────────────▼───────────────────┐
  │ SoundManager processes events → plays sounds      │  ~0.1ms
  └──────────────────────────────────────────────────┘

  Total: ~8-16ms per frame → 60fps achievable
```

### 4.5 Game Tick vs Render Frame

The simulation and rendering run at different rates:

```
Simulation:  20 ticks/second (50ms per tick) — deterministic
Rendering:   60 fps (16ms per frame) — interpolated

Frame 1:  [tick] ─── render ─── render ─── [tick] ─── render ─── render ───
          t=0ms      t=16ms     t=33ms      t=50ms     t=66ms     t=83ms

The renderer interpolates entity positions between ticks for smooth movement.
Rust writes both "current" and "previous" positions; JS lerps between them.
```

### 4.6 Map & Tile Data Model

The RTS battle map is a 64×64 isometric tile grid. Each tile is a compact struct in Rust:

```rust
// crates/core/src/map.rs

#[repr(u8)]
enum TerrainType {
    Open = 0,        // Walkable, no modifier
    Impassable = 1,  // Blocks movement and vision
    // Future:
    // Rough = 2,     // Slow infantry, hover tanks unaffected
    // Elevated = 3,  // Vision + damage bonus
    // Hazard = 4,    // Damage over time
    // Cover = 5,     // Damage reduction
    // Road = 6,      // Movement speed bonus
}

#[repr(C)]
struct Tile {
    terrain: u8,        // TerrainType
    elevation: u8,      // 0 for Phase 1, future: high ground
    sprite_variant: u8, // Visual variant index (avoids tiling repetition)
    flags: u8,          // Bitfield: has_capture_point, is_entry_zone, etc.
}

struct BattleMap {
    width: u32,                      // 64
    height: u32,                     // 64
    tiles: Vec<Tile>,                // width × height = 4,096 tiles
    capture_points: Vec<CapturePoint>,
    entry_zones: Vec<EntryZone>,
}

// Total map data: 64 × 64 × 4 bytes = 16 KB
// Trivially fits in WASM memory, fast to iterate for pathfinding
```

**Movement cost lookup** is a simple table indexed by terrain type and unit class:

```rust
// movement_cost[terrain_type][unit_class] → f32 multiplier
// 1.0 = normal speed, >1.0 = slower, 0.0 = impassable
const MOVEMENT_COST: [[f32; 3]; 7] = [
    // Open:       Thrall  Sentinel  HoverTank
    [1.0, 1.0, 1.0],
    // Impassable:
    [0.0, 0.0, 0.0],
    // Rough (future):
    [1.5, 1.5, 1.0],  // Hover tanks ignore rough terrain
    // ... extensible
];
```

Adding a new terrain type = add an enum variant + one row in the cost table. Zero refactoring.

**Edge tile auto-selection** uses a bitmask lookup. Each tile checks its 8 neighbors and produces an 8-bit mask where each bit represents whether that neighbor is the same terrain type. The mask maps to a sprite index in the transition tile atlas:

```rust
fn get_autotile_index(map: &BattleMap, x: u32, y: u32) -> u8 {
    let base = map.get(x, y).terrain;
    let mut mask: u8 = 0;
    // Check 8 neighbors, set bits for matching terrain
    for (i, (dx, dy)) in NEIGHBORS_8.iter().enumerate() {
        if map.get_safe(x + dx, y + dy).terrain == base {
            mask |= 1 << i;
        }
    }
    AUTOTILE_LOOKUP[mask as usize]  // → sprite index (0-47)
}
```

**Map data flows to JS** via the existing fog buffer (Section 4.2). The fog grid already provides per-tile visibility state. Terrain data is sent once at battle start via a separate buffer or a one-time WASM query (`get_map_tile()`), since terrain doesn't change during a battle.

---

## 5. Headless Server Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    HEADLESS SERVER BINARY                     │
│                    (crates/server)                            │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ Match Orchestrator                                    │   │
│  │                                                       │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐           │   │
│  │  │ Match #1 │  │ Match #2 │  │ Match #3 │  ...      │   │
│  │  │          │  │          │  │          │           │   │
│  │  │ GameCore │  │ GameCore │  │ GameCore │           │   │
│  │  │ Player[] │  │ Player[] │  │ Player[] │           │   │
│  │  │ Replay   │  │ Replay   │  │ Replay   │           │   │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘           │   │
│  │       │              │              │                 │   │
│  └───────┼──────────────┼──────────────┼─────────────────┘   │
│          │              │              │                     │
│  ┌───────▼──────────────▼──────────────▼─────────────────┐   │
│  │ Connection Manager                                     │   │
│  │                                                        │   │
│  │  WebSocket Server (port 8080)                         │   │
│  │    ├── Human player connections                        │   │
│  │    └── Spectator connections                           │   │
│  │                                                        │   │
│  │  MCP Server (port 8081, SSE transport)                │   │
│  │    ├── AI agent connections (Claude, etc.)             │   │
│  │    └── Tool calls → Commands, Resources → Game state  │   │
│  │                                                        │   │
│  │  HTTP API (port 8082)                                 │   │
│  │    ├── GET  /lobbies          — list open games        │   │
│  │    ├── POST /lobbies          — create game            │   │
│  │    ├── POST /lobbies/:id/join — join game              │   │
│  │    ├── GET  /matches/:id      — match status           │   │
│  │    └── GET  /replays/:id      — download replay        │   │
│  └────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

### Server Game Loop

```rust
// The server runs the SAME game core as the browser client.
// No WASM — it's a native Rust binary calling the core lib directly.

async fn run_match(config: MatchConfig) {
    let mut game = core::Game::new(config);
    let mut interval = tokio::time::interval(Duration::from_millis(50)); // 20 ticks/sec
    
    loop {
        interval.tick().await;
        
        // 1. Collect commands from all players (WebSocket + MCP + AI)
        let commands = collect_commands(&players).await;
        
        // 2. Feed commands into game core
        for cmd in commands {
            game.execute_command(cmd);
        }
        
        // 3. Tick the game simulation
        game.tick(50.0); // 50ms delta
        
        // 4. Get fog-filtered state for each player
        for player in &players {
            let state = game.get_player_state(player.id);
            player.send_state_delta(state).await;
        }
        
        // 5. Record tick for replay
        game.record_tick(&commands);
        
        // 6. Check win condition
        if game.is_over() { break; }
    }
}
```

---

## 6. Client ↔ Server Protocol

### 6.1 Message Format (MessagePack over WebSocket)

MessagePack chosen over JSON for ~30% smaller payloads and faster parse times.

```typescript
// Client → Server
type ClientMessage =
  | { t: 'join',    lobby_id: string, token: string }
  | { t: 'ready' }
  | { t: 'cmd',     cmd: Command }      // Same Command types as local
  | { t: 'ping',    seq: number }

// Server → Client  
type ServerMessage =
  | { t: 'state',   tick: number, entities: Uint8Array, 
                     events: Uint8Array, fog: Uint8Array, ui: Uint8Array }
  | { t: 'full',    tick: number, full_state: Uint8Array }  // On join/reconnect
  | { t: 'pong',    seq: number, server_tick: number }
  | { t: 'lobby',   players: PlayerInfo[], status: string }
  | { t: 'end',     winner: number, replay_id: string }
```

### 6.2 State Sync Strategy

```
LOCAL PLAY:     Client runs game core in WASM directly. No server.
                Input → WASM tick → render. Lowest latency.

ONLINE PLAY:    Server is authoritative. Client predicts locally.
                
                Client                          Server
                  │                               │
                  ├── cmd(move units) ────────────►│
                  │                               │── validate
                  │   [predict locally in WASM]    │── execute
                  │                               │── broadcast
                  │◄── state(tick N) ─────────────┤
                  │                               │
                  │   [reconcile: compare predicted│
                  │    state vs server state,      │
                  │    snap if diverged]           │
```

---

## 7. MCP Server Interface

The MCP server runs as part of the headless server process, exposing game interaction via the Model Context Protocol.

### Transport

SSE (Server-Sent Events) on port 8081 — standard MCP transport.

### Tools (Actions)

```
game/move_units        — Move units to target position
game/attack            — Attack a specific enemy entity
game/attack_move       — Move to target, engaging enemies en route
game/stop              — Halt selected units
game/build             — Place a building at tile coordinates
game/produce_unit      — Queue unit production at a building
game/cancel_production — Cancel production queue item
game/set_rally_point   — Set rally point for production building
game/get_suggestions   — Ask built-in AI for strategic advice
```

### Resources (State Queries)

```
game://state              — Full visible game state (fog-filtered)
game://state/my_units     — Player's units with positions, health, orders
game://state/my_buildings — Player's buildings with status, production queues
game://state/enemies      — Visible enemy entities
game://state/map          — Explored terrain data
game://state/economy      — Energy, income, expenses, power capacity
game://state/fog          — Fog of war grid
game://state/threats      — Influence map threat assessment
game://match              — Match metadata, time, players, scores
```

### Agent Connection Flow

```
Agent connects via MCP → receives tool/resource list
Agent reads game://state → understands current situation
Agent calls game/build, game/produce_unit, etc.
Agent receives notifications on state changes
Agent reads game://state/threats → plans strategy
Agent calls game/get_suggestions → gets built-in AI advice
```

---

## 8. Build Pipeline

### 8.1 Development

```bash
# Terminal 1: Watch and rebuild WASM on Rust changes
cd crates/wasm
wasm-pack build --target web --dev --out-dir ../../client/src/pkg
cargo watch -s "wasm-pack build --target web --dev --out-dir ../../client/src/pkg"

# Terminal 2: Vite dev server with HMR
cd client
pnpm dev    # → http://localhost:5173

# Terminal 3: (Optional) Run headless server
cd crates/server
cargo run -- --port 8080 --mcp-port 8081
```

### 8.2 Vite Configuration

```typescript
// client/vite.config.ts
import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  plugins: [wasm(), topLevelAwait()],
  build: {
    target: 'es2022',           // iOS Safari 15.4+
    outDir: 'dist',
    assetsInlineLimit: 0,       // Don't inline WASM
  },
  server: {
    headers: {
      // Required for SharedArrayBuffer (future multi-threading)
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
  },
});
```

### 8.3 Production Build

```bash
# 1. Build optimized WASM
cd crates/wasm
wasm-pack build --target web --release --out-dir ../../client/src/pkg
wasm-opt -O3 -o pkg/dominion_core_bg.wasm pkg/dominion_core_bg.wasm

# 2. Build client
cd client
pnpm build   # → client/dist/

# 3. Build server
cd crates/server
cargo build --release   # → target/release/dominion-server

# 4. Docker image for server
docker build -t dominion-server .
```

### 8.4 Cargo Workspace

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/core",
    "crates/wasm",
    "crates/server",
]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
rand = "0.8"
```

```toml
# crates/core/Cargo.toml
[package]
name = "dominion-core"
version = "0.1.0"
edition = "2024"

[lib]
# Compiles as both a regular lib (for server) and source for WASM crate

[dependencies]
serde = { workspace = true }
rand = { workspace = true }

[features]
default = []
headless = []   # Strips any optional visual helpers
```

```toml
# crates/wasm/Cargo.toml
[package]
name = "dominion-wasm"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
dominion-core = { path = "../core" }
wasm-bindgen = "0.2"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1     # Max optimization, slower build
strip = true
```

```toml
# crates/server/Cargo.toml
[package]
name = "dominion-server"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "dominion-server"
path = "src/main.rs"

[dependencies]
dominion-core = { path = "../core" }
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.24"
serde = { workspace = true }
serde_json = "1"
rmp-serde = "1"       # MessagePack serialization
clap = { version = "4", features = ["derive"] }  # CLI args
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## 9. iOS Safari Considerations

### Mandatory HTML Setup

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, 
    maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
  <meta name="apple-mobile-web-app-capable" content="yes">
  <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    html, body { 
      width: 100%; height: 100%; overflow: hidden;
      touch-action: none;           /* Prevent browser gestures */
      -webkit-touch-callout: none;  /* Disable callout menu */
      -webkit-user-select: none;    /* Disable text selection */
      overscroll-behavior: none;    /* Prevent bounce scroll */
    }
    canvas { display: block; width: 100%; height: 100%; }
  </style>
</head>
<body>
  <canvas id="game"></canvas>
  <script type="module" src="/src/main.ts"></script>
</body>
</html>
```

### Technical Constraints

| Constraint | Limit | Mitigation |
|-----------|-------|------------|
| WebGL context memory | ~1GB before tab kill | Keep sprite atlases ≤ 2048×2048, pool textures |
| WASM memory | ~2GB max | Pre-allocate, avoid dynamic growth |
| Audio context | Must init on user gesture | Show "tap to start" splash |
| Touch delay | 300ms default | `touch-action: manipulation` eliminates it |
| Viewport resize | Address bar show/hide | Listen to `visualViewport.resize` |
| FPS target | 30fps on iPhone SE (2nd gen) | Profile, reduce particles on low-end |
| No SharedArrayBuffer | Disabled without COOP/COEP | Single-threaded WASM for now |

### PWA Support

```json
// manifest.json
{
  "name": "Machine Empire",
  "short_name": "MachEmpire",
  "display": "fullscreen",
  "orientation": "landscape",
  "background_color": "#000000",
  "theme_color": "#000000",
  "icons": [...]
}
```

---

## 10. Deployment

### Client (Static Files)

```
Build output (client/dist/) → Cloudflare Pages or Vercel

Files served:
  index.html            ~2 KB
  main.[hash].js        ~50-80 KB (gzipped)
  dominion_core_bg.wasm ~200-500 KB (gzipped, depends on game complexity)
  sprites/*.png         ~500 KB - 2 MB
  tiles/*.png           ~200 KB
  audio/*               ~1-3 MB

Total cold load: ~2-4 MB (acceptable for a game)
```

### Server (Docker)

```dockerfile
FROM rust:1.85 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p dominion-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/dominion-server /usr/local/bin/
EXPOSE 8080 8081 8082
CMD ["dominion-server", "--ws-port", "8080", "--mcp-port", "8081", "--api-port", "8082"]
```

Deploy to: Fly.io, Railway, or any container host.

### Environment Architecture

```
Production:
  ┌─────────────────┐     ┌──────────────────────┐
  │ Cloudflare Pages │     │ Fly.io (1-N instances)│
  │ (static client)  │────►│ dominion-server       │
  │                   │ WS  │  ├── WebSocket :8080  │
  └─────────────────┘     │  ├── MCP SSE   :8081  │
                           │  └── HTTP API  :8082  │
                           └──────────────────────┘
```

---

## 11. Testing Strategy

| Layer | Tool | What's Tested |
|-------|------|---------------|
| Rust core unit tests | `cargo test` | ECS operations, pathfinding, combat math, fog calculations |
| Rust integration tests | `cargo test --test integration` | Full game scenarios (build base → produce units → attack → win) |
| AI regression tests | Custom harness | AI vs AI 100-game batches, win rate tracking per difficulty |
| WASM bridge tests | wasm-pack test --chrome | Buffer layout correctness, command round-trips |
| Client unit tests | Vitest | Input handling, buffer readers, sprite pool logic |
| E2E tests | Playwright | Full game session in browser, iOS Safari emulation |
| Performance benchmarks | Criterion (Rust) + browser perf API | Tick time budget, frame time budget, memory usage |

### Determinism Testing

```rust
// The game core must be deterministic given the same seed + commands.
// This test runs the same game twice and asserts identical final state.
#[test]
fn test_determinism() {
    let seed = 42;
    let commands = load_test_commands("test_game_1.replay");
    
    let state_a = run_game(seed, &commands);
    let state_b = run_game(seed, &commands);
    
    assert_eq!(state_a.hash(), state_b.hash());
}
```

---

## 12. Development Phases (Technical Milestones Only)

### Phase 1 — Foundation

- [ ] Cargo workspace with core/wasm/server crates
- [ ] Minimal ECS (world, entity, component storage, system runner)
- [ ] WASM bridge: `init_game()`, `tick()`, buffer pointer exports
- [ ] Vite + PixiJS rendering a grid of tiles from WASM fog buffer
- [ ] One entity type moving via A* (render buffer → sprite sync)
- [ ] Mouse click → `cmd_move()` → unit moves
- [ ] iOS Safari tested and working

### Phase 2 — Game Systems

- [ ] All building/unit types in ECS with blueprints
- [ ] Full command set (move, attack, build, produce)
- [ ] Combat system with damage, death, projectiles
- [ ] Fog of war (circle vision, 3-state grid)
- [ ] Resource system (energy income/spending)
- [ ] Touch input (tap-select, pinch-zoom, drag-pan)
- [ ] Basic HUD (resources, selection, build menu)

### Phase 3 — AI & Server

- [ ] Behavior tree engine + basic tactical AI
- [ ] MCTS strategic planner
- [ ] Influence maps
- [ ] Headless server binary (Tokio + WebSocket)
- [ ] MCP server with tool/resource definitions
- [ ] AI player can play a full match headlessly
- [ ] Agent (Claude) can connect via MCP and play

### Phase 4 — Multiplayer & Polish

- [ ] Client-server state sync via WebSocket
- [ ] Lobby system (HTTP API)
- [ ] Client-side prediction + reconciliation
- [ ] Replay recording and playback
- [ ] Audio (Howler.js)
- [ ] PWA manifest + offline support
- [ ] Performance profiling + optimization pass
- [ ] Docker deployment pipeline

# Machine Empire - Phase 1 Foundation Todo

## Chunk 1: Project Scaffolding & Build Pipeline
- [x] Create Cargo workspace (root Cargo.toml, rust-toolchain.toml, .gitignore)
- [x] Create core crate stub (Cargo.toml + lib.rs)
- [x] Create wasm crate stub (Cargo.toml + lib.rs)
- [x] Create server crate stub (Cargo.toml + main.rs)
- [x] Create client scaffolding (package.json, tsconfig, vite.config, index.html, main.ts)
- [x] Create pnpm workspace (pnpm-workspace.yaml, root package.json)
- [x] Verify: cargo build, pnpm wasm:build, pnpm dev

## Chunk 2: Minimal ECS in Rust Core
- [x] Entity allocator with generational IDs
- [x] SparseSet component storage
- [x] World struct (entities + components + resources)
- [x] SystemRunner
- [x] Types (Direction, SpriteId, AnimState)
- [x] Unit tests (33 passing)

## Chunk 3: Game State, Map, and Components
- [x] ECS Components (Position, PreviousPosition, UnitType, PathState, RenderState, etc.)
- [x] BattleMap with procedural generation (all open terrain, random variants)
- [x] Game struct with tick loop
- [x] Command enum

## Chunk 4: A* Pathfinding & Movement System
- [x] A* pathfinding on tile grid
- [x] Command processor system
- [x] Movement system
- [x] Animation system
- [x] Render buffer writer system
- [x] Unit tests

## Chunk 5: WASM Bridge Layer
- [x] Export init_game, tick, buffer pointers, cmd_move_unit
- [x] Verify wasm-pack build

## Chunk 6: PixiJS Renderer - Isometric Tilemap
- [x] Config constants
- [x] GameBridge, BufferReader, types
- [x] IsoUtils (coordinate conversion)
- [x] TerrainGenerator (diamond tiles from square textures via CanvasSource)
- [x] TilemapRenderer
- [x] CameraController (middle-mouse drag)
- [x] Main.ts integration
- [x] Tilemap renders correctly (4096 tiles, 4 terrain variants)

## Chunk 7: Sprite Rendering - Thralls from Atlas
- [x] SpritePool (render buffer -> PixiJS sprites)
- [x] GameRenderer (orchestrator with fixed timestep)
- [x] Spawn 5 test Thralls near map center
- [x] Atlas loading (11 spritesheets)
- [x] Thrall scale corrected to 48/512

## Chunk 8: Click-to-Move Interaction
- [x] InputManager (left-click select, right-click move)
- [x] SelectionIndicator
- [x] MoveOrderIndicator
- [x] **FIX: Sprite facing direction** - atlas had E/W mirrored, added ATLAS_DIR_NAMES mapping in SpritePool.ts

---

# Phase 2: Game Systems

## Chunk 9: Cleanup + Unit Blueprints + Health + Spawn Functions
- [x] Remove Phase 1 debug logging (SpritePool.ts, InputManager.ts)
- [x] Create blueprints.rs with UnitBlueprint struct and static data for all 5 types
- [x] Add Health, CombatState, VisionRange, Deployed components
- [x] Expand Command enum (Attack, AttackMove, Build, Produce, CancelProduction, SetRally)
- [x] Generic spawn_unit() function using blueprint data
- [x] spawn_command_post() function
- [x] render_buffer_system reads health_pct from Health component
- [x] Unit tests for blueprints and spawning (45 tests passing)
- [x] Verify: cargo test, pnpm wasm:build

## Chunk 10: Combat System (Attack, Damage, Death, Despawn)
- [x] combat_system (targeting, range check, damage, cooldown, attack animation)
- [x] death_cleanup_system (death animation, DeathTimer, entity despawn)
- [x] AttackMoveTarget, DeathTimer components
- [x] Command processor handles Attack, AttackMove commands
- [x] Movement system integration (stop when in range, chase logic)
- [x] EventType enum, event buffer helper (write_event)
- [x] WASM exports: cmd_attack, cmd_attack_move
- [x] Client bridge: attack command methods, EventType enum
- [x] Unit tests for combat, death, events (53 tests passing)

## Chunk 11: Fog of War
- [x] FogGrid resource (per-player, 3-state: Unexplored/Explored/Visible)
- [x] fog_system (circle vision computation, squared distance, bounded iteration)
- [x] render_buffer_system filters enemies by fog visibility
- [x] WASM exports: get_fog_buffer_ptr, get_fog_buffer_len
- [x] FogRenderer.ts (per-tile diamond overlay, 3 visual states)
- [x] GameBridge.ts fog buffer method
- [x] GameRenderer.ts integrates FogRenderer
- [x] Unit tests for fog computation (60 tests passing)

## Chunk 12: Resource System + Conscription Strain
- [x] PlayerEconomy struct (energy_bank, income, upkeep, strain)
- [x] resource_system (income, upkeep calculation, strain decay)
- [x] Strain income penalty (thresholds: 0-30→0%, 30-50→5-15%, 50-70→15-30%, 70-90→30-50%, 90+→50%+)
- [x] Strain production penalty (similar thresholds)
- [x] Strain squared recovery curve
- [x] UIStateBuffer (256 bytes, energy/income/expense/strain/game_tick)
- [x] WASM exports: get_ui_state_ptr, get_ui_state_len
- [x] Client bridge: getUIState(), readUIState()
- [x] Unit tests for economy and strain (69 tests passing)

## Chunk 13: Production System (Forge Queues, Unit Spawning)
- [x] ProductionJob, PlayerProduction structs
- [x] production_system (advance queues, deduct energy, add strain, spawn units)
- [x] Command processor handles Produce, CancelProduction, SetRally
- [x] 1 infantry + 1 armor production line per player (start)
- [x] Units spawn at Command Post location
- [x] Rally point system (move to rally after spawn)
- [x] Write production queue data to UIStateBuffer
- [x] WASM exports: cmd_produce, cmd_cancel_production, cmd_set_rally
- [x] Unit tests for production (79 tests passing)

## Chunk 14: Event Buffer Integration + Visual Effects
- [x] Event buffer functional end-to-end (reset count per tick, write_event helper)
- [x] Combat writes Shot events, death_cleanup writes Death events
- [x] Production writes UnitSpawned events
- [x] Client BufferReader.readEvent() method
- [x] ParticleManager.ts (muzzle flash, death effect, spawn effect)
- [x] GameRenderer processes events and spawns effects
- [x] Unit tests for event buffer (79 tests passing)

## Chunk 15: Touch Input + Multi-Select
- [x] TouchHandler.ts (tap, drag, pinch, two-finger pan gestures)
- [x] SelectionBox.ts (drag-select rectangle)
- [x] InputManager refactor: selectedEntities Set, multi-select, touch integration
- [x] CameraController: mouse wheel zoom, pinch zoom, touch pan, zoom clamping (0.5x-2.0x)
- [x] SelectionIndicator: pool of ellipses for multi-select (64 max)
- [x] WASM batch commands: cmd_move_units, cmd_attack_target (loop-based)
- [ ] Manual testing on desktop and iOS Safari

## Chunk 16: HUD with Preact + htm
- [x] Install preact + htm dependencies
- [x] HUD.ts orchestrator (overlay div above canvas)
- [x] ResourceBar.ts (energy, income, expense, net rate, strain meter with color thresholds)
- [x] SelectionPanel.ts (single unit info, multi-unit group display)
- [x] BuildMenu.ts (produce buttons, production queue progress bars, cancel)
- [x] HealthBars.ts (PixiJS Graphics pool, positioned above sprites, green/yellow/red)
- [x] styles.ts (CSS-in-JS, dark industrial theme, responsive)
- [x] index.html: add hud-root overlay div
- [ ] Manual testing on desktop and iOS Safari

---

# Phase 3: AI & Server

## Chunk 17: Serde Serialization + Network Protocol Types
- [x] Add serde_json, rmp-serde to workspace deps
- [x] Add serde derives to core types (Direction, SpriteId, AnimState, EventType)
- [x] Add serde derives to Command enum (+ Clone, Debug)
- [x] Add serde derives to components (Position, UnitType, Health, etc.)
- [x] Add serde derives to blueprints (UnitBlueprint, ProductionLine)
- [x] Create protocol.rs (MatchId, PlayerInfo, MatchConfig, EntitySnapshot, EventSnapshot, EconomySnapshot, ProductionLineSnapshot, ClientMessage, ServerMessage)
- [x] Create state_snapshot.rs (snapshot_entities_for_player, snapshot_events, snapshot_economy, snapshot_production, snapshot_fog, snapshot_map_tiles)
- [x] Unit tests for serialization roundtrips and snapshot extraction (94 tests passing)

## Chunk 18: Server Skeleton + WebSocket
- [x] Update server Cargo.toml (tokio-tungstenite, futures-util, clap, tracing, uuid, etc.)
- [x] Rewrite main.rs (CLI args, tracing, ServerState, WebSocket listener)
- [x] Create ws_server.rs (TCP listener, WebSocket upgrade, MessagePack encode/decode)
- [x] Create connection.rs (Connection struct, ConnectionId)
- [x] Create server_state.rs (ServerState with connections/matches/lobbies)
- [x] Unit tests for message encode/decode (5 tests passing)

## Chunk 19: Match Orchestrator
- [x] Create lobby.rs (Lobby, PlayerSlot, LobbyStatus, add/remove/ready)
- [x] Create match_runner.rs (MatchRunner, PlayerHandle, async game loop at 20Hz, win detection)
- [x] Route WebSocket messages to lobby/match
- [x] Add spawn_forge(), check_forge_alive(), spawn_starting_units() to game.rs
- [x] Add Send bounds to ECS World for tokio::spawn compatibility
- [x] Unit tests for lobby, match runner, win condition (113 tests passing)

## Chunk 20: Influence Maps
- [x] Create ai/mod.rs and ai/influence_map.rs
- [x] InfluenceGrid resource (threat, friendly_strength, tension, vulnerability, density)
- [x] update() with DPS-weighted linear distance falloff
- [x] Query helpers (get_threat, get_tension, highest_threat_tile, find_safe_position)
- [x] Unit tests for influence computation (12 tests, 106 core total)

## Chunk 21: Behavior Tree Engine
- [x] Create ai/behavior_tree.rs
- [x] BtNode enum (Sequence, Selector, Inverter, Succeeder, Condition, Action)
- [x] ConditionId and ActionId enums
- [x] BehaviorTree (flat array), BtState (per-entity), BtContext
- [x] evaluate() recursive tree evaluation
- [x] build_combat_bt() predefined template
- [x] Unit tests for all node types and combat BT (20 tests, 126 core total)

## Chunk 22: Tactical AI System
- [x] Create ai/tactical.rs
- [x] AiControlled component, BtTemplateId, AiBlackboard, BtTemplates resources
- [x] ai_tactical_system (iterate AI entities, build context, evaluate BT, push commands)
- [x] Register in game.rs system order (after fog, before production)
- [x] Unit tests for AI entity behavior (133 core + 19 server = 152 tests passing)

## Chunk 23: State Broadcasting + Fog Filtering
- [x] Create state_broadcaster.rs (StateBroadcaster, per-player fog-filtered state)
- [x] Flesh out state_snapshot.rs fog filtering (snapshot_events_for_player with fog + ownership)
- [x] Integrate StateBroadcaster into match_runner.rs
- [x] Unit tests for fog-filtered broadcasting (135 core + 27 server = 162 tests passing)

## Chunk 24: Strategic AI (MCTS)
- [x] Create ai/mcts.rs
- [x] MctsState (8x8 sectors, economy snapshots, forge alive)
- [x] StrategicAction enum (ProduceThrall/Sentinel/HoverTank, AttackSector, DefendSector, Retreat, DoNothing)
- [x] MctsPlanner (UCB1 selection, expansion, rollout, backpropagation)
- [x] evaluate_state() heuristic (army strength 50%, economy 30%, survival 20%)
- [x] Unit tests for MCTS (146 core + 27 server = 173 tests passing)

## Chunk 25: AI Player Integration
- [x] Create ai/player.rs
- [x] AiDifficulty (Easy/Normal/Hard), AiPlayer, AiPlayers resource
- [x] ai_strategic_system (MCTS every N ticks, translate to Commands)
- [x] add_ai_player() in game.rs, register ai_strategic_system in system runner
- [x] Integrate with match_runner.rs (auto-register AI players on match start)
- [x] Unit tests for AI player (production, combat, full match) (155 core + 27 server = 182 tests passing)

## Chunk 26: MCP Server
- [x] Add axum, tower, tower-http deps
- [x] Create mcp/ module (mod.rs, types.rs, server.rs, tools.rs, resources.rs)
- [x] JSON-RPC 2.0 types and SSE endpoint
- [x] 7 tools (move_units, attack, attack_move, produce_unit, cancel_production, set_rally_point, get_suggestions)
- [x] 9 resources (state, my_units, my_buildings, enemies, map, economy, fog, threats, match)
- [x] Start MCP server alongside WebSocket in main.rs
- [x] Unit tests for tools and resources (155 core + 56 server = 211 tests passing)

## Chunk 27: Integration Testing + Headless AI Match
- [x] AI vs AI headless match test (2 players, 4000 tick limit)
- [x] AI produces units test
- [x] Determinism test (same seed = same result)
- [x] Server message roundtrip integration test
- [x] MCP tool definitions integration test
- [x] End-to-end command flow integration test
- [x] hash_game_state() for determinism verification
- [x] Verify: cargo test --all passes (222 tests passing)

---

# Phase 4: Game Mechanics + Multiplayer & Polish

## Chunk 28: Terrain Types + Movement Cost (232 tests passing, 0 warnings)
- [x] Expand TerrainType enum: Open(0), Impassable(1), Rough(2), Elevated(3), Hazard(4), Cover(5), Road(6)
- [x] Add movement_cost(terrain, unit_kind) -> f32 (HoverTank always 1.0 except Impassable)
- [x] Add damage_reduction(terrain) -> f32 (Cover = 0.25, Elevated = 0.15)
- [x] Add HAZARD_DPS constant (2.0 DPS to non-hover ground units)
- [x] Update generate_simple() to place terrain clusters (~15% Rough, ~5% Elevated, ~3% Hazard, ~8% Cover, ~5% Road, ~62% Open, ~2% Impassable)
- [x] Spawn corner safety margin keeps corners Open for fair start positions
- [x] Add unit_kind: Option<SpriteId> param to find_path(), neighbor cost uses movement_cost
- [x] Pass unit kind in command_processor, combat chase, production rally
- [x] Movement speed multiplied by 1.0/movement_cost per tile in movement system
- [x] Combat applies Cover/Elevated damage reduction to attacks
- [x] Hazard tiles deal 2 DPS to non-hover ground units (not buildings, not HoverTank)
- [x] Unit tests for terrain mechanics (movement_cost, damage_reduction, pathfinding road preference, hover tank ignoring terrain, etc.)
- [x] Updated combat tests to use spawn corner positions (guaranteed Open terrain)

## Chunk 29: Capture Points (245 tests passing, 0 warnings)
- [x] CapturePointState component (capture_radius=3.0, capture_speed=5.0, owner, progress 0-100, contested, point_index)
- [x] capture_system(): proximity capture, contested state, decay, sqrt(count) speed scaling
- [x] spawn_capture_points(): deterministic odd-count placement (3-7 points) spread across map
- [x] CapturePointCounts resource for win condition checking
- [x] Added SpriteId::CapturePoint=5, EventType::CaptureProgress=5, CaptureComplete=6
- [x] CapturePoint blueprint (1000 HP, 6.0 vision, no attack/movement)
- [x] CapturePointSnapshot in protocol + snapshot_capture_points() in state_snapshot
- [x] capture_points field added to ServerMessage::State and FullState
- [x] Updated all ServerMessage constructors (state_broadcaster, match_runner, protocol tests, integration tests)
- [x] Updated AI mcts.rs to handle CapturePoint sprite type
- [x] 13 new capture point tests (spawn count, odd enforcement, neutral start, unique indices, spread, single player capture, contested pauses, speed scaling, owner flip at 100, recapture, decay, events, counts resource)

## Chunk 30: Battle Win Conditions (254 tests passing, 0 warnings)
- [x] BattleState resource (status, majority_hold_timer[8], winner, victory_reason, player_count)
- [x] BattleStatus enum (Deployment, Active, Finished)
- [x] VictoryReason enum (AllCapturePoints, MajorityHold, TotalElimination)
- [x] battle_victory_system(): checks 3 win conditions each tick
- [x] EventType::BattleEnd=7
- [x] Minimum 200-tick (10s) wait before win conditions activate
- [x] 9 tests (no winner initially, no early win, all-points win, majority timer, majority 60s win, timer reset, elimination, BattleEnd event, no double win)

## Chunk 31: Deployment Phase (265 tests passing, 0 warnings)
- [x] DeploymentZone struct with contains() check, DeploymentState resource (zones, confirmed, countdown, command_posts)
- [x] deployment_zones() for 2P (opposite corners), 3P (triangle), 4P (all corners)
- [x] validate_placement() checks position within player's deployment zone
- [x] deploy_force() spawns 16 entities: CP + Forge + 10 Thralls + 3 Sentinels + 1 Hover Tank
- [x] spawn_entity() helper with full component setup (Position, Health, PathState, CombatState, etc.)
- [x] Deploy/ConfirmDeployment added to Command enum
- [x] command_processor handles Deploy, ConfirmDeployment
- [x] Cannot deploy twice (second deploy ignored)
- [x] Cannot confirm without deploying first
- [x] Protocol serialization roundtrip for new command variants
- [x] 11 tests

## Chunk 32: Campaign Map Layer - Data Model (324 tests passing, 0 warnings)
- [x] campaign/ module (mod.rs, map.rs, economy.rs, dispatch.rs, garrison.rs, research.rs, bridge.rs)
- [x] SiteType enum (Forge, MiningStation, RelicSite), GarrisonedUnit, CampaignSite
- [x] CampaignMap with generate(player_count, seed)
- [x] 14 tests

## Chunk 33: Campaign-RTS Bridge
- [x] BattleRequest, BattleResult structs
- [x] create_battle_from_campaign(), spawn_force(), extract_battle_result(), apply_battle_result()
- [x] 7 tests

## Chunk 34: Campaign Economy - Multi-Source Income
- [x] CampaignEconomy (forge 5e/s, mines 8e/s, relics 3e/s, starting bank 500)
- [x] 9 tests

## Chunk 35: Force Dispatch + Garrison System
- [x] DispatchOrder, DispatchQueue with dispatch_force(), tick(), process_arrival()
- [x] 14 tests

## Chunk 36: Reinforcement + Withdrawal
- [x] Included within bridge and dispatch modules

## Chunk 37: Research + Tech System
- [x] TechId enum (12 techs across 3 tiers), TechDefinition, PlayerResearch, ResearchJob
- [x] 14 tests

## Chunk 38: Forge Upgrades + Starting Conditions (338 tests passing, 0 warnings)
- [x] UpgradeForge command added
- [x] CampaignResearch, CampaignDispatch, CampaignWithdraw commands added

## Chunk 39: Campaign AI
- [x] CampaignAiState, CampaignAiDifficulty, CampaignGame struct
- [x] 14 tests

## Chunk 40: Client WebSocket Networking + State Sync (345 tests passing, 0 warnings)
- [x] GameSocket, NetBridge, mode detection

## Chunk 41: Lobby HTTP API (345 tests passing, 0 warnings)
- [x] Axum HTTP API routes, LobbyScreen Preact component

## Chunk 42: Client-Side Prediction + Replay (358 tests passing, 0 warnings)
- [x] ReplayRecorder, ReplayData, ReplayPlayer

## Chunk 43: Audio (Howler.js) + PWA (358 tests passing, 0 warnings)
- [x] SoundManager, PWA manifest, service worker

## Chunk 44: Performance Profiling + Docker (363 tests passing, 0 warnings)
- [x] TickProfile, LRU path cache, Dockerfile, CI

## Chunk 45: Integration Testing + Campaign Playthrough (378 tests passing, 0 warnings)
- [x] campaign_integration.rs, replay_integration.rs

---

# Phase 5: Campaign Client (Playable Game)

## Chunk 46: Campaign WASM Bridge
- [x] All campaign WASM exports

## Chunk 47: Campaign Client Bridge
- [x] CampaignTypes.ts, CampaignBridge.ts

## Chunk 48: Campaign Map Renderer
- [x] CampaignRenderer.ts, CampaignSiteSprite.ts, DispatchLineRenderer.ts

## Chunk 49: Campaign UI (Preact)
- [x] CampaignHUD.ts, CampaignResourceBar.ts, SitePanel.ts, DispatchDialog.ts, ResearchPanel.ts, ProductionPanel.ts, CampaignAlerts.ts

## Chunk 50: Campaign Input + Game Flow
- [x] CampaignInputManager.ts, GameFlowController.ts, CampaignBattleAdapter.ts

## Chunk 51: Minimap + Polish + Victory
- [x] MinimapRenderer.ts, VictoryScreen.ts, MainMenu.ts

## Phase 5 Complete
- [x] All 378 Rust tests pass, WASM/TS/Vite builds clean

---

## Bug Fixes (Post Phase 5)

### Fix: Battle indicator not showing for first battle
- [x] Changed next_battle_id: 0 -> 1

### Fix: DispatchDialog travel time estimate wrong
- [x] Changed distance / 2 -> distance / 5

### Fix: Browser context menu interfering with right-click dispatch
- [x] Added contextmenu event prevention

### Enhancement: Better dispatch feedback messages
- [x] Dispatch alerts now indicate target type

---

## Chunk 52: Campaign GUI Complete Redesign

### Problem
- Panels with position:fixed bottom:8px not visible in browser
- No battle notification system (battles happen silently)
- Campaign map microscopic (CAMPAIGN_MAP_SCALE=10, 1000x1000px map)
- Main menu offset to the left
- Research panel only accessible via hidden R key shortcut
- Production panel invisible separate panel

### Solution: Full GUI rewrite with unified HUD layout

- [x] index.html: Add #canvas-area div, proper layout structure
- [x] styles.ts: Complete CAMPAIGN_STYLES rewrite for grid layout + battle notification
- [x] config.ts: CAMPAIGN_MAP_SCALE 10 -> 30 (3000x3000px map)
- [x] CampaignResourceBar.ts: Rewrite for top bar zone + Research/Production buttons
- [x] SitePanel.ts: Rewrite with production integrated (merge ProductionPanel)
- [x] CampaignAlerts.ts: Rewrite as scrollable feed in right panel
- [x] CampaignHUD.ts: Full rewrite with CSS grid layout + battle notification overlay
- [x] GameFlowController.ts: Canvas in #canvas-area, battle auto-pause, notify HUD
- [x] MainMenu.ts: Fix centering (min-width -> width + max-width)
- [x] CampaignSiteSprite.ts: Double all icon sizes + label fonts
- [x] CampaignRenderer.ts: Better grid, connection lines, larger hit radius
- [x] MinimapRenderer.ts: Reposition to bottom-right of center area
- [x] Build + verify all panels visible and functional (tsc + vite build clean)

### Post-Redesign Bug Fixes
- [x] Fix: Main menu not clickable (re-add pointer-events:auto to #hud-root children)
- [x] Fix: Canvas sizing conflict (remove width/height 100% from #canvas-area canvas CSS)
- [x] Fix: Map too large (CAMPAIGN_MAP_SCALE 30 -> 20), MIN_ZOOM 0.5 -> 0.25
- [x] Fix: Left panel blocking 280px of clicks (conditional render only when site selected)
- [x] Fix: Auto-fit zoom on campaign map init
- [x] Fix: Battle view frozen (unpause campaign when entering battle from notification)
- [x] Fix: SitePanel "View Battle" also unpauses campaign before entering battle
- [x] Fix: CommandPost, HoverTank, Forge invisible in battle view - atlas frame naming mismatch
  - Atlas frames named `{unit}_{dir}` (e.g. `command_post_S`, `hover_tank_N`)
  - SpritePool generated `{unit}_{anim}_{dir}_{frame}` (e.g. `command_post_Idle_S_0000`)
  - Added static fallback lookup `{unit}_{dir}` in SpritePool.getFrameTexture()

## Chunk 53: RTS Battle View Bug Fixes

### Bug 1: Map off-center / can't zoom in RTS battle view
- [x] CampaignRenderer: add disableCamera()/enableCamera() methods
- [x] GameFlowController: call disableCamera on enterBattle, enableCamera on returnToCampaign
- [x] CameraController: rewrite centerOnMap() with proper isometric bounds + auto-fit zoom

### Bug 2: Multi-selected troops merge into 2 sprites (spacing)
- [x] campaign/bridge.rs spawn_force(): spacing 0.8 -> 1.5, offset 2.0 -> 3.0
- [x] deployment.rs deploy_force(): match spacing for consistency

### Bug 3: AI did not move in campaign battle
- [x] campaign/bridge.rs: add is_ai param to spawn_force, add AiControlled components
- [x] campaign/bridge.rs: create_battle_from_campaign() register AI players via add_ai_player()
- [x] Add test: test_battle_ai_initialized

### Bug 4: Both players get Forge in battle (should be CommandPost only)
- [x] campaign/bridge.rs spawn_force(): remove Forge spawn line
- [x] Update test_battle_spawns_units assertions (16->15, 12->11)

### Verification
- [x] cargo test --all passes (all tests pass including new test_battle_ai_initialized)
- [x] pnpm build compiles clean

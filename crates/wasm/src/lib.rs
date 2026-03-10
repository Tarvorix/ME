use wasm_bindgen::prelude::*;
use machine_empire_core::game::{Game, GameConfig, EventBuffer, EventCount};
use machine_empire_core::systems::fog::FogGrid;
use machine_empire_core::systems::resource::UIStateBuffer;
use machine_empire_core::command::Command;
use machine_empire_core::campaign_game::CampaignGame;
use machine_empire_core::campaign::map::{SiteType, GarrisonedUnit};
use machine_empire_core::campaign::production::queue_campaign_production;
use machine_empire_core::campaign::research::TechId;
use machine_empire_core::ai::campaign_ai::CampaignAiDifficulty;
use machine_empire_core::systems::battle_victory::BattleState;
use std::cell::UnsafeCell;

/// Single-threaded global game state. Safe because WASM is single-threaded.
struct GameCell(UnsafeCell<Option<Game>>);
unsafe impl Sync for GameCell {}

static GAME: GameCell = GameCell(UnsafeCell::new(None));

fn game() -> &'static Game {
    unsafe { (*GAME.0.get()).as_ref().unwrap_unchecked() }
}

fn game_mut() -> &'static mut Game {
    unsafe { (*GAME.0.get()).as_mut().unwrap_unchecked() }
}

#[wasm_bindgen]
pub fn init_game(map_width: u32, map_height: u32, player_count: u32, seed: u32) {
    let config = GameConfig {
        map_width,
        map_height,
        player_count,
        seed,
    };

    let mut game = Game::new(config);

    // Spawn 5 test Thralls near the center of the map
    let cx = map_width / 2;
    let cy = map_height / 2;
    for i in 0..5u32 {
        let x = cx as f32 - 2.0 + i as f32 + 0.5;
        let y = cy as f32 + 0.5;
        game.spawn_thrall(x, y, 0);
    }

    unsafe { *GAME.0.get() = Some(game); }
}

#[wasm_bindgen]
pub fn tick(delta_ms: f32) {
    game_mut().tick(delta_ms);
}

#[wasm_bindgen]
pub fn get_render_buffer_ptr() -> *const u8 {
    game().render_buffer_ptr()
}

#[wasm_bindgen]
pub fn get_render_count() -> u32 {
    game().render_count()
}

#[wasm_bindgen]
pub fn get_event_buffer_ptr() -> *const u8 {
    let g = game();
    g.world.get_resource::<EventBuffer>()
        .map(|eb| eb.0.as_ptr())
        .unwrap_or(std::ptr::null())
}

#[wasm_bindgen]
pub fn get_event_count() -> u32 {
    let g = game();
    g.world.get_resource::<EventCount>()
        .map(|ec| ec.0)
        .unwrap_or(0)
}

#[wasm_bindgen]
pub fn get_map_width() -> u32 {
    game().map().width
}

#[wasm_bindgen]
pub fn get_map_height() -> u32 {
    game().map().height
}

/// Returns packed tile data: terrain_type | (sprite_variant << 8)
#[wasm_bindgen]
pub fn get_map_tile(tile_x: u32, tile_y: u32) -> u32 {
    let map = game().map();
    if tile_x >= map.width || tile_y >= map.height {
        return 0;
    }
    let tile = map.get(tile_x, tile_y);
    (tile.terrain as u32) | ((tile.sprite_variant as u32) << 8)
}

/// Move a single unit to a target position.
#[wasm_bindgen]
pub fn cmd_move_unit(entity_id: u32, target_x: f32, target_y: f32) {
    game_mut().push_command(Command::Move {
        unit_ids: vec![entity_id],
        target_x,
        target_y,
    });
}

/// Attack a target entity.
#[wasm_bindgen]
pub fn cmd_attack(entity_id: u32, target_id: u32) {
    game_mut().push_command(Command::Attack {
        unit_ids: vec![entity_id],
        target_id,
    });
}

/// Attack-move: move to position, engaging enemies along the way.
#[wasm_bindgen]
pub fn cmd_attack_move(entity_id: u32, target_x: f32, target_y: f32) {
    game_mut().push_command(Command::AttackMove {
        unit_ids: vec![entity_id],
        target_x,
        target_y,
    });
}

/// Move multiple units to a target position (batch command).
#[wasm_bindgen]
pub fn cmd_move_units(ids_ptr: *const u32, count: u32, target_x: f32, target_y: f32) {
    let ids = unsafe {
        std::slice::from_raw_parts(ids_ptr, count as usize)
    };
    game_mut().push_command(Command::Move {
        unit_ids: ids.to_vec(),
        target_x,
        target_y,
    });
}

/// Attack a target with multiple units (batch command).
#[wasm_bindgen]
pub fn cmd_attack_target(ids_ptr: *const u32, count: u32, target_id: u32) {
    let ids = unsafe {
        std::slice::from_raw_parts(ids_ptr, count as usize)
    };
    game_mut().push_command(Command::Attack {
        unit_ids: ids.to_vec(),
        target_id,
    });
}

/// Stop units.
#[wasm_bindgen]
pub fn cmd_stop_unit(entity_id: u32) {
    game_mut().push_command(Command::Stop {
        unit_ids: vec![entity_id],
    });
}

/// Get pointer to a player's fog grid buffer for JS reading.
#[wasm_bindgen]
pub fn get_fog_buffer_ptr(player: u32) -> *const u8 {
    let g = game();
    g.world.get_resource::<FogGrid>()
        .map(|fg| fg.grid_ptr(player))
        .unwrap_or(std::ptr::null())
}

/// Get the byte length of one player's fog grid.
#[wasm_bindgen]
pub fn get_fog_buffer_len() -> u32 {
    let g = game();
    g.world.get_resource::<FogGrid>()
        .map(|fg| fg.grid_len())
        .unwrap_or(0)
}

/// Queue a unit for production.
#[wasm_bindgen]
pub fn cmd_produce(player: u8, unit_type: u16) {
    game_mut().push_command(Command::Produce {
        player,
        unit_type,
    });
}

/// Cancel production on a specific line.
#[wasm_bindgen]
pub fn cmd_cancel_production(player: u8, line_index: u8) {
    game_mut().push_command(Command::CancelProduction {
        player,
        line_index,
    });
}

/// Set rally point for newly produced units.
#[wasm_bindgen]
pub fn cmd_set_rally(player: u8, x: f32, y: f32) {
    game_mut().push_command(Command::SetRally {
        player,
        x,
        y,
    });
}

/// Get pointer to UIStateBuffer for JS reading.
#[wasm_bindgen]
pub fn get_ui_state_ptr() -> *const u8 {
    let g = game();
    g.world.get_resource::<UIStateBuffer>()
        .map(|ui| ui.0.as_ptr())
        .unwrap_or(std::ptr::null())
}

/// Get the byte length of UIStateBuffer (256 bytes).
#[wasm_bindgen]
pub fn get_ui_state_len() -> u32 {
    256
}

/// Greeting for smoke test.
#[wasm_bindgen]
pub fn greet() -> String {
    machine_empire_core::hello().to_string()
}

// ════════════════════════════════════════════════════════════════════════════
// Campaign WASM Bridge
// ════════════════════════════════════════════════════════════════════════════

/// Single-threaded global campaign state. Safe because WASM is single-threaded.
struct CampaignCell(UnsafeCell<Option<CampaignGame>>);
unsafe impl Sync for CampaignCell {}

static CAMPAIGN: CampaignCell = CampaignCell(UnsafeCell::new(None));

fn campaign() -> &'static CampaignGame {
    unsafe { (*CAMPAIGN.0.get()).as_ref().unwrap_unchecked() }
}

fn campaign_mut() -> &'static mut CampaignGame {
    unsafe { (*CAMPAIGN.0.get()).as_mut().unwrap_unchecked() }
}

// ── Campaign buffer sizes ────────────────────────────────────────────────
const MAX_SITES: usize = 32;
const SITE_ENTRY_BYTES: usize = 32;
const MAX_CAMPAIGN_PLAYERS: usize = 4;
const ECONOMY_ENTRY_BYTES: usize = 40;
const PRODUCTION_ENTRY_BYTES: usize = 28;
const RESEARCH_ENTRY_BYTES: usize = 22;
const MAX_DISPATCH_ORDERS: usize = 32;
const DISPATCH_ENTRY_BYTES: usize = 28;
const MAX_ACTIVE_BATTLES: usize = 8;
const BATTLE_ENTRY_BYTES: usize = 12;
const MAX_TECH_COUNT: usize = 12;

/// UnsafeCell wrapper for campaign byte buffers. Safe because WASM is single-threaded.
struct Buf<const N: usize>(UnsafeCell<[u8; N]>);
unsafe impl<const N: usize> Sync for Buf<N> {}

impl<const N: usize> Buf<N> {
    const fn new() -> Self {
        Buf(UnsafeCell::new([0u8; N]))
    }

    /// Get a mutable slice to the buffer contents.
    fn get_mut(&self) -> &mut [u8; N] {
        unsafe { &mut *self.0.get() }
    }

    /// Get a const pointer to the buffer for returning to JS.
    fn as_ptr(&self) -> *const u8 {
        self.0.get() as *const u8
    }
}

// Campaign buffers (UnsafeCell-wrapped, safe because WASM is single-threaded)
static SITE_BUF: Buf<{ MAX_SITES * SITE_ENTRY_BYTES }> = Buf::new();
static ECONOMY_BUF: Buf<ECONOMY_ENTRY_BYTES> = Buf::new();
static PRODUCTION_BUF: Buf<PRODUCTION_ENTRY_BYTES> = Buf::new();
static RESEARCH_BUF: Buf<RESEARCH_ENTRY_BYTES> = Buf::new();
static DISPATCH_BUF: Buf<{ MAX_DISPATCH_ORDERS * DISPATCH_ENTRY_BYTES }> = Buf::new();
static BATTLE_BUF: Buf<{ MAX_ACTIVE_BATTLES * BATTLE_ENTRY_BYTES }> = Buf::new();
static AVAIL_TECHS_BUF: Buf<MAX_TECH_COUNT> = Buf::new();
static ELIMINATED_BUF: Buf<MAX_CAMPAIGN_PLAYERS> = Buf::new();

// ── TechId conversion ────────────────────────────────────────────────────

/// All TechId variants in canonical order for u8 conversion.
const ALL_TECHS: [TechId; 12] = [
    TechId::ThrallPlating,           // 0
    TechId::SentinelHeavyWeapons,    // 1
    TechId::HoverTankReactiveArmor,  // 2
    TechId::ImprovedVision,          // 3
    TechId::ThrallFireRate,          // 4
    TechId::SentinelShields,         // 5
    TechId::HoverTankSiege,          // 6
    TechId::FastProduction,          // 7
    TechId::ThrallRange,             // 8
    TechId::SentinelStealth,         // 9
    TechId::HoverTankOvercharge,     // 10
    TechId::EconomicEfficiency,      // 11
];

fn tech_to_u8(tech: TechId) -> u8 {
    ALL_TECHS.iter().position(|t| *t == tech).unwrap_or(255) as u8
}

fn u8_to_tech(id: u8) -> Option<TechId> {
    ALL_TECHS.get(id as usize).copied()
}

fn site_type_to_u8(st: SiteType) -> u8 {
    match st {
        SiteType::Node => 0,
        SiteType::MiningStation => 1,
        SiteType::RelicSite => 2,
    }
}

fn difficulty_from_u32(d: u32) -> CampaignAiDifficulty {
    match d {
        0 => CampaignAiDifficulty::Easy,
        1 => CampaignAiDifficulty::Normal,
        _ => CampaignAiDifficulty::Hard,
    }
}

fn battle_status_to_u8(s: machine_empire_core::systems::battle_victory::BattleStatus) -> u8 {
    match s {
        machine_empire_core::systems::battle_victory::BattleStatus::Deployment => 0,
        machine_empire_core::systems::battle_victory::BattleStatus::Active => 1,
        machine_empire_core::systems::battle_victory::BattleStatus::Finished => 2,
    }
}

/// Write a little-endian u32 into a buffer at a given offset.
fn write_u32(buf: &mut [u8], offset: usize, val: u32) {
    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
}

/// Write a little-endian f32 into a buffer at a given offset.
fn write_f32(buf: &mut [u8], offset: usize, val: f32) {
    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
}

// ── Lifecycle ────────────────────────────────────────────────────────────

/// Initialize a new campaign game.
#[wasm_bindgen]
pub fn campaign_init(player_count: u32, seed: u32) {
    let cg = CampaignGame::new(player_count as u8, seed as u64);
    unsafe { *CAMPAIGN.0.get() = Some(cg); }
}

/// Run one campaign tick (economy, research, dispatch, battles, AI).
#[wasm_bindgen]
pub fn campaign_tick() {
    campaign_mut().tick();
}

/// Pause or unpause the campaign. 0 = unpause, nonzero = pause.
#[wasm_bindgen]
pub fn campaign_set_paused(paused: u32) {
    campaign_mut().paused = paused != 0;
}

/// Returns 1 if paused, 0 if running.
#[wasm_bindgen]
pub fn campaign_is_paused() -> u32 {
    if campaign().paused { 1 } else { 0 }
}

/// Get the current campaign tick count.
#[wasm_bindgen]
pub fn campaign_get_tick() -> u32 {
    campaign().tick_count
}

/// Add an AI player. difficulty: 0=Easy, 1=Normal, 2+=Hard.
#[wasm_bindgen]
pub fn campaign_add_ai(player_id: u32, difficulty: u32) {
    campaign_mut().add_ai_player(player_id as u8, difficulty_from_u32(difficulty));
}

/// Get the number of players in the campaign.
#[wasm_bindgen]
pub fn campaign_get_player_count() -> u32 {
    campaign().player_count as u32
}

// ── Map Queries ──────────────────────────────────────────────────────────

/// Get the number of sites on the campaign map.
#[wasm_bindgen]
pub fn campaign_get_site_count() -> u32 {
    campaign().campaign_map.sites.len() as u32
}

/// Write all site data into the site buffer and return pointer.
/// Buffer format per site (32 bytes):
///   [0..3]   site_id            : u32
///   [4]      site_type          : u8 (0=Node, 1=Mine, 2=Relic)
///   [5]      owner              : u8 (255=neutral)
///   [6]      is_contested       : u8
///   [7]      garrison_count     : u8 (total units, capped at 255)
///   [8..11]  x                  : f32
///   [12..15] y                  : f32
///   [16..19] garrison_thralls   : u32
///   [20..23] garrison_sentinels : u32
///   [24..27] garrison_tanks     : u32
///   [28..31] battle_id          : u32 (0 = no battle)
#[wasm_bindgen]
pub fn campaign_get_site_data_ptr() -> *const u8 {
    let cg = campaign();
    let count = cg.campaign_map.sites.len().min(MAX_SITES);
    let buf = SITE_BUF.get_mut();
    buf.fill(0);

    for i in 0..count {
        let site = &cg.campaign_map.sites[i];
        let base = i * SITE_ENTRY_BYTES;

        write_u32(buf, base, site.id);
        buf[base + 4] = site_type_to_u8(site.site_type);
        buf[base + 5] = site.owner;
        buf[base + 6] = if site.is_contested { 1 } else { 0 };
        buf[base + 7] = site.garrison_count().min(255) as u8;
        write_f32(buf, base + 8, site.x);
        write_f32(buf, base + 12, site.y);

        // Garrison breakdown by unit type
        let thralls: u32 = site.garrison.iter()
            .filter(|g| g.unit_type == 0)
            .map(|g| g.count)
            .sum();
        let sentinels: u32 = site.garrison.iter()
            .filter(|g| g.unit_type == 1)
            .map(|g| g.count)
            .sum();
        let tanks: u32 = site.garrison.iter()
            .filter(|g| g.unit_type == 2)
            .map(|g| g.count)
            .sum();

        write_u32(buf, base + 16, thralls);
        write_u32(buf, base + 20, sentinels);
        write_u32(buf, base + 24, tanks);
        write_u32(buf, base + 28, site.battle_id.unwrap_or(0));
    }

    SITE_BUF.as_ptr()
}

/// Get the byte length of the site data buffer.
#[wasm_bindgen]
pub fn campaign_get_site_data_len() -> u32 {
    let count = campaign().campaign_map.sites.len().min(MAX_SITES);
    (count * SITE_ENTRY_BYTES) as u32
}

/// Get the campaign map width.
#[wasm_bindgen]
pub fn campaign_get_map_width() -> f32 {
    campaign().campaign_map.width
}

/// Get the campaign map height.
#[wasm_bindgen]
pub fn campaign_get_map_height() -> f32 {
    campaign().campaign_map.height
}

/// Get the node site ID for a player. Returns u32::MAX if invalid.
#[wasm_bindgen]
pub fn campaign_get_player_node(player: u32) -> u32 {
    campaign().campaign_map.player_nodes
        .get(player as usize)
        .copied()
        .unwrap_or(u32::MAX)
}

// ── Economy Queries ──────────────────────────────────────────────────────

/// Write economy data for a player into the economy buffer and return pointer.
/// Buffer format (40 bytes):
///   [0..3]   energy_bank     : f32
///   [4..7]   node_income     : f32
///   [8..11]  mine_income     : f32
///   [12..15] relic_income    : f32
///   [16..19] total_income    : f32
///   [20..23] total_expenses  : f32
///   [24..27] net_rate        : f32
///   [28..31] strain          : f32
///   [32..35] garrison_upkeep : f32
///   [36..39] deployed_upkeep : f32
#[wasm_bindgen]
pub fn campaign_get_economy_ptr(player: u32) -> *const u8 {
    let cg = campaign();
    let pid = player as usize;
    let buf = ECONOMY_BUF.get_mut();
    buf.fill(0);

    if let Some(econ) = cg.economies.get(pid) {
        write_f32(buf, 0, econ.energy_bank);
        write_f32(buf, 4, econ.node_income);
        write_f32(buf, 8, econ.mine_income);
        write_f32(buf, 12, econ.relic_income);
        write_f32(buf, 16, econ.total_income());
        write_f32(buf, 20, econ.total_expenses());
        write_f32(buf, 24, econ.net_rate());
        write_f32(buf, 28, econ.strain);
        write_f32(buf, 32, econ.garrison_upkeep);
        write_f32(buf, 36, econ.deployed_upkeep);
    }

    ECONOMY_BUF.as_ptr()
}

/// Get the byte length of the economy buffer (40 bytes).
#[wasm_bindgen]
pub fn campaign_get_economy_len() -> u32 {
    ECONOMY_ENTRY_BYTES as u32
}

// ── Production Queries ───────────────────────────────────────────────────

/// Write campaign production state for a player into the production buffer and return pointer.
/// Buffer format (28 bytes):
///   [0]      active_unit_type  : u8 (255 = idle)
///   [1..4]   active_progress   : f32
///   [5..8]   active_total_time : f32
///   [9..12]  queued_count      : u32
///   [13..16] queued_thralls    : u32
///   [17..20] queued_sentinels  : u32
///   [21..24] queued_tanks      : u32
///   [25..27] reserved
#[wasm_bindgen]
pub fn campaign_get_production_ptr(player: u32) -> *const u8 {
    let cg = campaign();
    let pid = player as usize;
    let buf = PRODUCTION_BUF.get_mut();
    buf.fill(0);
    buf[0] = 255;

    if let Some(queue) = cg.productions.0.get(pid) {
        if let Some(job) = &queue.active_job {
            buf[0] = job.unit_type as u8;
            write_f32(buf, 1, job.progress);
            write_f32(buf, 5, job.total_time);
        }

        let (queued_thralls, queued_sentinels, queued_tanks) = queue.queued_counts_by_type();
        write_u32(buf, 9, queue.queued_count());
        write_u32(buf, 13, queued_thralls);
        write_u32(buf, 17, queued_sentinels);
        write_u32(buf, 21, queued_tanks);
    }

    PRODUCTION_BUF.as_ptr()
}

/// Get the byte length of the campaign production buffer (28 bytes).
#[wasm_bindgen]
pub fn campaign_get_production_len() -> u32 {
    PRODUCTION_ENTRY_BYTES as u32
}

// ── Research Queries ─────────────────────────────────────────────────────

/// Write research state for a player into the research buffer and return pointer.
/// Buffer format (22 bytes):
///   [0]      active_tech_id    : u8 (255 = none)
///   [1..4]   active_progress   : f32
///   [5..8]   active_total_time : f32
///   [9]      completed_count   : u8
///   [10..21] completed_techs   : [u8; 12] (tech IDs, 255=empty)
#[wasm_bindgen]
pub fn campaign_get_research_ptr(player: u32) -> *const u8 {
    let cg = campaign();
    let pid = player as usize;
    let buf = RESEARCH_BUF.get_mut();
    // Fill with 255 (no-data sentinel)
    buf.fill(255);

    if let Some(pr) = cg.research.get(pid) {
        // Active research
        if let Some(job) = &pr.active_job {
            buf[0] = tech_to_u8(job.tech_id);
            write_f32(buf, 1, job.progress);
            write_f32(buf, 5, job.research_time);
        } else {
            buf[0] = 255;
            write_f32(buf, 1, 0.0);
            write_f32(buf, 5, 0.0);
        }

        // Completed techs
        let completed_count = pr.completed.len().min(12);
        buf[9] = completed_count as u8;
        for i in 0..12 {
            if i < completed_count {
                buf[10 + i] = tech_to_u8(pr.completed[i]);
            } else {
                buf[10 + i] = 255;
            }
        }
    }

    RESEARCH_BUF.as_ptr()
}

/// Get the byte length of the research buffer (22 bytes).
#[wasm_bindgen]
pub fn campaign_get_research_len() -> u32 {
    RESEARCH_ENTRY_BYTES as u32
}

/// Write available (researchable) tech IDs for a player into the buffer.
/// Each byte is a tech ID (0-11) that the player can currently research.
#[wasm_bindgen]
pub fn campaign_get_available_techs_ptr(player: u32) -> *const u8 {
    let cg = campaign();
    let pid = player as usize;
    let buf = AVAIL_TECHS_BUF.get_mut();
    buf.fill(255);

    if let Some(pr) = cg.research.get(pid) {
        let owned_relics = cg.campaign_map.count_relics(pid as u8);
        let energy = cg.economies.get(pid).map(|e| e.energy_bank).unwrap_or(0.0);

        let mut idx = 0;
        for &tech in &ALL_TECHS {
            if pr.can_research(tech, owned_relics, energy) {
                buf[idx] = tech_to_u8(tech);
                idx += 1;
                if idx >= MAX_TECH_COUNT { break; }
            }
        }
    }

    AVAIL_TECHS_BUF.as_ptr()
}

/// Get the number of technologies currently available to research for a player.
#[wasm_bindgen]
pub fn campaign_get_available_techs_count(player: u32) -> u32 {
    let cg = campaign();
    let pid = player as usize;

    if let Some(pr) = cg.research.get(pid) {
        let owned_relics = cg.campaign_map.count_relics(pid as u8);
        let energy = cg.economies.get(pid).map(|e| e.energy_bank).unwrap_or(0.0);

        let mut count = 0u32;
        for &tech in &ALL_TECHS {
            if pr.can_research(tech, owned_relics, energy) {
                count += 1;
            }
        }
        count
    } else {
        0
    }
}

// ── Dispatch Queries ─────────────────────────────────────────────────────

/// Write active dispatch orders into the dispatch buffer and return pointer.
/// Buffer format per order (28 bytes):
///   [0..3]   order_id          : u32
///   [4]      player            : u8
///   [5..7]   _padding          : [u8; 3]
///   [8..11]  source_site       : u32
///   [12..15] target_site       : u32
///   [16..19] travel_remaining  : f32
///   [20..23] total_time        : f32
///   [24..27] unit_count        : u32
#[wasm_bindgen]
pub fn campaign_get_dispatch_orders_ptr() -> *const u8 {
    let cg = campaign();
    let orders = &cg.dispatch_queue.orders;
    let count = orders.len().min(MAX_DISPATCH_ORDERS);
    let buf = DISPATCH_BUF.get_mut();
    buf.fill(0);

    for i in 0..count {
        let order = &orders[i];
        let base = i * DISPATCH_ENTRY_BYTES;

        write_u32(buf, base, order.order_id);
        buf[base + 4] = order.player;
        buf[base + 5] = 0; // padding
        buf[base + 6] = 0; // padding
        buf[base + 7] = 0; // padding
        write_u32(buf, base + 8, order.source_site);
        write_u32(buf, base + 12, order.target_site);
        write_f32(buf, base + 16, order.travel_remaining);
        write_f32(buf, base + 20, order.total_time);

        let unit_count: u32 = order.units.iter().map(|u| u.count).sum();
        write_u32(buf, base + 24, unit_count);
    }

    DISPATCH_BUF.as_ptr()
}

/// Get the number of active dispatch orders.
#[wasm_bindgen]
pub fn campaign_get_dispatch_orders_count() -> u32 {
    campaign().dispatch_queue.orders.len().min(MAX_DISPATCH_ORDERS) as u32
}

// ── Battle Queries ───────────────────────────────────────────────────────

/// Get the number of active battles.
#[wasm_bindgen]
pub fn campaign_get_active_battle_count() -> u32 {
    campaign().active_battles.len().min(MAX_ACTIVE_BATTLES) as u32
}

/// Write active battle data into the battle buffer and return pointer.
/// Buffer format per battle (12 bytes):
///   [0..3]   site_id    : u32
///   [4]      attacker   : u8
///   [5]      defender   : u8
///   [6]      status     : u8 (0=deployment, 1=active, 2=finished)
///   [7]      winner     : u8 (255=none)
///   [8..11]  tick_count : u32
#[wasm_bindgen]
pub fn campaign_get_active_battles_ptr() -> *const u8 {
    let cg = campaign();
    let battles = &cg.active_battles;
    let count = battles.len().min(MAX_ACTIVE_BATTLES);
    let buf = BATTLE_BUF.get_mut();
    buf.fill(0);

    for i in 0..count {
        let battle = &battles[i];
        let base = i * BATTLE_ENTRY_BYTES;

        write_u32(buf, base, battle.site_id);
        buf[base + 4] = battle.attacker;
        buf[base + 5] = battle.defender;

        // Get battle status from the Game's BattleState resource
        let (status, winner, ticks) = if let Some(bs) = battle.game.world.get_resource::<BattleState>() {
            (battle_status_to_u8(bs.status), bs.winner, battle.game.tick_count)
        } else {
            (1, 255, battle.game.tick_count)
        };

        buf[base + 6] = status;
        buf[base + 7] = winner;
        write_u32(buf, base + 8, ticks);
    }

    BATTLE_BUF.as_ptr()
}

/// Get the render buffer pointer for a campaign battle's RTS Game.
/// Allows the client to render the battle using existing render infrastructure.
#[wasm_bindgen]
pub fn campaign_get_battle_render_ptr(battle_index: u32) -> *const u8 {
    let cg = campaign();
    if let Some(battle) = cg.active_battles.get(battle_index as usize) {
        battle.game.render_buffer_ptr()
    } else {
        std::ptr::null()
    }
}

/// Get the render entity count for a campaign battle.
#[wasm_bindgen]
pub fn campaign_get_battle_render_count(battle_index: u32) -> u32 {
    let cg = campaign();
    if let Some(battle) = cg.active_battles.get(battle_index as usize) {
        battle.game.render_count()
    } else {
        0
    }
}

/// Get the event buffer pointer for a campaign battle.
#[wasm_bindgen]
pub fn campaign_get_battle_event_ptr(battle_index: u32) -> *const u8 {
    let cg = campaign();
    if let Some(battle) = cg.active_battles.get(battle_index as usize) {
        battle.game.world.get_resource::<EventBuffer>()
            .map(|eb| eb.0.as_ptr())
            .unwrap_or(std::ptr::null())
    } else {
        std::ptr::null()
    }
}

/// Get the event count for a campaign battle.
#[wasm_bindgen]
pub fn campaign_get_battle_event_count(battle_index: u32) -> u32 {
    let cg = campaign();
    if let Some(battle) = cg.active_battles.get(battle_index as usize) {
        battle.game.world.get_resource::<EventCount>()
            .map(|ec| ec.0)
            .unwrap_or(0)
    } else {
        0
    }
}

/// Get the fog buffer pointer for a specific player in a campaign battle.
#[wasm_bindgen]
pub fn campaign_get_battle_fog_ptr(battle_index: u32, player: u32) -> *const u8 {
    let cg = campaign();
    if let Some(battle) = cg.active_battles.get(battle_index as usize) {
        battle.game.world.get_resource::<FogGrid>()
            .map(|fg| fg.grid_ptr(player))
            .unwrap_or(std::ptr::null())
    } else {
        std::ptr::null()
    }
}

/// Get the fog buffer byte length for a campaign battle.
#[wasm_bindgen]
pub fn campaign_get_battle_fog_len(battle_index: u32) -> u32 {
    let cg = campaign();
    if let Some(battle) = cg.active_battles.get(battle_index as usize) {
        battle.game.world.get_resource::<FogGrid>()
            .map(|fg| fg.grid_len())
            .unwrap_or(0)
    } else {
        0
    }
}

/// Get a campaign battle's map width.
#[wasm_bindgen]
pub fn campaign_get_battle_map_width(battle_index: u32) -> u32 {
    let cg = campaign();
    if let Some(battle) = cg.active_battles.get(battle_index as usize) {
        battle.game.map().width
    } else {
        0
    }
}

/// Get a campaign battle's map height.
#[wasm_bindgen]
pub fn campaign_get_battle_map_height(battle_index: u32) -> u32 {
    let cg = campaign();
    if let Some(battle) = cg.active_battles.get(battle_index as usize) {
        battle.game.map().height
    } else {
        0
    }
}

/// Get a map tile from a campaign battle. Returns packed terrain_type | (sprite_variant << 8).
#[wasm_bindgen]
pub fn campaign_get_battle_map_tile(battle_index: u32, tile_x: u32, tile_y: u32) -> u32 {
    let cg = campaign();
    if let Some(battle) = cg.active_battles.get(battle_index as usize) {
        let map = battle.game.map();
        if tile_x >= map.width || tile_y >= map.height {
            return 0;
        }
        let tile = map.get(tile_x, tile_y);
        (tile.terrain as u32) | ((tile.sprite_variant as u32) << 8)
    } else {
        0
    }
}

/// Get the UI state buffer pointer for a campaign battle.
#[wasm_bindgen]
pub fn campaign_get_battle_ui_state_ptr(battle_index: u32) -> *const u8 {
    let cg = campaign();
    if let Some(battle) = cg.active_battles.get(battle_index as usize) {
        battle.game.world.get_resource::<UIStateBuffer>()
            .map(|ui| ui.0.as_ptr())
            .unwrap_or(std::ptr::null())
    } else {
        std::ptr::null()
    }
}

/// Get eliminated player IDs as a buffer.
/// Each byte is a player ID (255 = unused slot).
#[wasm_bindgen]
pub fn campaign_get_eliminated_players_ptr() -> *const u8 {
    let cg = campaign();
    let eliminated = cg.eliminated_players();
    let buf = ELIMINATED_BUF.get_mut();
    buf.fill(255);

    for (i, &pid) in eliminated.iter().enumerate() {
        if i >= MAX_CAMPAIGN_PLAYERS { break; }
        buf[i] = pid;
    }

    ELIMINATED_BUF.as_ptr()
}

/// Get the number of eliminated players.
#[wasm_bindgen]
pub fn campaign_get_eliminated_count() -> u32 {
    campaign().eliminated_players().len() as u32
}

// ── Campaign Commands ────────────────────────────────────────────────────

/// Dispatch units from source site to target site.
/// unit_pairs is a flat array of u32 pairs: [unit_type, count, unit_type, count, ...]
/// Returns the order ID on success, or u32::MAX on failure.
#[wasm_bindgen]
pub fn campaign_cmd_dispatch(
    player: u32,
    source_site: u32,
    target_site: u32,
    unit_pairs: &[u32],
) -> u32 {
    let mut units = Vec::new();
    for chunk in unit_pairs.chunks(2) {
        if chunk.len() == 2 && chunk[1] > 0 {
            units.push(GarrisonedUnit::new(chunk[0] as u16, chunk[1]));
        }
    }

    if units.is_empty() {
        return u32::MAX;
    }

    let cg = campaign_mut();
    match cg.dispatch_queue.dispatch_force(
        &mut cg.campaign_map,
        player as u8,
        source_site,
        target_site,
        units,
    ) {
        Some(order_id) => order_id,
        None => u32::MAX,
    }
}

/// Start researching a technology.
/// tech_id: 0-11 matching the TechId enum order.
/// Returns 1 on success, 0 on failure.
#[wasm_bindgen]
pub fn campaign_cmd_research(player: u32, tech_id: u32) -> u32 {
    let tech = match u8_to_tech(tech_id as u8) {
        Some(t) => t,
        None => return 0,
    };

    let cg = campaign_mut();
    let pid = player as usize;

    if pid >= cg.research.len() || pid >= cg.economies.len() {
        return 0;
    }

    let owned_relics = cg.campaign_map.count_relics(player as u8);
    let energy = cg.economies[pid].energy_bank;

    if cg.research[pid].can_research(tech, owned_relics, energy) {
        let cost = cg.research[pid].start_research(tech);
        cg.economies[pid].energy_bank -= cost;
        1
    } else {
        0
    }
}

/// Produce units at the player's node.
/// Deducts energy immediately and queues units one-at-a-time at the node.
/// unit_type: 0=Thrall, 1=Sentinel, 2=HoverTank.
/// Returns 1 on success, 0 on failure.
#[wasm_bindgen]
pub fn campaign_cmd_produce(player: u32, unit_type: u32, count: u32) -> u32 {
    let cg = campaign_mut();
    let node_exists = cg.campaign_map.get_node(player as u8).is_some();
    if queue_campaign_production(
        &mut cg.economies,
        &mut cg.productions,
        player as u8,
        unit_type as u16,
        count,
        node_exists,
    ) { 1 } else { 0 }
}

/// Withdraw all garrison from a site, dispatching them back to the player's node.
/// Returns 1 on success, 0 on failure.
#[wasm_bindgen]
pub fn campaign_cmd_withdraw(player: u32, site_id: u32) -> u32 {
    let cg = campaign_mut();
    let pid = player as u8;

    // Validate site
    let (is_valid, units) = if let Some(site) = cg.campaign_map.get_site(site_id) {
        if site.owner != pid || site.is_contested || site.garrison.is_empty() {
            (false, Vec::new())
        } else {
            (true, site.garrison.clone())
        }
    } else {
        (false, Vec::new())
    };

    if !is_valid {
        return 0;
    }

    // Get node ID
    let node_id = match cg.campaign_map.player_nodes.get(pid as usize) {
        Some(&id) => id,
        None => return 0,
    };

    // Can't withdraw from node itself
    if site_id == node_id {
        return 0;
    }

    // dispatch_force validates ownership, removes from source garrison, creates order
    match cg.dispatch_queue.dispatch_force(
        &mut cg.campaign_map,
        pid,
        site_id,
        node_id,
        units,
    ) {
        Some(_) => 1,
        None => 0,
    }
}

// ── Battle Commands (for controlling units during campaign battles) ──────

/// Move a unit in a campaign battle.
#[wasm_bindgen]
pub fn campaign_battle_cmd_move(battle_index: u32, entity_id: u32, target_x: f32, target_y: f32) {
    let cg = campaign_mut();
    if let Some(battle) = cg.active_battles.get_mut(battle_index as usize) {
        battle.game.push_command(Command::Move {
            unit_ids: vec![entity_id],
            target_x,
            target_y,
        });
    }
}

/// Attack a target in a campaign battle.
#[wasm_bindgen]
pub fn campaign_battle_cmd_attack(battle_index: u32, entity_id: u32, target_id: u32) {
    let cg = campaign_mut();
    if let Some(battle) = cg.active_battles.get_mut(battle_index as usize) {
        battle.game.push_command(Command::Attack {
            unit_ids: vec![entity_id],
            target_id,
        });
    }
}

/// Attack-move a unit in a campaign battle.
#[wasm_bindgen]
pub fn campaign_battle_cmd_attack_move(battle_index: u32, entity_id: u32, target_x: f32, target_y: f32) {
    let cg = campaign_mut();
    if let Some(battle) = cg.active_battles.get_mut(battle_index as usize) {
        battle.game.push_command(Command::AttackMove {
            unit_ids: vec![entity_id],
            target_x,
            target_y,
        });
    }
}

/// Move multiple units in a campaign battle.
#[wasm_bindgen]
pub fn campaign_battle_cmd_move_units(battle_index: u32, unit_ids: &[u32], target_x: f32, target_y: f32) {
    let cg = campaign_mut();
    if let Some(battle) = cg.active_battles.get_mut(battle_index as usize) {
        battle.game.push_command(Command::Move {
            unit_ids: unit_ids.to_vec(),
            target_x,
            target_y,
        });
    }
}

/// Attack with multiple units in a campaign battle.
#[wasm_bindgen]
pub fn campaign_battle_cmd_attack_target(battle_index: u32, unit_ids: &[u32], target_id: u32) {
    let cg = campaign_mut();
    if let Some(battle) = cg.active_battles.get_mut(battle_index as usize) {
        battle.game.push_command(Command::Attack {
            unit_ids: unit_ids.to_vec(),
            target_id,
        });
    }
}

/// Stop a unit in a campaign battle.
#[wasm_bindgen]
pub fn campaign_battle_cmd_stop(battle_index: u32, entity_id: u32) {
    let cg = campaign_mut();
    if let Some(battle) = cg.active_battles.get_mut(battle_index as usize) {
        battle.game.push_command(Command::Stop {
            unit_ids: vec![entity_id],
        });
    }
}

/// Request reinforcements for a player in a campaign battle.
/// Units are drawn from the player's Node garrison.
/// Returns 1 on success, 0 on failure.
#[wasm_bindgen]
pub fn campaign_battle_cmd_reinforce(battle_index: u32, player: u8, unit_type: u16, count: u32) -> u32 {
    let cg = campaign_mut();
    if cg.request_reinforcement(battle_index as usize, player, unit_type, count) {
        1
    } else {
        0
    }
}

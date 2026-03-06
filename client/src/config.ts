/** Set to true to enable verbose console logging for debugging. */
export const DEBUG = false;

export const TILE_WIDTH = 64;
export const TILE_HEIGHT = 32;
export const MAP_WIDTH = 64;
export const MAP_HEIGHT = 64;
export const MAX_ENTITIES = 2048;
export const RENDER_ENTRY_SIZE = 32;
export const EVENT_ENTRY_SIZE = 32;
export const MAX_EVENTS = 256;
export const SIM_TICK_RATE = 20;
export const SIM_TICK_MS = 50;

// Sprite scale: atlas frames are 512x512, thralls should be ~48px on screen
export const THRALL_SCALE = 48 / 512;
export const SENTINEL_SCALE = 56 / 512;
export const HOVER_TANK_SCALE = 72 / 512;
export const COMMAND_POST_SCALE = 96 / 512;
export const NODE_SCALE = 96 / 512;

// ── Campaign Map Constants ──────────────────────────────────────────────
/** Pixels per campaign map unit. Campaign map is 100x100 units → 2000x2000 px. */
export const CAMPAIGN_MAP_SCALE = 20;
/** Campaign simulation tick interval in ms (matches Rust 0.05s delta). */
export const CAMPAIGN_TICK_MS = 50;

// ── Player Colors (shared between campaign and RTS) ─────────────────────
export const PLAYER_COLORS: readonly number[] = [
    0x4488FF, // Player 0: Blue
    0xFF4444, // Player 1: Red
    0x44CC44, // Player 2: Green
    0xFFCC44, // Player 3: Yellow
];
export const NEUTRAL_COLOR = 0x666666;

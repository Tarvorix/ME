import { TILE_WIDTH, TILE_HEIGHT } from '../config';

/**
 * Convert tile-space coordinates to screen-space (pixel) coordinates.
 * Tile (0,0) maps to screen (0,0). +X goes right-down, +Y goes left-down.
 */
export function tileToScreen(tileX: number, tileY: number): { sx: number; sy: number } {
    const sx = (tileX - tileY) * (TILE_WIDTH / 2);
    const sy = (tileX + tileY) * (TILE_HEIGHT / 2);
    return { sx, sy };
}

/**
 * Convert screen-space coordinates back to tile-space (floating point).
 */
export function screenToTile(sx: number, sy: number): { tx: number; ty: number } {
    const tx = (sx / (TILE_WIDTH / 2) + sy / (TILE_HEIGHT / 2)) / 2;
    const ty = (sy / (TILE_HEIGHT / 2) - sx / (TILE_WIDTH / 2)) / 2;
    return { tx, ty };
}

import { TILE_WIDTH } from '../config';

export const BATTLE_TILE_SIZE = TILE_WIDTH;

export interface BattleWorldBounds {
    minX: number;
    maxX: number;
    minY: number;
    maxY: number;
    width: number;
    height: number;
}

/**
 * Convert battle tile coordinates into orthogonal battle-world coordinates.
 * Units are placed at tile centers on a true square battlefield grid.
 */
export function tileToBattleWorld(tileX: number, tileY: number): { x: number; y: number } {
    return {
        x: (tileX + 0.5) * BATTLE_TILE_SIZE,
        y: (tileY + 0.5) * BATTLE_TILE_SIZE,
    };
}

/**
 * Convert battle tile coordinates into the top-left corner of an orthogonal cell.
 */
export function tileToBattleCellOrigin(tileX: number, tileY: number): { x: number; y: number } {
    return {
        x: tileX * BATTLE_TILE_SIZE,
        y: tileY * BATTLE_TILE_SIZE,
    };
}

/**
 * Convert orthogonal battle-world coordinates back to tile-space coordinates.
 */
export function battleWorldToTile(worldX: number, worldY: number): { tx: number; ty: number } {
    return {
        tx: worldX / BATTLE_TILE_SIZE,
        ty: worldY / BATTLE_TILE_SIZE,
    };
}

export function getBattleWorldBounds(mapWidth: number, mapHeight: number): BattleWorldBounds {
    return {
        minX: 0,
        maxX: mapWidth * BATTLE_TILE_SIZE,
        minY: 0,
        maxY: mapHeight * BATTLE_TILE_SIZE,
        width: mapWidth * BATTLE_TILE_SIZE,
        height: mapHeight * BATTLE_TILE_SIZE,
    };
}

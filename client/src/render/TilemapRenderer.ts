import { Container, Sprite } from 'pixi.js';
import { tileToBattleCellOrigin } from './BattleViewProjection';
import { TerrainGenerator } from './TerrainGenerator';
import type { GameBridge } from '../bridge/GameBridge';

/**
 * Creates and manages the orthogonal battle tile grid as PixiJS Sprites.
 */
export class TilemapRenderer {
    readonly container = new Container();

    async build(bridge: GameBridge, terrainGen: TerrainGenerator): Promise<void> {
        const mapW = bridge.getMapWidth();
        const mapH = bridge.getMapHeight();

        for (let y = 0; y < mapH; y++) {
            for (let x = 0; x < mapW; x++) {
                const tile = bridge.getMapTile(x, y);

                // Edge textures only for Impassable tiles on the map border (within 1 tile of edge).
                // Interior Impassable tiles use regular ground textures (they're still blocked gameplay-wise).
                const isMapBorder = x === 0 || y === 0 || x === mapW - 1 || y === mapH - 1;
                const tex = (tile.terrain === 1 && isMapBorder)
                    ? terrainGen.getEdgeTexture(tile.variant)
                    : terrainGen.getTexture(tile.variant);

                const sprite = new Sprite(tex);
                const { x: worldX, y: worldY } = tileToBattleCellOrigin(x, y);

                sprite.anchor.set(0, 0);
                sprite.x = worldX;
                sprite.y = worldY;

                this.container.addChild(sprite);
            }
        }

        this.container.sortableChildren = true;
        for (const child of this.container.children) {
            (child as Sprite).zIndex = (child as Sprite).y;
        }
    }
}

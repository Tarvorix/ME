import { Graphics } from 'pixi.js';
import type { SpritePool } from '../render/SpritePool';

const MAX_INDICATORS = 64;

/**
 * Draws green ellipse indicators under selected units.
 * Maintains a pool of Graphics objects for multi-select.
 */
export class SelectionIndicator {
    private pool: Graphics[] = [];
    private activeCount = 0;

    constructor(private spritePool: SpritePool) {
        // Pre-allocate pool
        for (let i = 0; i < MAX_INDICATORS; i++) {
            const g = new Graphics();
            g.zIndex = -1;
            g.visible = false;
            this.spritePool.container.addChild(g);
            this.pool.push(g);
        }
    }

    update(selectedEntities: Set<number>): void {
        // Hide all previous indicators
        for (let i = 0; i < this.activeCount; i++) {
            this.pool[i].visible = false;
            this.pool[i].clear();
        }
        this.activeCount = 0;

        if (selectedEntities.size === 0) return;

        let idx = 0;
        for (const entityId of selectedEntities) {
            if (idx >= MAX_INDICATORS) break;

            const sprite = this.spritePool.getSprite(entityId);
            if (!sprite) continue;

            const g = this.pool[idx];
            g.visible = true;
            g.x = sprite.x;
            g.y = sprite.y;

            g.ellipse(0, 4, 16, 8);
            g.stroke({ width: 2, color: 0x00ff44, alpha: 0.8 });

            idx++;
        }
        this.activeCount = idx;
    }
}

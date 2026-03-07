import { Graphics } from 'pixi.js';
import type { SpritePool } from '../render/SpritePool';
import { tileToBattleWorld } from '../render/BattleViewProjection';

const FADE_DURATION = 800; // ms

/**
 * Shows a fading diamond marker at the move-order target position.
 */
export class MoveOrderIndicator {
    private graphic: Graphics;
    private fadeStart = 0;
    private active = false;

    constructor(private spritePool: SpritePool) {
        this.graphic = new Graphics();
        this.graphic.zIndex = 999;
        this.spritePool.container.addChild(this.graphic);
    }

    show(tileX: number, tileY: number): void {
        const { x, y } = tileToBattleWorld(tileX, tileY);
        this.graphic.clear();
        this.graphic.x = x;
        this.graphic.y = y;

        // Draw a small diamond marker
        this.graphic.moveTo(0, -8);
        this.graphic.lineTo(12, 0);
        this.graphic.lineTo(0, 8);
        this.graphic.lineTo(-12, 0);
        this.graphic.closePath();
        this.graphic.stroke({ width: 2, color: 0x44ff44 });

        this.graphic.visible = true;
        this.graphic.alpha = 1;
        this.fadeStart = performance.now();
        this.active = true;
    }

    update(): void {
        if (!this.active) return;

        const elapsed = performance.now() - this.fadeStart;
        if (elapsed >= FADE_DURATION) {
            this.graphic.visible = false;
            this.active = false;
            return;
        }

        this.graphic.alpha = 1 - elapsed / FADE_DURATION;
    }
}

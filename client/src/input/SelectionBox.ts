import { Graphics } from 'pixi.js';
import type { SpritePool } from '../render/SpritePool';

/**
 * Drag-select rectangle that highlights entities within the selection area.
 * Renders as a semi-transparent green rectangle during drag.
 */
export class SelectionBox {
    private graphic: Graphics;
    private startX = 0;
    private startY = 0;
    private endX = 0;
    private endY = 0;
    private active = false;

    constructor(private spritePool: SpritePool) {
        this.graphic = new Graphics();
        this.graphic.zIndex = 1000;
        this.graphic.visible = false;
        this.spritePool.container.addChild(this.graphic);
    }

    /** Begin a selection box drag at the given world coordinates. */
    start(worldX: number, worldY: number): void {
        this.startX = worldX;
        this.startY = worldY;
        this.endX = worldX;
        this.endY = worldY;
        this.active = true;
        this.graphic.visible = true;
        this.redraw();
    }

    /** Update the selection box during drag. */
    move(worldX: number, worldY: number): void {
        if (!this.active) return;
        this.endX = worldX;
        this.endY = worldY;
        this.redraw();
    }

    /**
     * End the selection box and return entity IDs within the bounds.
     * Entities are matched by their screen-space sprite position.
     */
    end(): number[] {
        if (!this.active) return [];
        this.active = false;
        this.graphic.visible = false;
        this.graphic.clear();

        const minX = Math.min(this.startX, this.endX);
        const maxX = Math.max(this.startX, this.endX);
        const minY = Math.min(this.startY, this.endY);
        const maxY = Math.max(this.startY, this.endY);

        // If the box is too small, treat as a click not a drag-select
        if (maxX - minX < 5 && maxY - minY < 5) return [];

        return this.spritePool.getEntitiesInRect(minX, minY, maxX, maxY);
    }

    /** Cancel an active selection without selecting. */
    cancel(): void {
        this.active = false;
        this.graphic.visible = false;
        this.graphic.clear();
    }

    private redraw(): void {
        this.graphic.clear();
        const x = Math.min(this.startX, this.endX);
        const y = Math.min(this.startY, this.endY);
        const w = Math.abs(this.endX - this.startX);
        const h = Math.abs(this.endY - this.startY);

        this.graphic.rect(x, y, w, h);
        this.graphic.fill({ color: 0x00ff44, alpha: 0.15 });
        this.graphic.stroke({ width: 1, color: 0x00ff44, alpha: 0.6 });
    }
}

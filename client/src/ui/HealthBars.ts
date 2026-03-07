import { Container, Graphics } from 'pixi.js';

const BAR_WIDTH = 32;
const BAR_HEIGHT = 4;
interface HealthBarEntry {
    gfx: Graphics;
    active: boolean;
}

/**
 * PixiJS-based health bars rendered above damaged units.
 * Only shows bars for units below 100% health.
 * Uses a pool of Graphics objects for performance.
 */
export class HealthBars {
    readonly container = new Container();
    private pool: HealthBarEntry[] = [];
    private activeCount = 0;

    constructor(maxBars: number = 256) {
        for (let i = 0; i < maxBars; i++) {
            const gfx = new Graphics();
            gfx.visible = false;
            this.container.addChild(gfx);
            this.pool.push({ gfx, active: false });
        }
    }

    /**
     * Update health bars from render buffer data.
     * Call after sprite sync so positions are current.
     */
    sync(
        entries: Array<{ x: number; y: number; healthPct: number }>,
    ): void {
        // Reset all
        for (let i = 0; i < this.activeCount; i++) {
            this.pool[i].gfx.visible = false;
            this.pool[i].gfx.clear();
            this.pool[i].active = false;
        }
        this.activeCount = 0;

        for (const entry of entries) {
            if (entry.healthPct >= 100) continue; // Don't show full health
            if (this.activeCount >= this.pool.length) break;

            const bar = this.pool[this.activeCount];
            const gfx = bar.gfx;

            gfx.clear();

            // Background (dark)
            gfx.rect(-BAR_WIDTH / 2, 0, BAR_WIDTH, BAR_HEIGHT);
            gfx.fill({ color: 0x222222, alpha: 0.8 });

            // Health fill
            const fillWidth = (entry.healthPct / 100) * BAR_WIDTH;
            const color = entry.healthPct > 60 ? 0x44cc44 : entry.healthPct > 30 ? 0xcccc44 : 0xcc4444;
            gfx.rect(-BAR_WIDTH / 2, 0, fillWidth, BAR_HEIGHT);
            gfx.fill({ color, alpha: 0.9 });

            // Border
            gfx.rect(-BAR_WIDTH / 2, 0, BAR_WIDTH, BAR_HEIGHT);
            gfx.stroke({ width: 1, color: 0x444444, alpha: 0.6 });

            gfx.x = entry.x;
            gfx.y = entry.y;
            gfx.visible = true;
            bar.active = true;
            this.activeCount++;
        }
    }
}

import { Container, Graphics, Text } from 'pixi.js';
import type { DispatchOrderData, CampaignSiteData } from '../bridge/CampaignTypes';
import { PLAYER_COLORS, NEUTRAL_COLOR, CAMPAIGN_MAP_SCALE } from '../config';

/**
 * Renders animated dispatch route lines on the campaign map.
 * Each active dispatch order is shown as:
 *  - A thin semi-transparent line from source to target site
 *  - A bright traveling dot at the current progress position
 *  - A short trail behind the dot for motion effect
 *  - Unit count label near the dot
 *
 * All graphics are cleared and redrawn each frame since orders
 * change frequently (new dispatches, progress updates, completions).
 */
export class DispatchLineRenderer {
    /** Add this container to the campaign world container. */
    readonly container = new Container();

    /** Graphics for the route lines (drawn behind dots). */
    private lineGfx: Graphics;

    /** Graphics for the traveling dots and trails (drawn above lines). */
    private dotGfx: Graphics;

    /** Pool of Text objects for unit count labels. */
    private labelPool: Text[] = [];

    /** Currently active labels (subset of pool). */
    private activeLabels = 0;

    constructor() {
        this.lineGfx = new Graphics();
        this.container.addChild(this.lineGfx);

        this.dotGfx = new Graphics();
        this.container.addChild(this.dotGfx);
    }

    /**
     * Redraw all dispatch routes based on current orders and site positions.
     * @param orders Active dispatch orders from the campaign bridge.
     * @param sites Current site data for position lookup.
     * @param animTime Accumulated animation time in seconds (for dot pulsing).
     */
    update(orders: DispatchOrderData[], sites: CampaignSiteData[], animTime: number): void {
        this.lineGfx.clear();
        this.dotGfx.clear();

        // Hide all labels first
        for (let i = 0; i < this.activeLabels; i++) {
            this.labelPool[i].visible = false;
        }
        this.activeLabels = 0;

        if (orders.length === 0) return;

        // Build site position lookup (site ID → pixel coords)
        const sitePositions = new Map<number, { x: number; y: number }>();
        for (const site of sites) {
            sitePositions.set(site.siteId, {
                x: site.x * CAMPAIGN_MAP_SCALE,
                y: site.y * CAMPAIGN_MAP_SCALE,
            });
        }

        for (let i = 0; i < orders.length; i++) {
            const order = orders[i];

            const source = sitePositions.get(order.sourceSite);
            const target = sitePositions.get(order.targetSite);
            if (!source || !target) continue;

            // Skip zero-length routes (source == target)
            const dx = target.x - source.x;
            const dy = target.y - source.y;
            if (dx === 0 && dy === 0) continue;

            const color = PLAYER_COLORS[order.player] ?? NEUTRAL_COLOR;

            // Progress: 0.0 at source, 1.0 at target
            const progress = order.totalTime > 0
                ? Math.max(0, Math.min(1, 1.0 - (order.travelRemaining / order.totalTime)))
                : 0;

            // ── Route line ──
            this.lineGfx.moveTo(source.x, source.y);
            this.lineGfx.lineTo(target.x, target.y);
            this.lineGfx.stroke({ color, width: 1.5, alpha: 0.2 });

            // ── Traveling dot position ──
            const dotX = source.x + dx * progress;
            const dotY = source.y + dy * progress;

            // ── Trail behind dot ──
            // Short segment from slightly behind the dot to the dot
            const trailLen = 0.05; // 5% of total path
            const trailProgress = Math.max(0, progress - trailLen);
            const trailX = source.x + dx * trailProgress;
            const trailY = source.y + dy * trailProgress;

            this.dotGfx.moveTo(trailX, trailY);
            this.dotGfx.lineTo(dotX, dotY);
            this.dotGfx.stroke({ color, width: 3, alpha: 0.4 });

            // ── Dot ──
            // Subtle pulse on the dot for liveliness
            const dotAlpha = 0.7 + 0.3 * Math.abs(Math.sin(animTime * 4.0 + i * 1.7));

            this.dotGfx.circle(dotX, dotY, 4);
            this.dotGfx.fill({ color, alpha: dotAlpha });
            this.dotGfx.circle(dotX, dotY, 4);
            this.dotGfx.stroke({ color: 0xFFFFFF, width: 1, alpha: 0.3 });

            // ── Unit count label ──
            if (order.unitCount > 0) {
                const label = this.getOrCreateLabel(this.activeLabels);
                label.text = `${order.unitCount}`;
                label.x = dotX + 7;
                label.y = dotY - 10;
                label.visible = true;
                this.activeLabels++;
            }
        }
    }

    /**
     * Get a label from the pool, creating one if needed.
     */
    private getOrCreateLabel(index: number): Text {
        if (index < this.labelPool.length) {
            return this.labelPool[index];
        }

        const text = new Text({
            text: '',
            style: {
                fill: 0xcccccc,
                fontSize: 9,
                fontFamily: 'Arial, Helvetica, sans-serif',
                fontWeight: 'bold',
            },
        });
        this.container.addChild(text);
        this.labelPool.push(text);
        return text;
    }

    /**
     * Clean up all graphics and labels.
     */
    destroy(): void {
        this.lineGfx.destroy();
        this.dotGfx.destroy();
        for (const label of this.labelPool) {
            label.destroy();
        }
        this.labelPool.length = 0;
        this.activeLabels = 0;
    }
}

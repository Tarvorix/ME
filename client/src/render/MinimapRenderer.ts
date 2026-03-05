import { Container, Graphics } from 'pixi.js';
import type { CampaignBridge } from '../bridge/CampaignBridge';
import type { CampaignRenderer } from './CampaignRenderer';
import type { CameraController } from './CameraController';
import { PLAYER_COLORS, NEUTRAL_COLOR, CAMPAIGN_MAP_SCALE } from '../config';
import { NEUTRAL_OWNER, SiteType } from '../bridge/CampaignTypes';

const MINIMAP_SIZE = 180;
const MINIMAP_PADDING = 8;
const MINIMAP_BG_COLOR = 0x0a0a14;
const MINIMAP_BORDER_COLOR = 0x333355;

/**
 * Minimap renderer for the campaign map.
 * Shows all sites as colored dots, dispatch routes as lines,
 * and the camera viewport rectangle. Clickable to pan the camera.
 *
 * Positioned in the bottom-left corner of the screen (above the site panel).
 * Uses a fixed-size PixiJS Graphics overlay (not part of the world container).
 */
export class MinimapRenderer {
    /** Container added directly to app.stage (not world container). */
    readonly container = new Container();

    private bgGfx: Graphics;
    private contentGfx: Graphics;
    private viewportGfx: Graphics;

    private bridge: CampaignBridge;
    private campaignRenderer: CampaignRenderer;
    private camera: CameraController;

    private mapWidth = 100;
    private mapHeight = 100;
    private scale = 1; // minimap pixels per campaign unit

    private isDragging = false;
    private canvas: HTMLCanvasElement;

    constructor(
        bridge: CampaignBridge,
        campaignRenderer: CampaignRenderer,
        camera: CameraController,
        canvas: HTMLCanvasElement,
    ) {
        this.bridge = bridge;
        this.campaignRenderer = campaignRenderer;
        this.camera = camera;
        this.canvas = canvas;

        const mapSize = bridge.getMapSize();
        this.mapWidth = mapSize.width;
        this.mapHeight = mapSize.height;
        this.scale = MINIMAP_SIZE / Math.max(this.mapWidth, this.mapHeight);

        // Position is set dynamically in update()
        this.container.x = 0;
        this.container.y = 0;

        // Background
        this.bgGfx = new Graphics();
        this.drawBackground();
        this.container.addChild(this.bgGfx);

        // Content (sites, dispatch lines)
        this.contentGfx = new Graphics();
        this.container.addChild(this.contentGfx);

        // Camera viewport rectangle
        this.viewportGfx = new Graphics();
        this.container.addChild(this.viewportGfx);

        // Click-to-pan interaction
        this.canvas.addEventListener('pointerdown', this.onPointerDown);
        this.canvas.addEventListener('pointermove', this.onPointerMove);
        this.canvas.addEventListener('pointerup', this.onPointerUp);
    }

    private drawBackground(): void {
        const w = this.mapWidth * this.scale;
        const h = this.mapHeight * this.scale;

        this.bgGfx.rect(0, 0, w, h);
        this.bgGfx.fill({ color: MINIMAP_BG_COLOR, alpha: 0.85 });
        this.bgGfx.rect(0, 0, w, h);
        this.bgGfx.stroke({ color: MINIMAP_BORDER_COLOR, width: 1, alpha: 0.7 });
    }

    /**
     * Update minimap content each frame.
     * @param screenWidth Current screen width (for right positioning).
     * @param screenHeight Current screen height (for bottom positioning).
     */
    update(screenWidth: number, screenHeight: number): void {
        const minimapW = this.mapWidth * this.scale;
        const minimapH = this.mapHeight * this.scale;
        // Position bottom-right of center area (left of the right panel, which is 240px wide)
        const rightPanelWidth = 240;
        this.container.x = screenWidth - rightPanelWidth - minimapW - MINIMAP_PADDING;
        this.container.y = screenHeight - minimapH - MINIMAP_PADDING;

        this.drawContent();
        this.drawViewport();
    }

    private drawContent(): void {
        this.contentGfx.clear();
        const sites = this.bridge.getSites();
        const orders = this.bridge.getDispatchOrders();

        // Draw dispatch routes
        for (const order of orders) {
            const source = sites.find(s => s.siteId === order.sourceSite);
            const target = sites.find(s => s.siteId === order.targetSite);
            if (!source || !target) continue;

            const color = PLAYER_COLORS[order.player] ?? NEUTRAL_COLOR;
            this.contentGfx.moveTo(source.x * this.scale, source.y * this.scale);
            this.contentGfx.lineTo(target.x * this.scale, target.y * this.scale);
            this.contentGfx.stroke({ color, width: 1, alpha: 0.4 });
        }

        // Draw sites
        for (const site of sites) {
            const sx = site.x * this.scale;
            const sy = site.y * this.scale;
            const color = site.owner === NEUTRAL_OWNER
                ? NEUTRAL_COLOR
                : (PLAYER_COLORS[site.owner] ?? NEUTRAL_COLOR);

            // Forge sites are larger
            const radius = site.siteType === SiteType.Forge ? 4 : 2.5;

            this.contentGfx.circle(sx, sy, radius);
            this.contentGfx.fill({ color, alpha: 0.9 });

            // Battle indicator
            if (site.battleId !== 0) {
                this.contentGfx.circle(sx, sy, radius + 2);
                this.contentGfx.stroke({ color: 0xFF4444, width: 1, alpha: 0.8 });
            }

            // Selected indicator
            if (site.siteId === this.campaignRenderer.selectedSiteId) {
                this.contentGfx.circle(sx, sy, radius + 3);
                this.contentGfx.stroke({ color: 0xFFFFFF, width: 1, alpha: 0.7 });
            }
        }
    }

    private drawViewport(): void {
        this.viewportGfx.clear();

        // Calculate camera viewport in campaign map coordinates
        const worldContainer = this.campaignRenderer.getWorldContainer();
        const zoom = this.camera.getZoom();

        // Top-left of visible area in world coordinates
        const worldX = -worldContainer.x / zoom;
        const worldY = -worldContainer.y / zoom;

        // Visible area size in world coordinates
        const viewW = this.canvas.clientWidth / zoom;
        const viewH = this.canvas.clientHeight / zoom;

        // Convert to minimap coordinates
        const mmX = (worldX / CAMPAIGN_MAP_SCALE) * this.scale;
        const mmY = (worldY / CAMPAIGN_MAP_SCALE) * this.scale;
        const mmW = (viewW / CAMPAIGN_MAP_SCALE) * this.scale;
        const mmH = (viewH / CAMPAIGN_MAP_SCALE) * this.scale;

        this.viewportGfx.rect(mmX, mmY, mmW, mmH);
        this.viewportGfx.stroke({ color: 0xFFFFFF, width: 1, alpha: 0.5 });
    }

    // ── Click-to-Pan ────────────────────────────────────────────────────

    private isInsideMinimap(screenX: number, screenY: number): boolean {
        const bounds = this.container.getBounds();
        return screenX >= bounds.x && screenX <= bounds.x + bounds.width
            && screenY >= bounds.y && screenY <= bounds.y + bounds.height;
    }

    private panToMinimapPoint(screenX: number, screenY: number): void {
        const bounds = this.container.getBounds();
        const localX = screenX - bounds.x;
        const localY = screenY - bounds.y;

        // Convert minimap coords to campaign map coords
        const mapX = localX / this.scale;
        const mapY = localY / this.scale;

        // Convert to world pixel coords
        const worldX = mapX * CAMPAIGN_MAP_SCALE;
        const worldY = mapY * CAMPAIGN_MAP_SCALE;

        // Center camera on this point
        const zoom = this.camera.getZoom();
        const worldContainer = this.campaignRenderer.getWorldContainer();
        worldContainer.x = this.canvas.clientWidth / 2 - worldX * zoom;
        worldContainer.y = this.canvas.clientHeight / 2 - worldY * zoom;
    }

    private onPointerDown = (e: PointerEvent): void => {
        if (e.button !== 0) return;
        if (this.isInsideMinimap(e.clientX, e.clientY)) {
            this.isDragging = true;
            this.panToMinimapPoint(e.clientX, e.clientY);
            e.stopPropagation();
        }
    };

    private onPointerMove = (e: PointerEvent): void => {
        if (!this.isDragging) return;
        this.panToMinimapPoint(e.clientX, e.clientY);
    };

    private onPointerUp = (e: PointerEvent): void => {
        if (e.button !== 0) return;
        this.isDragging = false;
    };

    /** Show/hide the minimap. */
    show(): void { this.container.visible = true; }
    hide(): void { this.container.visible = false; }

    destroy(): void {
        this.canvas.removeEventListener('pointerdown', this.onPointerDown);
        this.canvas.removeEventListener('pointermove', this.onPointerMove);
        this.canvas.removeEventListener('pointerup', this.onPointerUp);
        this.container.destroy({ children: true });
    }
}

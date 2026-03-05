import { Application, Container, Graphics } from 'pixi.js';
import { CampaignSiteSprite } from './CampaignSiteSprite';
import { DispatchLineRenderer } from './DispatchLineRenderer';
import { CameraController } from './CameraController';
import type { CampaignBridge } from '../bridge/CampaignBridge';
import type { CampaignSiteData } from '../bridge/CampaignTypes';
import { CAMPAIGN_MAP_SCALE } from '../config';

/**
 * Main campaign map renderer.
 * Displays all campaign sites, dispatch routes, ownership, garrisons, and battles
 * on a dark industrial grid background. Uses PixiJS for rendering and reuses the
 * existing CameraController for pan/zoom.
 *
 * The campaign map uses a direct coordinate system (not isometric):
 *   - Campaign map is 100×100 float units
 *   - Each unit = CAMPAIGN_MAP_SCALE pixels
 *   - Sites positioned by their (x, y) float coords scaled to screen space
 *
 * This renderer also runs the campaign simulation at a fixed timestep
 * (CAMPAIGN_TICK_MS intervals) via the CampaignBridge.
 */
export class CampaignRenderer {
    private app!: Application;
    private worldContainer!: Container;
    private gridGfx!: Graphics;
    private connectionGfx!: Graphics;
    private siteLayer!: Container;
    private siteSprites: Map<number, CampaignSiteSprite> = new Map();
    private dispatchRenderer!: DispatchLineRenderer;
    private camera!: CameraController;
    private bridge!: CampaignBridge;
    private connectionsDrawn = false;

    // Selection state (managed by CampaignInputManager in Chunk 50)
    private _selectedSiteId = -1;
    private _hoveredSiteId = -1;

    // Animation timing
    private animTime = 0;

    // Cached map dimensions in pixels
    private mapPixelWidth = 0;
    private mapPixelHeight = 0;

    /**
     * Initialize the campaign map renderer.
     * @param app PixiJS Application (shared with battle renderer via GameFlowController).
     * @param bridge Campaign WASM bridge (must already be initialized with initCampaign).
     */
    async init(app: Application, bridge: CampaignBridge): Promise<void> {
        this.app = app;
        this.bridge = bridge;

        // Campaign map dimensions in pixels
        const mapSize = bridge.getMapSize();
        this.mapPixelWidth = mapSize.width * CAMPAIGN_MAP_SCALE;
        this.mapPixelHeight = mapSize.height * CAMPAIGN_MAP_SCALE;

        // ── Container hierarchy ─────────────────────────────────────────

        // World container: everything that moves with the camera
        this.worldContainer = new Container();
        this.app.stage.addChild(this.worldContainer);

        // Grid background (drawn once, never changes)
        this.gridGfx = new Graphics();
        this.drawGrid();
        this.worldContainer.addChild(this.gridGfx);

        // Connection lines between nearby sites (drawn once after first sync)
        this.connectionGfx = new Graphics();
        this.worldContainer.addChild(this.connectionGfx);

        // Dispatch lines (below sites so dots don't obscure icons)
        this.dispatchRenderer = new DispatchLineRenderer();
        this.worldContainer.addChild(this.dispatchRenderer.container);

        // Site layer (above dispatch lines)
        this.siteLayer = new Container();
        this.worldContainer.addChild(this.siteLayer);

        // ── Camera ──────────────────────────────────────────────────────

        // Reuse existing CameraController for middle-mouse pan + scroll zoom
        this.camera = new CameraController(this.worldContainer, this.app.canvas);

        // Zoom out so the entire map is visible, then center
        const screenW = this.app.screen.width;
        const screenH = this.app.screen.height;
        const fitZoom = Math.min(screenW / this.mapPixelWidth, screenH / this.mapPixelHeight) * 0.85;
        const initialZoom = Math.max(0.25, Math.min(1.0, fitZoom));
        this.camera.zoomAt(initialZoom - 1.0, screenW / 2, screenH / 2);

        // Center view on the campaign map center
        const centerX = this.mapPixelWidth / 2;
        const centerY = this.mapPixelHeight / 2;
        this.worldContainer.x = screenW / 2 - centerX * initialZoom;
        this.worldContainer.y = screenH / 2 - centerY * initialZoom;

        // ── Initial site sync ───────────────────────────────────────────
        this.syncSites();
    }

    // ── Grid Drawing ────────────────────────────────────────────────────

    /**
     * Draw the dark industrial grid background.
     * Called once during init (the grid never changes).
     */
    private drawGrid(): void {
        const g = this.gridGfx;
        const w = this.mapPixelWidth;
        const h = this.mapPixelHeight;

        // ── Dark background fill ──
        g.rect(0, 0, w, h);
        g.fill({ color: 0x0a0a1a });

        // ── Sub-grid lines (every 5 campaign units) ──
        const subGrid = 5 * CAMPAIGN_MAP_SCALE;
        const majorGrid = 10 * CAMPAIGN_MAP_SCALE;
        for (let x = subGrid; x < w; x += subGrid) {
            if (x % majorGrid === 0) continue; // skip major lines (drawn separately)
            g.moveTo(x, 0);
            g.lineTo(x, h);
        }
        for (let y = subGrid; y < h; y += subGrid) {
            if (y % majorGrid === 0) continue;
            g.moveTo(0, y);
            g.lineTo(w, y);
        }
        g.stroke({ color: 0x1a1a30, width: 1, alpha: 0.4 });

        // ── Major grid lines (every 10 campaign units) ──
        for (let x = 0; x <= w; x += majorGrid) {
            g.moveTo(x, 0);
            g.lineTo(x, h);
        }
        for (let y = 0; y <= h; y += majorGrid) {
            g.moveTo(0, y);
            g.lineTo(w, y);
        }
        g.stroke({ color: 0x222244, width: 1.5, alpha: 0.5 });

        // ── Map border ──
        g.rect(0, 0, w, h);
        g.stroke({ color: 0x3a3a5a, width: 3, alpha: 0.7 });

        // ── Corner brackets (industrial aesthetic) ──
        const bracketLen = 40;
        // Top-left
        g.moveTo(0, bracketLen);
        g.lineTo(0, 0);
        g.lineTo(bracketLen, 0);
        // Top-right
        g.moveTo(w - bracketLen, 0);
        g.lineTo(w, 0);
        g.lineTo(w, bracketLen);
        // Bottom-left
        g.moveTo(0, h - bracketLen);
        g.lineTo(0, h);
        g.lineTo(bracketLen, h);
        // Bottom-right
        g.moveTo(w - bracketLen, h);
        g.lineTo(w, h);
        g.lineTo(w, h - bracketLen);
        g.stroke({ color: 0x4444aa, width: 3, alpha: 0.6 });

        // ── Center crosshair (subtle map center indicator) ──
        const cx = w / 2;
        const cy = h / 2;
        const crossLen = 16;
        g.moveTo(cx - crossLen, cy);
        g.lineTo(cx + crossLen, cy);
        g.moveTo(cx, cy - crossLen);
        g.lineTo(cx, cy + crossLen);
        g.stroke({ color: 0x2a2a4a, width: 1.5, alpha: 0.4 });
    }

    // ── Update Loop ─────────────────────────────────────────────────────

    /**
     * Called every frame from the game loop.
     * Updates visual elements only (sites, dispatch lines, animations).
     * Campaign simulation ticks are driven by the GameFlowController.
     * @param dt Frame delta time in milliseconds.
     */
    update(dt: number): void {
        // Advance animation clock (in seconds for smooth sin/cos oscillations)
        this.animTime += dt / 1000;

        // Sync visuals with latest bridge data
        this.syncSites();
        this.syncDispatches();
    }

    /** Show the campaign map (re-enable visibility). */
    show(): void {
        this.worldContainer.visible = true;
    }

    /** Hide the campaign map (disable visibility during battle view). */
    hide(): void {
        this.worldContainer.visible = false;
    }

    /**
     * Disable the campaign camera controller (removes event listeners).
     * Call when entering battle view to prevent competing camera controllers.
     */
    disableCamera(): void {
        this.camera.destroy();
    }

    /**
     * Re-enable the campaign camera controller (recreates event listeners).
     * Call when returning from battle view to campaign map.
     */
    enableCamera(): void {
        this.camera = new CameraController(this.worldContainer, this.app.canvas);
    }

    /**
     * Create, update, and remove site sprites to match current bridge data.
     */
    private syncSites(): void {
        const sites = this.bridge.getSites();
        const liveSiteIds = new Set<number>();

        for (const site of sites) {
            liveSiteIds.add(site.siteId);

            let sprite = this.siteSprites.get(site.siteId);
            if (!sprite) {
                // First time seeing this site — create its sprite
                sprite = new CampaignSiteSprite(site);
                this.siteSprites.set(site.siteId, sprite);
                this.siteLayer.addChild(sprite);
            }

            // Update position (campaign coords → pixel coords)
            sprite.position.set(
                site.x * CAMPAIGN_MAP_SCALE,
                site.y * CAMPAIGN_MAP_SCALE,
            );

            // Update data (ownership, garrison, battle state)
            sprite.updateData(site, this.animTime);

            // Update selection/hover state
            sprite.setSelected(site.siteId === this._selectedSiteId);
            sprite.setHovered(site.siteId === this._hoveredSiteId);
        }

        // Remove sprites for sites that no longer exist
        for (const [id, sprite] of this.siteSprites) {
            if (!liveSiteIds.has(id)) {
                this.siteLayer.removeChild(sprite);
                sprite.destroy({ children: true });
                this.siteSprites.delete(id);
            }
        }

        // Draw connection lines once (sites don't move)
        if (!this.connectionsDrawn && sites.length > 0) {
            this.drawConnectionLines(sites);
            this.connectionsDrawn = true;
        }
    }

    /**
     * Draw subtle connection lines between nearby sites for visual context.
     * Creates a network-like appearance on the campaign map.
     */
    private drawConnectionLines(sites: CampaignSiteData[]): void {
        const g = this.connectionGfx;
        g.clear();

        // Connect sites within a threshold distance (30 campaign units)
        const threshold = 30;
        const thresholdSq = threshold * threshold;

        for (let i = 0; i < sites.length; i++) {
            for (let j = i + 1; j < sites.length; j++) {
                const a = sites[i];
                const b = sites[j];
                const dx = a.x - b.x;
                const dy = a.y - b.y;
                const distSq = dx * dx + dy * dy;

                if (distSq < thresholdSq) {
                    g.moveTo(a.x * CAMPAIGN_MAP_SCALE, a.y * CAMPAIGN_MAP_SCALE);
                    g.lineTo(b.x * CAMPAIGN_MAP_SCALE, b.y * CAMPAIGN_MAP_SCALE);
                }
            }
        }
        g.stroke({ color: 0x1a1a3a, width: 1, alpha: 0.25 });
    }

    /**
     * Sync dispatch route visualizations with current bridge data.
     */
    private syncDispatches(): void {
        const orders = this.bridge.getDispatchOrders();
        const sites = this.bridge.getSites();
        this.dispatchRenderer.update(orders, sites, this.animTime);
    }

    // ── Public API (used by CampaignInputManager, CampaignHUD, etc.) ────

    /** Currently selected site ID, or -1 for no selection. */
    get selectedSiteId(): number { return this._selectedSiteId; }
    set selectedSiteId(id: number) { this._selectedSiteId = id; }

    /** Currently hovered site ID, or -1 for no hover. */
    get hoveredSiteId(): number { return this._hoveredSiteId; }
    set hoveredSiteId(id: number) { this._hoveredSiteId = id; }

    /** Get the camera controller for coordinate conversion and zoom info. */
    getCamera(): CameraController { return this.camera; }

    /** Get the world container (for adding overlays, etc.). */
    getWorldContainer(): Container { return this.worldContainer; }

    /** Get the campaign bridge reference. */
    getBridge(): CampaignBridge { return this.bridge; }

    /** Get the PixiJS application reference. */
    getApp(): Application { return this.app; }

    /**
     * Hit-test: find which site (if any) is at the given screen position.
     * Returns site ID or -1 if no site is within the hit radius.
     */
    hitTestSite(screenX: number, screenY: number): number {
        const { wx, wy } = this.camera.screenToWorld(screenX, screenY);
        const hitRadiusSq = 50 * 50; // 50px hit radius in world space (scaled for larger icons)

        let closestId = -1;
        let closestDistSq = hitRadiusSq;

        for (const [id, sprite] of this.siteSprites) {
            const dx = sprite.x - wx;
            const dy = sprite.y - wy;
            const distSq = dx * dx + dy * dy;
            if (distSq < closestDistSq) {
                closestDistSq = distSq;
                closestId = id;
            }
        }

        return closestId;
    }

    /**
     * Get a specific site sprite by ID (for positioning UI elements, etc.).
     */
    getSiteSprite(siteId: number): CampaignSiteSprite | undefined {
        return this.siteSprites.get(siteId);
    }

    /**
     * Get all site sprites (for minimap, batch operations, etc.).
     */
    getAllSiteSprites(): Map<number, CampaignSiteSprite> {
        return this.siteSprites;
    }

    /**
     * Clean up all renderer resources.
     * Removes event listeners, destroys all graphics and containers.
     */
    destroy(): void {
        // Remove camera event listeners
        this.camera.destroy();

        // Destroy dispatch renderer
        this.dispatchRenderer.destroy();

        // Destroy all site sprites
        for (const sprite of this.siteSprites.values()) {
            sprite.destroy({ children: true });
        }
        this.siteSprites.clear();

        // Remove world container from stage and destroy
        if (this.worldContainer.parent) {
            this.worldContainer.parent.removeChild(this.worldContainer);
        }
        this.worldContainer.destroy({ children: true });
    }
}

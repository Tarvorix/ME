import type { CampaignRenderer } from '../render/CampaignRenderer';
import type { CampaignHUD } from '../ui/CampaignHUD';

/**
 * Handles user input on the campaign map.
 *
 * Mouse:
 *  - Left-click site: select it (show SitePanel)
 *  - Left-click empty: deselect
 *  - Right-click site (with source selected): open dispatch dialog
 *  - Middle-mouse drag: pan camera (handled by CameraController)
 *  - Scroll wheel: zoom camera (handled by CameraController)
 *
 * Keyboard:
 *  - Space: pause/unpause campaign
 *  - R: toggle research panel
 *  - Escape: close overlays, deselect
 */
export class CampaignInputManager {
    private canvas: HTMLCanvasElement;
    private renderer: CampaignRenderer;
    private hud: CampaignHUD;

    constructor(
        canvas: HTMLCanvasElement,
        renderer: CampaignRenderer,
        hud: CampaignHUD,
    ) {
        this.canvas = canvas;
        this.renderer = renderer;
        this.hud = hud;

        // Mouse input
        this.canvas.addEventListener('pointerdown', this.onPointerDown);

        // Hover tracking
        this.canvas.addEventListener('pointermove', this.onPointerMove);

        // Prevent browser context menu so right-click dispatch works cleanly
        this.canvas.addEventListener('contextmenu', this.onContextMenu);

        // Keyboard input
        window.addEventListener('keydown', this.onKeyDown);
    }

    /** Update per-frame visuals (hover highlighting). */
    update(): void {
        // Hover is updated via pointermove events
    }

    /** Clean up event listeners. */
    destroy(): void {
        this.canvas.removeEventListener('pointerdown', this.onPointerDown);
        this.canvas.removeEventListener('pointermove', this.onPointerMove);
        this.canvas.removeEventListener('contextmenu', this.onContextMenu);
        window.removeEventListener('keydown', this.onKeyDown);
    }

    // ── Mouse Input ─────────────────────────────────────────────────────

    private onContextMenu = (e: Event): void => {
        e.preventDefault();
    };

    private onPointerDown = (e: PointerEvent): void => {
        // Ignore touch events (could add touch support later)
        if (e.pointerType === 'touch') return;
        // Ignore middle-mouse (handled by CameraController for pan)
        if (e.button === 1) return;

        if (e.button === 0) {
            this.handleLeftClick(e.clientX, e.clientY);
        } else if (e.button === 2) {
            this.handleRightClick(e.clientX, e.clientY);
        }
    };

    private onPointerMove = (e: PointerEvent): void => {
        // Update hover highlight
        const siteId = this.renderer.hitTestSite(e.clientX, e.clientY);
        this.renderer.hoveredSiteId = siteId;
    };

    private handleLeftClick(screenX: number, screenY: number): void {
        // If an overlay is open, don't process map clicks
        if (this.hud.isResearchOpen() || this.hud.isDispatchOpen()) return;

        const siteId = this.renderer.hitTestSite(screenX, screenY);

        // Check if we're waiting for a dispatch target
        if (this.hud.isWaitingForDispatchTarget()) {
            if (siteId >= 0) {
                this.hud.setDispatchTarget(siteId);
            }
            return;
        }

        // Normal selection
        if (siteId >= 0) {
            this.renderer.selectedSiteId = siteId;
        } else {
            // Clicked empty space: deselect
            this.renderer.selectedSiteId = -1;
        }
    }

    private handleRightClick(screenX: number, screenY: number): void {
        // If an overlay is open, don't process
        if (this.hud.isResearchOpen() || this.hud.isDispatchOpen()) return;

        const targetSiteId = this.renderer.hitTestSite(screenX, screenY);
        if (targetSiteId < 0) return;

        // Right-click on a site: if we have a selected owned site, treat as dispatch target
        const selectedId = this.renderer.selectedSiteId;
        if (selectedId >= 0 && selectedId !== targetSiteId) {
            // Check if the selected site is player-owned
            const bridge = this.renderer.getBridge();
            const sites = bridge.getSites();
            const selectedSite = sites.find(s => s.siteId === selectedId);
            if (selectedSite && selectedSite.owner === 0) {
                const totalGarrison = selectedSite.garrisonThralls + selectedSite.garrisonSentinels + selectedSite.garrisonTanks;
                if (totalGarrison > 0) {
                    // Open dispatch dialog: selected site → right-clicked site
                    this.hud.setDispatchTarget(targetSiteId);
                    // The CampaignHUD handleDispatchFrom was called when the Dispatch button was clicked,
                    // or we can initiate it directly for right-click dispatch
                    if (!this.hud.isWaitingForDispatchTarget()) {
                        // Direct right-click dispatch: set source and target at once
                        (this.hud as any).dispatchSourceId = selectedId;
                        this.hud.setDispatchTarget(targetSiteId);
                    }
                }
            }
        }
    }

    // ── Keyboard Input ──────────────────────────────────────────────────

    private onKeyDown = (e: KeyboardEvent): void => {
        // Don't handle input if an input field is focused
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

        switch (e.key) {
            case ' ':
                e.preventDefault();
                // Toggle pause
                {
                    const bridge = this.renderer.getBridge();
                    bridge.setPaused(!bridge.isPaused());
                }
                break;

            case 'r':
            case 'R':
                // Toggle research panel
                this.hud.toggleResearch();
                break;

            case 'Escape':
                // Close overlays first, then deselect
                if (this.hud.isResearchOpen() || this.hud.isDispatchOpen()) {
                    this.hud.closeOverlays();
                } else {
                    this.renderer.selectedSiteId = -1;
                }
                break;
        }
    };
}

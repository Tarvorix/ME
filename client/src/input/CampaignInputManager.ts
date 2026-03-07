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
 * Touch (mobile):
 *  - Single-finger tap: select/deselect site
 *  - Single-finger drag: pan camera
 *  - Two-finger pinch: zoom camera
 *  - Long press on site: open dispatch (like right-click)
 *
 * Keyboard:
 *  - Space: pause/unpause campaign
 *  - R: toggle research panel
 *  - Escape: close overlays, deselect
 */

const TOUCH_TAP_MAX_DIST = 12; // pixels
const TOUCH_TAP_MAX_TIME = 300; // ms
const TOUCH_LONG_PRESS_TIME = 500; // ms
const TOUCH_DRAG_THRESHOLD = 8; // pixels before drag starts

interface TrackedTouch {
    id: number;
    startX: number;
    startY: number;
    currentX: number;
    currentY: number;
    startTime: number;
}

export class CampaignInputManager {
    private canvas: HTMLCanvasElement;
    private renderer: CampaignRenderer;
    private hud: CampaignHUD;

    // Touch state
    private touches = new Map<number, TrackedTouch>();
    private touchDragging = false;
    private longPressTimer: ReturnType<typeof setTimeout> | null = null;
    private prevPinchDist = 0;
    private prevPanCenter = { x: 0, y: 0 };

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

        // Touch input (separate from pointer events for gesture recognition)
        this.canvas.addEventListener('touchstart', this.onTouchStart, { passive: false });
        this.canvas.addEventListener('touchmove', this.onTouchMove, { passive: false });
        this.canvas.addEventListener('touchend', this.onTouchEnd, { passive: false });
        this.canvas.addEventListener('touchcancel', this.onTouchEnd, { passive: false });
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
        this.canvas.removeEventListener('touchstart', this.onTouchStart);
        this.canvas.removeEventListener('touchmove', this.onTouchMove);
        this.canvas.removeEventListener('touchend', this.onTouchEnd);
        this.canvas.removeEventListener('touchcancel', this.onTouchEnd);
        this.clearLongPress();
    }

    // ── Mouse Input ─────────────────────────────────────────────────────

    private onContextMenu = (e: Event): void => {
        e.preventDefault();
    };

    private onPointerDown = (e: PointerEvent): void => {
        // Touch events are handled by the touch* listeners for gesture recognition
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

    // ── Touch Input ──────────────────────────────────────────────────────

    private clearLongPress(): void {
        if (this.longPressTimer !== null) {
            clearTimeout(this.longPressTimer);
            this.longPressTimer = null;
        }
    }

    private touchDistance(t1: TrackedTouch, t2: TrackedTouch): number {
        const dx = t1.currentX - t2.currentX;
        const dy = t1.currentY - t2.currentY;
        return Math.sqrt(dx * dx + dy * dy);
    }

    private touchCenter(t1: TrackedTouch, t2: TrackedTouch): { x: number; y: number } {
        return {
            x: (t1.currentX + t2.currentX) / 2,
            y: (t1.currentY + t2.currentY) / 2,
        };
    }

    private onTouchStart = (e: TouchEvent): void => {
        e.preventDefault();

        for (let i = 0; i < e.changedTouches.length; i++) {
            const t = e.changedTouches[i];
            this.touches.set(t.identifier, {
                id: t.identifier,
                startX: t.clientX,
                startY: t.clientY,
                currentX: t.clientX,
                currentY: t.clientY,
                startTime: performance.now(),
            });
        }

        if (this.touches.size === 1) {
            // Single touch — start long press timer for dispatch
            const touch = Array.from(this.touches.values())[0];
            this.clearLongPress();
            this.longPressTimer = setTimeout(() => {
                // Long press = right-click equivalent (dispatch)
                this.handleRightClick(touch.currentX, touch.currentY);
                this.longPressTimer = null;
            }, TOUCH_LONG_PRESS_TIME);
        } else if (this.touches.size === 2) {
            // Two fingers — initialize pinch/pan
            this.clearLongPress();
            this.touchDragging = false;
            const [t1, t2] = Array.from(this.touches.values());
            this.prevPinchDist = this.touchDistance(t1, t2);
            this.prevPanCenter = this.touchCenter(t1, t2);
        }
    };

    private onTouchMove = (e: TouchEvent): void => {
        e.preventDefault();

        for (let i = 0; i < e.changedTouches.length; i++) {
            const t = e.changedTouches[i];
            const tracked = this.touches.get(t.identifier);
            if (tracked) {
                tracked.currentX = t.clientX;
                tracked.currentY = t.clientY;
            }
        }

        if (this.touches.size === 1) {
            // Single finger drag = pan camera
            const touch = Array.from(this.touches.values())[0];
            const dx = touch.currentX - touch.startX;
            const dy = touch.currentY - touch.startY;
            const dist = Math.sqrt(dx * dx + dy * dy);

            if (dist > TOUCH_DRAG_THRESHOLD) {
                this.clearLongPress();

                if (!this.touchDragging) {
                    this.touchDragging = true;
                    // Store previous position for incremental panning
                    (touch as any)._prevX = touch.startX;
                    (touch as any)._prevY = touch.startY;
                }

                const prevX = (touch as any)._prevX ?? touch.startX;
                const prevY = (touch as any)._prevY ?? touch.startY;
                const panDx = touch.currentX - prevX;
                const panDy = touch.currentY - prevY;
                (touch as any)._prevX = touch.currentX;
                (touch as any)._prevY = touch.currentY;

                this.renderer.getCamera().pan(panDx, panDy);
            }
        } else if (this.touches.size === 2) {
            const [t1, t2] = Array.from(this.touches.values());

            // Pinch zoom
            const newDist = this.touchDistance(t1, t2);
            if (this.prevPinchDist > 0) {
                const zoomDelta = (newDist - this.prevPinchDist) * 0.005;
                const c = this.touchCenter(t1, t2);
                if (Math.abs(zoomDelta) > 0.001) {
                    this.renderer.getCamera().zoomAt(zoomDelta, c.x, c.y);
                }
            }
            this.prevPinchDist = newDist;

            // Two-finger pan
            const newCenter = this.touchCenter(t1, t2);
            const panDx = newCenter.x - this.prevPanCenter.x;
            const panDy = newCenter.y - this.prevPanCenter.y;
            if (Math.abs(panDx) > 0.5 || Math.abs(panDy) > 0.5) {
                this.renderer.getCamera().pan(panDx, panDy);
            }
            this.prevPanCenter = newCenter;
        }
    };

    private onTouchEnd = (e: TouchEvent): void => {
        e.preventDefault();

        for (let i = 0; i < e.changedTouches.length; i++) {
            const t = e.changedTouches[i];
            const tracked = this.touches.get(t.identifier);

            if (tracked && this.touches.size === 1) {
                // Single touch ending
                this.clearLongPress();
                const elapsed = performance.now() - tracked.startTime;
                const dx = t.clientX - tracked.startX;
                const dy = t.clientY - tracked.startY;
                const dist = Math.sqrt(dx * dx + dy * dy);

                if (!this.touchDragging && dist < TOUCH_TAP_MAX_DIST && elapsed < TOUCH_TAP_MAX_TIME) {
                    // Tap = select site (like left-click)
                    this.handleLeftClick(t.clientX, t.clientY);
                }
                this.touchDragging = false;
            }

            this.touches.delete(t.identifier);
        }

        if (this.touches.size === 0) {
            this.touchDragging = false;
            this.prevPinchDist = 0;
        }
    };
}

/**
 * Touch gesture recognizer for iOS Safari and desktop touch.
 * Detects: single tap, single drag (selection box), two-finger pan, pinch zoom.
 */

export interface TouchCallbacks {
    onTapSelect: (screenX: number, screenY: number) => void;
    onDragStart: (screenX: number, screenY: number) => void;
    onDragMove: (screenX: number, screenY: number) => void;
    onDragEnd: (screenX: number, screenY: number) => void;
    onPan: (dx: number, dy: number) => void;
    onZoom: (delta: number, centerX: number, centerY: number) => void;
    onLongPress: (screenX: number, screenY: number) => void;
}

const TAP_MAX_DISTANCE = 10; // pixels
const TAP_MAX_TIME = 300; // ms
const LONG_PRESS_TIME = 500; // ms
const DRAG_THRESHOLD = 8; // pixels before drag starts

interface TrackedTouch {
    id: number;
    startX: number;
    startY: number;
    currentX: number;
    currentY: number;
    startTime: number;
}

export class TouchHandler {
    private touches = new Map<number, TrackedTouch>();
    private isDragging = false;
    private longPressTimer: ReturnType<typeof setTimeout> | null = null;
    private prevPinchDist = 0;
    private prevPanCenter = { x: 0, y: 0 };

    constructor(
        private canvas: HTMLCanvasElement,
        private callbacks: TouchCallbacks,
    ) {
        // Use pointer events for unified mouse + touch handling
        this.canvas.addEventListener('touchstart', this.onTouchStart, { passive: false });
        this.canvas.addEventListener('touchmove', this.onTouchMove, { passive: false });
        this.canvas.addEventListener('touchend', this.onTouchEnd, { passive: false });
        this.canvas.addEventListener('touchcancel', this.onTouchEnd, { passive: false });
    }

    destroy(): void {
        this.canvas.removeEventListener('touchstart', this.onTouchStart);
        this.canvas.removeEventListener('touchmove', this.onTouchMove);
        this.canvas.removeEventListener('touchend', this.onTouchEnd);
        this.canvas.removeEventListener('touchcancel', this.onTouchEnd);
        this.clearLongPress();
    }

    private clearLongPress(): void {
        if (this.longPressTimer !== null) {
            clearTimeout(this.longPressTimer);
            this.longPressTimer = null;
        }
    }

    private onTouchStart = (e: TouchEvent): void => {
        e.preventDefault(); // Prevent browser gestures (scroll, zoom)

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
            // Single touch — start long press timer
            const touch = Array.from(this.touches.values())[0];
            this.clearLongPress();
            this.longPressTimer = setTimeout(() => {
                this.callbacks.onLongPress(touch.currentX, touch.currentY);
                this.longPressTimer = null;
            }, LONG_PRESS_TIME);
        } else if (this.touches.size === 2) {
            // Two fingers — initialize pinch/pan
            this.clearLongPress();
            this.isDragging = false;
            const [t1, t2] = Array.from(this.touches.values());
            this.prevPinchDist = this.distance(t1, t2);
            this.prevPanCenter = this.center(t1, t2);
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
            const touch = Array.from(this.touches.values())[0];
            const dx = touch.currentX - touch.startX;
            const dy = touch.currentY - touch.startY;
            const dist = Math.sqrt(dx * dx + dy * dy);

            if (dist > DRAG_THRESHOLD) {
                this.clearLongPress();

                if (!this.isDragging) {
                    this.isDragging = true;
                    this.callbacks.onDragStart(touch.startX, touch.startY);
                }
                this.callbacks.onDragMove(touch.currentX, touch.currentY);
            }
        } else if (this.touches.size === 2) {
            const [t1, t2] = Array.from(this.touches.values());

            // Pinch zoom
            const newDist = this.distance(t1, t2);
            if (this.prevPinchDist > 0) {
                const zoomDelta = (newDist - this.prevPinchDist) * 0.005;
                const c = this.center(t1, t2);
                if (Math.abs(zoomDelta) > 0.001) {
                    this.callbacks.onZoom(zoomDelta, c.x, c.y);
                }
            }
            this.prevPinchDist = newDist;

            // Two-finger pan
            const newCenter = this.center(t1, t2);
            const panDx = newCenter.x - this.prevPanCenter.x;
            const panDy = newCenter.y - this.prevPanCenter.y;
            if (Math.abs(panDx) > 0.5 || Math.abs(panDy) > 0.5) {
                this.callbacks.onPan(panDx, panDy);
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

                if (this.isDragging) {
                    this.callbacks.onDragEnd(t.clientX, t.clientY);
                    this.isDragging = false;
                } else if (dist < TAP_MAX_DISTANCE && elapsed < TAP_MAX_TIME) {
                    // Tap select
                    this.callbacks.onTapSelect(t.clientX, t.clientY);
                }
            }

            this.touches.delete(t.identifier);
        }

        if (this.touches.size === 0) {
            this.isDragging = false;
            this.prevPinchDist = 0;
        }
    };

    private distance(t1: TrackedTouch, t2: TrackedTouch): number {
        const dx = t1.currentX - t2.currentX;
        const dy = t1.currentY - t2.currentY;
        return Math.sqrt(dx * dx + dy * dy);
    }

    private center(t1: TrackedTouch, t2: TrackedTouch): { x: number; y: number } {
        return {
            x: (t1.currentX + t2.currentX) / 2,
            y: (t1.currentY + t2.currentY) / 2,
        };
    }
}

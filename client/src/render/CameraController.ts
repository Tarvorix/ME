import { Container } from 'pixi.js';
import { TILE_WIDTH, TILE_HEIGHT, MAP_WIDTH, MAP_HEIGHT } from '../config';
import { tileToScreen } from './IsoUtils';

const MIN_ZOOM = 0.25;
const MAX_ZOOM = 2.0;
const ZOOM_SPEED = 0.1;

/**
 * Camera controller for panning and zooming the isometric view.
 * Middle-mouse drag to pan, mouse wheel to zoom, touch gestures supported via external callbacks.
 */
export class CameraController {
    private dragging = false;
    private lastX = 0;
    private lastY = 0;
    private zoom = 1.0;

    constructor(
        private worldContainer: Container,
        private canvas: HTMLCanvasElement,
    ) {
        this.canvas.addEventListener('pointerdown', this.onPointerDown);
        this.canvas.addEventListener('pointermove', this.onPointerMove);
        this.canvas.addEventListener('pointerup', this.onPointerUp);
        this.canvas.addEventListener('pointerleave', this.onPointerUp);
        this.canvas.addEventListener('wheel', this.onWheel, { passive: false });
        // Prevent context menu on right-click
        this.canvas.addEventListener('contextmenu', (e) => e.preventDefault());
    }

    /**
     * Center the camera on the isometric map diamond and auto-fit zoom.
     * Computes the full isometric diamond bounds from the 4 map corners,
     * then calculates a zoom level that fits the diamond within the screen
     * (with 10% margin), clamped to [MIN_ZOOM, MAX_ZOOM].
     */
    centerOnMap(screenWidth: number, screenHeight: number): void {
        // Compute isometric screen positions of the 4 map corners
        const topLeft = tileToScreen(0, 0);
        const topRight = tileToScreen(MAP_WIDTH, 0);
        const bottomLeft = tileToScreen(0, MAP_HEIGHT);
        const bottomRight = tileToScreen(MAP_WIDTH, MAP_HEIGHT);

        // Find bounding box of the isometric diamond
        const minX = Math.min(topLeft.sx, topRight.sx, bottomLeft.sx, bottomRight.sx);
        const maxX = Math.max(topLeft.sx, topRight.sx, bottomLeft.sx, bottomRight.sx);
        const minY = Math.min(topLeft.sy, topRight.sy, bottomLeft.sy, bottomRight.sy);
        const maxY = Math.max(topLeft.sy, topRight.sy, bottomLeft.sy, bottomRight.sy);

        const mapW = maxX - minX;
        const mapH = maxY - minY;

        // Center of the diamond in world space
        const centerX = (minX + maxX) / 2;
        const centerY = (minY + maxY) / 2;

        // Compute zoom to fit diamond with 10% margin, clamped to valid range
        const fitZoom = Math.min(
            (screenWidth * 0.9) / mapW,
            (screenHeight * 0.9) / mapH,
        );
        this.zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, fitZoom));
        this.worldContainer.scale.set(this.zoom);

        // Position so diamond center is at screen center
        this.worldContainer.x = screenWidth / 2 - centerX * this.zoom;
        this.worldContainer.y = screenHeight / 2 - centerY * this.zoom;
    }

    /** Convert screen coords to world coords (accounting for camera offset and zoom). */
    screenToWorld(screenX: number, screenY: number): { wx: number; wy: number } {
        return {
            wx: (screenX - this.worldContainer.x) / this.zoom,
            wy: (screenY - this.worldContainer.y) / this.zoom,
        };
    }

    /** Pan the camera by the given screen-space delta. */
    pan(dx: number, dy: number): void {
        this.worldContainer.x += dx;
        this.worldContainer.y += dy;
    }

    /** Zoom the camera by a delta amount, centered on a screen point. */
    zoomAt(delta: number, screenX: number, screenY: number): void {
        const oldZoom = this.zoom;
        this.zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, this.zoom + delta));

        // Adjust position so the zoom is centered on the focal point
        const ratio = this.zoom / oldZoom;
        this.worldContainer.x = screenX - (screenX - this.worldContainer.x) * ratio;
        this.worldContainer.y = screenY - (screenY - this.worldContainer.y) * ratio;
        this.worldContainer.scale.set(this.zoom);
    }

    getZoom(): number {
        return this.zoom;
    }

    destroy(): void {
        this.canvas.removeEventListener('pointerdown', this.onPointerDown);
        this.canvas.removeEventListener('pointermove', this.onPointerMove);
        this.canvas.removeEventListener('pointerup', this.onPointerUp);
        this.canvas.removeEventListener('pointerleave', this.onPointerUp);
        this.canvas.removeEventListener('wheel', this.onWheel);
    }

    private onPointerDown = (e: PointerEvent): void => {
        // Middle mouse button (button 1)
        if (e.button === 1) {
            this.dragging = true;
            this.lastX = e.clientX;
            this.lastY = e.clientY;
            e.preventDefault();
        }
    };

    private onPointerMove = (e: PointerEvent): void => {
        if (!this.dragging) return;

        const dx = e.clientX - this.lastX;
        const dy = e.clientY - this.lastY;
        this.worldContainer.x += dx;
        this.worldContainer.y += dy;
        this.lastX = e.clientX;
        this.lastY = e.clientY;
    };

    private onPointerUp = (e: PointerEvent): void => {
        if (e.button === 1) {
            this.dragging = false;
        }
    };

    private onWheel = (e: WheelEvent): void => {
        e.preventDefault();
        const delta = -Math.sign(e.deltaY) * ZOOM_SPEED;
        this.zoomAt(delta, e.clientX, e.clientY);
    };
}

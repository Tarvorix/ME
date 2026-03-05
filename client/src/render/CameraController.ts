import { Container } from 'pixi.js';
import { TILE_WIDTH, TILE_HEIGHT, MAP_WIDTH, MAP_HEIGHT } from '../config';
import { tileToScreen } from './IsoUtils';

const MIN_ZOOM = 0.25;
const MAX_ZOOM = 2.0;
const ZOOM_SPEED = 0.1;
const PAN_SPEED = 8; // pixels per frame for WASD/arrow key panning
const EDGE_SCROLL_MARGIN = 20; // pixels from screen edge to trigger edge scrolling
const EDGE_SCROLL_SPEED = 6; // pixels per frame for edge scrolling

/**
 * Camera controller for panning and zooming the isometric view.
 * Pan: WASD / arrow keys, edge scrolling, middle-mouse drag, or touch two-finger drag.
 * Zoom: mouse wheel / trackpad scroll, pinch gesture via external callbacks.
 */
export class CameraController {
    private dragging = false;
    private lastX = 0;
    private lastY = 0;
    private zoom = 1.0;

    // Keyboard pan state
    private keysDown = new Set<string>();
    private keyDownHandler: ((e: KeyboardEvent) => void) | null = null;
    private keyUpHandler: ((e: KeyboardEvent) => void) | null = null;

    // Edge scroll state
    private mouseX = 0;
    private mouseY = 0;
    private mouseMoveHandler: ((e: MouseEvent) => void) | null = null;

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

        // WASD / arrow key panning
        this.keyDownHandler = (e: KeyboardEvent) => {
            const key = e.key.toLowerCase();
            if (key === 'w' || key === 'a' || key === 's' || key === 'd' ||
                key === 'arrowup' || key === 'arrowdown' || key === 'arrowleft' || key === 'arrowright') {
                this.keysDown.add(key);
                e.preventDefault();
            }
        };
        this.keyUpHandler = (e: KeyboardEvent) => {
            this.keysDown.delete(e.key.toLowerCase());
        };
        window.addEventListener('keydown', this.keyDownHandler);
        window.addEventListener('keyup', this.keyUpHandler);

        // Track mouse position for edge scrolling
        this.mouseMoveHandler = (e: MouseEvent) => {
            this.mouseX = e.clientX;
            this.mouseY = e.clientY;
        };
        window.addEventListener('mousemove', this.mouseMoveHandler);
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

    /**
     * Call once per frame to apply keyboard pan and edge scrolling.
     * Must be called from the game loop (e.g. GameRenderer.onFrame or CampaignRenderer.update).
     */
    update(): void {
        let dx = 0;
        let dy = 0;

        // WASD / arrow key panning
        if (this.keysDown.has('w') || this.keysDown.has('arrowup')) dy += PAN_SPEED;
        if (this.keysDown.has('s') || this.keysDown.has('arrowdown')) dy -= PAN_SPEED;
        if (this.keysDown.has('a') || this.keysDown.has('arrowleft')) dx += PAN_SPEED;
        if (this.keysDown.has('d') || this.keysDown.has('arrowright')) dx -= PAN_SPEED;

        // Edge scrolling (mouse near screen edge)
        const w = window.innerWidth;
        const h = window.innerHeight;
        if (this.mouseX < EDGE_SCROLL_MARGIN) dx += EDGE_SCROLL_SPEED;
        if (this.mouseX > w - EDGE_SCROLL_MARGIN) dx -= EDGE_SCROLL_SPEED;
        if (this.mouseY < EDGE_SCROLL_MARGIN) dy += EDGE_SCROLL_SPEED;
        if (this.mouseY > h - EDGE_SCROLL_MARGIN) dy -= EDGE_SCROLL_SPEED;

        if (dx !== 0 || dy !== 0) {
            this.worldContainer.x += dx;
            this.worldContainer.y += dy;
        }
    }

    destroy(): void {
        this.canvas.removeEventListener('pointerdown', this.onPointerDown);
        this.canvas.removeEventListener('pointermove', this.onPointerMove);
        this.canvas.removeEventListener('pointerup', this.onPointerUp);
        this.canvas.removeEventListener('pointerleave', this.onPointerUp);
        this.canvas.removeEventListener('wheel', this.onWheel);
        if (this.keyDownHandler) {
            window.removeEventListener('keydown', this.keyDownHandler);
        }
        if (this.keyUpHandler) {
            window.removeEventListener('keyup', this.keyUpHandler);
        }
        if (this.mouseMoveHandler) {
            window.removeEventListener('mousemove', this.mouseMoveHandler);
        }
    }

    private onPointerDown = (e: PointerEvent): void => {
        // Middle mouse button (button 1) for panning
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

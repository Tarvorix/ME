import { Container } from 'pixi.js';
import { TILE_WIDTH, TILE_HEIGHT, MAP_WIDTH, MAP_HEIGHT } from '../config';
import { tileToScreen } from './IsoUtils';

const DEFAULT_MIN_ZOOM = 0.25;
const DEFAULT_MAX_ZOOM = 2.0;
const ZOOM_SPEED = 0.1;
const PAN_SPEED = 8; // pixels per frame for WASD/arrow key panning
const EDGE_SCROLL_MARGIN = 20; // pixels from screen edge to trigger edge scrolling
const EDGE_SCROLL_SPEED = 6; // pixels per frame for edge scrolling
const DEFAULT_FIT_PADDING = 0.9;

interface CameraControllerOptions {
    mapWidth?: number;
    mapHeight?: number;
    worldWidth?: number;
    worldHeight?: number;
    worldOriginX?: number;
    worldOriginY?: number;
    rotation?: number;
    baseScaleX?: number;
    baseScaleY?: number;
    initialZoom?: number;
    minZoom?: number;
    maxZoom?: number;
    fitPadding?: number;
}

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
    private readonly mapWidth: number;
    private readonly mapHeight: number;
    private readonly worldWidth: number | null;
    private readonly worldHeight: number | null;
    private readonly worldOriginX: number;
    private readonly worldOriginY: number;
    private readonly rotation: number;
    private readonly baseScaleX: number;
    private readonly baseScaleY: number;
    private readonly minZoom: number;
    private readonly maxZoom: number;
    private readonly fitPadding: number;

    constructor(
        private worldContainer: Container,
        private canvas: HTMLCanvasElement,
        options: CameraControllerOptions = {},
    ) {
        this.mapWidth = options.mapWidth ?? MAP_WIDTH;
        this.mapHeight = options.mapHeight ?? MAP_HEIGHT;
        this.worldWidth = options.worldWidth ?? null;
        this.worldHeight = options.worldHeight ?? null;
        this.worldOriginX = options.worldOriginX ?? 0;
        this.worldOriginY = options.worldOriginY ?? 0;
        this.rotation = options.rotation ?? 0;
        this.baseScaleX = options.baseScaleX ?? 1;
        this.baseScaleY = options.baseScaleY ?? 1;
        this.zoom = options.initialZoom ?? 1.0;
        this.minZoom = options.minZoom ?? DEFAULT_MIN_ZOOM;
        this.maxZoom = options.maxZoom ?? DEFAULT_MAX_ZOOM;
        this.fitPadding = options.fitPadding ?? DEFAULT_FIT_PADDING;
        this.worldContainer.rotation = this.rotation;
        this.applyScale();

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
     * Computes the full map bounds from the 4 map corners after the battle-view rotation,
     * then calculates a zoom level that fits the diamond within the screen
     * (with padding), clamped to the configured zoom range.
     */
    centerOnMap(screenWidth: number, screenHeight: number): void {
        if (this.worldWidth !== null && this.worldHeight !== null) {
            const fitZoom = Math.min(
                (screenWidth * this.fitPadding) / this.worldWidth,
                (screenHeight * this.fitPadding) / this.worldHeight,
            );
            this.zoom = Math.max(this.minZoom, Math.min(this.maxZoom, fitZoom));
            this.applyScale();
            this.worldContainer.x = screenWidth / 2 - (this.worldOriginX + this.worldWidth / 2) * this.zoom;
            this.worldContainer.y = screenHeight / 2 - (this.worldOriginY + this.worldHeight / 2) * this.zoom;
            return;
        }

        // Compute isometric screen positions of the 4 map corners
        const topLeft = tileToScreen(0, 0);
        const topRight = tileToScreen(this.mapWidth, 0);
        const bottomLeft = tileToScreen(0, this.mapHeight);
        const bottomRight = tileToScreen(this.mapWidth, this.mapHeight);

        const corners = [topLeft, topRight, bottomLeft, bottomRight].map((point) => (
            this.projectPoint(point.sx, point.sy)
        ));

        // Find bounding box of the rotated map footprint
        const minX = Math.min(...corners.map((corner) => corner.x));
        const maxX = Math.max(...corners.map((corner) => corner.x));
        const minY = Math.min(...corners.map((corner) => corner.y));
        const maxY = Math.max(...corners.map((corner) => corner.y));

        const mapW = maxX - minX;
        const mapH = maxY - minY;

        // Center of the rotated footprint in screen space at zoom=1
        const centerX = (minX + maxX) / 2;
        const centerY = (minY + maxY) / 2;

        // Compute zoom to fit the rotated map footprint within the viewport
        const fitZoom = Math.min(
            (screenWidth * this.fitPadding) / mapW,
            (screenHeight * this.fitPadding) / mapH,
        );
        this.zoom = Math.max(this.minZoom, Math.min(this.maxZoom, fitZoom));
        this.applyScale();

        // Position so the rotated map center is at screen center
        this.worldContainer.x = screenWidth / 2 - centerX * this.zoom;
        this.worldContainer.y = screenHeight / 2 - centerY * this.zoom;
    }

    /** Convert client coords to world coords (accounting for camera offset, zoom, and rotation). */
    screenToWorld(screenX: number, screenY: number): { wx: number; wy: number } {
        const viewport = this.clientToViewport(screenX, screenY);
        const dx = viewport.x - this.worldContainer.x;
        const dy = viewport.y - this.worldContainer.y;
        const cos = Math.cos(this.rotation);
        const sin = Math.sin(this.rotation);
        const unrotatedX = dx * cos + dy * sin;
        const unrotatedY = -dx * sin + dy * cos;

        return {
            wx: unrotatedX / (this.baseScaleX * this.zoom),
            wy: unrotatedY / (this.baseScaleY * this.zoom),
        };
    }

    /** Pan the camera by the given screen-space delta. */
    pan(dx: number, dy: number): void {
        this.worldContainer.x += dx;
        this.worldContainer.y += dy;
    }

    /** Zoom the camera by a delta amount, centered on a screen point. */
    zoomAt(delta: number, screenX: number, screenY: number): void {
        const viewport = this.clientToViewport(screenX, screenY);
        const oldZoom = this.zoom;
        this.zoom = Math.max(this.minZoom, Math.min(this.maxZoom, this.zoom + delta));

        // Adjust position so the zoom is centered on the focal point
        const ratio = this.zoom / oldZoom;
        this.worldContainer.x = viewport.x - (viewport.x - this.worldContainer.x) * ratio;
        this.worldContainer.y = viewport.y - (viewport.y - this.worldContainer.y) * ratio;
        this.applyScale();
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
        const rect = this.canvas.getBoundingClientRect();
        const localMouseX = this.mouseX - rect.left;
        const localMouseY = this.mouseY - rect.top;
        if (localMouseX >= 0 && localMouseX <= rect.width && localMouseY >= 0 && localMouseY <= rect.height) {
            if (localMouseX < EDGE_SCROLL_MARGIN) dx += EDGE_SCROLL_SPEED;
            if (localMouseX > rect.width - EDGE_SCROLL_MARGIN) dx -= EDGE_SCROLL_SPEED;
            if (localMouseY < EDGE_SCROLL_MARGIN) dy += EDGE_SCROLL_SPEED;
            if (localMouseY > rect.height - EDGE_SCROLL_MARGIN) dy -= EDGE_SCROLL_SPEED;
        }

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

    private clientToViewport(clientX: number, clientY: number): { x: number; y: number } {
        const rect = this.canvas.getBoundingClientRect();
        return {
            x: clientX - rect.left,
            y: clientY - rect.top,
        };
    }

    private rotatePoint(x: number, y: number): { x: number; y: number } {
        const cos = Math.cos(this.rotation);
        const sin = Math.sin(this.rotation);
        return {
            x: x * cos - y * sin,
            y: x * sin + y * cos,
        };
    }

    private projectPoint(x: number, y: number): { x: number; y: number } {
        return this.rotatePoint(x * this.baseScaleX, y * this.baseScaleY);
    }

    private applyScale(): void {
        this.worldContainer.scale.set(
            this.baseScaleX * this.zoom,
            this.baseScaleY * this.zoom,
        );
    }
}

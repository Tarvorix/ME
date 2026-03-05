import { Container, Sprite, Texture, CanvasSource } from 'pixi.js';
import { tileToScreen } from './IsoUtils';
import { TILE_WIDTH, TILE_HEIGHT } from '../config';
import type { GameBridge } from '../bridge/GameBridge';

/** Fog visibility states matching Rust FOG_* constants. */
const FOG_UNEXPLORED = 0;
const FOG_EXPLORED = 1;
const FOG_VISIBLE = 2;

/**
 * Renders per-tile fog of war overlay as isometric diamond sprites.
 * Unexplored = fully black diamond, Explored = semi-transparent black, Visible = transparent (hidden).
 * Only updates tiles that changed state since last frame for performance.
 */
export class FogRenderer {
    readonly container = new Container();
    private sprites: Sprite[] = [];
    private prevStates: Uint8Array | null = null;
    private fogFullTex!: Texture; // Fully black diamond (unexplored)
    private fogHalfTex!: Texture; // 50% black diamond (explored)
    private mapWidth = 0;
    private mapHeight = 0;

    build(bridge: GameBridge): void {
        this.mapWidth = bridge.getMapWidth();
        this.mapHeight = bridge.getMapHeight();

        // Create fog diamond textures
        this.fogFullTex = this.createFogDiamond(1.0); // Fully opaque black
        this.fogHalfTex = this.createFogDiamond(0.5); // 50% opacity black

        const total = this.mapWidth * this.mapHeight;
        this.prevStates = new Uint8Array(total);
        // Initialize all as unexplored (0) which matches initial state
        this.prevStates.fill(FOG_UNEXPLORED);

        // Create one sprite per tile, all start as fully fogged (unexplored)
        for (let y = 0; y < this.mapHeight; y++) {
            for (let x = 0; x < this.mapWidth; x++) {
                const sprite = new Sprite(this.fogFullTex);
                const { sx, sy } = tileToScreen(x, y);
                sprite.anchor.set(0.5, 0);
                sprite.x = sx;
                sprite.y = sy;
                sprite.visible = true; // Starts visible (unexplored = black)
                this.sprites.push(sprite);
                this.container.addChild(sprite);
            }
        }
    }

    /**
     * Update fog overlay from the WASM fog buffer.
     * Only modifies sprites whose state changed for performance.
     */
    update(bridge: GameBridge, localPlayer: number): void {
        const fogBuf = bridge.getFogBuffer(localPlayer);
        const total = this.mapWidth * this.mapHeight;

        for (let i = 0; i < total; i++) {
            const newState = fogBuf[i];
            const oldState = this.prevStates![i];

            if (newState !== oldState) {
                this.prevStates![i] = newState;
                const sprite = this.sprites[i];

                switch (newState) {
                    case FOG_UNEXPLORED:
                        sprite.texture = this.fogFullTex;
                        sprite.visible = true;
                        break;
                    case FOG_EXPLORED:
                        sprite.texture = this.fogHalfTex;
                        sprite.visible = true;
                        break;
                    case FOG_VISIBLE:
                        sprite.visible = false; // Fully clear, no overlay
                        break;
                }
            }
        }
    }

    /**
     * Create an isometric diamond texture filled with black at the given opacity.
     */
    private createFogDiamond(alpha: number): Texture {
        const canvas = document.createElement('canvas');
        canvas.width = TILE_WIDTH;
        canvas.height = TILE_HEIGHT;
        const ctx = canvas.getContext('2d')!;

        // Diamond path
        ctx.beginPath();
        ctx.moveTo(TILE_WIDTH / 2, 0);
        ctx.lineTo(TILE_WIDTH, TILE_HEIGHT / 2);
        ctx.lineTo(TILE_WIDTH / 2, TILE_HEIGHT);
        ctx.lineTo(0, TILE_HEIGHT / 2);
        ctx.closePath();

        ctx.fillStyle = `rgba(0, 0, 0, ${alpha})`;
        ctx.fill();

        const source = new CanvasSource({ resource: canvas });
        return new Texture({ source });
    }
}

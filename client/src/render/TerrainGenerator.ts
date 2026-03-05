import { Texture, CanvasSource } from 'pixi.js';
import { TILE_WIDTH, TILE_HEIGHT } from '../config';

/** Ground tile variants (5 textures, randomly assigned by sprite_variant). */
const GROUND_FILES = [
    'Terrain/ME_Sand.png',
    'Terrain/ME_Sand2.png',
    'Terrain/ME_50-50.png',
    'Terrain/ME_50-50_2.png',
    'Terrain/ME_concrete.png',
];

/** Edge tiles for impassable/border terrain (randomly picked per tile). */
const EDGE_FILES = [
    'Terrain/ME_end.png',
    'Terrain/ME_end_concrete.png',
];

/**
 * Loads square terrain PNGs and creates isometric diamond textures (64x32)
 * by clipping through a diamond-shaped path on an offscreen canvas.
 */
export class TerrainGenerator {
    private groundTextures: Texture[] = [];
    private edgeTextures: Texture[] = [];

    async load(): Promise<void> {
        const [groundImages, edgeImages] = await Promise.all([
            Promise.all(GROUND_FILES.map((src) => this.loadImage(src))),
            Promise.all(EDGE_FILES.map((src) => this.loadImage(src))),
        ]);

        for (const img of groundImages) {
            this.groundTextures.push(this.createDiamondTexture(img));
        }
        for (const img of edgeImages) {
            this.edgeTextures.push(this.createDiamondTexture(img));
        }

        console.log(`Terrain loaded: ${this.groundTextures.length} ground, ${this.edgeTextures.length} edge`);
    }

    /** Get a ground texture by sprite variant index. */
    getTexture(variant: number): Texture {
        return this.groundTextures[variant % this.groundTextures.length];
    }

    /** Get an edge/impassable texture by variant index. */
    getEdgeTexture(variant: number): Texture {
        return this.edgeTextures[variant % this.edgeTextures.length];
    }

    private loadImage(src: string): Promise<HTMLImageElement> {
        return new Promise((resolve, reject) => {
            const img = new Image();
            img.onload = () => resolve(img);
            img.onerror = () => reject(new Error(`Failed to load terrain: ${src}`));
            img.src = src;
        });
    }

    private createDiamondTexture(img: HTMLImageElement): Texture {
        const canvas = document.createElement('canvas');
        canvas.width = TILE_WIDTH;
        canvas.height = TILE_HEIGHT;
        const ctx = canvas.getContext('2d')!;

        // Diamond clip path
        ctx.beginPath();
        ctx.moveTo(TILE_WIDTH / 2, 0);
        ctx.lineTo(TILE_WIDTH, TILE_HEIGHT / 2);
        ctx.lineTo(TILE_WIDTH / 2, TILE_HEIGHT);
        ctx.lineTo(0, TILE_HEIGHT / 2);
        ctx.closePath();
        ctx.clip();

        // Draw source tile scaled to fill the diamond area
        ctx.drawImage(img, 0, 0, TILE_WIDTH, TILE_HEIGHT);

        const source = new CanvasSource({ resource: canvas });
        return new Texture({ source });
    }
}

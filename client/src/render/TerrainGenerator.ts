import { Texture, CanvasSource } from 'pixi.js';
import { TILE_WIDTH } from '../config';

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

/** Loads square terrain PNGs and scales them to orthogonal 64x64 battle tiles. */
export class TerrainGenerator {
    private groundTextures: Texture[] = [];
    private edgeTextures: Texture[] = [];

    async load(): Promise<void> {
        const [groundImages, edgeImages] = await Promise.all([
            Promise.all(GROUND_FILES.map((src) => this.loadImage(src))),
            Promise.all(EDGE_FILES.map((src) => this.loadImage(src))),
        ]);

        for (const img of groundImages) {
            this.groundTextures.push(this.createSquareTexture(img));
        }
        for (const img of edgeImages) {
            this.edgeTextures.push(this.createSquareTexture(img));
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

    private createSquareTexture(img: HTMLImageElement): Texture {
        const canvas = document.createElement('canvas');
        canvas.width = TILE_WIDTH;
        canvas.height = TILE_WIDTH;
        const ctx = canvas.getContext('2d')!;

        // Draw source tile scaled into a square orthogonal battle cell.
        ctx.drawImage(img, 0, 0, TILE_WIDTH, TILE_WIDTH);

        const source = new CanvasSource({ resource: canvas });
        return new Texture({ source });
    }
}

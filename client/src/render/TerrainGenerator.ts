import { Texture, CanvasSource } from 'pixi.js';
import { TILE_WIDTH, TILE_HEIGHT } from '../config';

const TERRAIN_FILES = [
    'Terrain/rts_1.png',
    'Terrain/rts_2.png',
    'Terrain/rts_3.png',
    'Terrain/rts_4.png',
];

/**
 * Loads square terrain PNGs and creates isometric diamond textures (64x32)
 * by clipping through a diamond-shaped path on an offscreen canvas.
 */
export class TerrainGenerator {
    private diamondTextures: Texture[] = [];

    async load(): Promise<void> {
        const images = await Promise.all(
            TERRAIN_FILES.map((src) => this.loadImage(src)),
        );

        for (const img of images) {
            const tex = this.createDiamondTexture(img);
            this.diamondTextures.push(tex);
        }

        console.log(`Terrain loaded: ${this.diamondTextures.length} variants`);
    }

    getTexture(variant: number): Texture {
        return this.diamondTextures[variant % this.diamondTextures.length];
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

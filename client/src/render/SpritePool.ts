import { Container, Sprite, Assets, Spritesheet, Texture } from 'pixi.js';
import { BufferReader } from '../bridge/BufferReader';
import { ANIM_NAMES, UNIT_NAMES } from '../bridge/types';
import type { RenderEntry } from '../bridge/types';
import { tileToScreen } from './IsoUtils';
import { DEBUG } from '../config';

/**
 * Maps screen-space Direction enum values to atlas frame direction names.
 * The atlas has the horizontal axis mirrored: atlas "E" frames face screen-left,
 * atlas "W" frames face screen-right. N and S are correct.
 * Enum order: S=0, SW=1, W=2, NW=3, N=4, NE=5, E=6, SE=7
 * Mirror:     S,    SE,   E,   NE,   N,   NW,   W,   SW
 */
const ATLAS_DIR_NAMES = ['S', 'SE', 'E', 'NE', 'N', 'NW', 'W', 'SW'];

/** Atlas JSON files to preload, keyed by unit_anim. */
const ATLAS_ENTRIES = [
    { unit: 'Thrall', anim: 'Idle', json: 'atlases/Thrall_Idle.json' },
    { unit: 'Thrall', anim: 'Move', json: 'atlases/Thrall_Move.json' },
    { unit: 'Thrall', anim: 'Shoot', json: 'atlases/Thrall_Shoot.json' },
    { unit: 'Thrall', anim: 'Death', json: 'atlases/Thrall_Death.json' },
    { unit: 'Sentinel', anim: 'Idle', json: 'atlases/Sentinel_Idle.json' },
    { unit: 'Sentinel', anim: 'Move', json: 'atlases/Sentinel_Move.json' },
    { unit: 'Sentinel', anim: 'Shoot', json: 'atlases/Sentinel_Shoot.json' },
    { unit: 'Sentinel', anim: 'Death', json: 'atlases/Sentinel_Death.json' },
    { unit: 'hover_tank', anim: 'Idle', json: 'atlases/hover_tank.json' },
    { unit: 'command_post', anim: 'Idle', json: 'atlases/command_post.json' },
    { unit: 'forge', anim: 'Idle', json: 'atlases/forge.json' },
];

/**
 * Manages PixiJS Sprites for entities, synced from the WASM render buffer each frame.
 */
export class SpritePool {
    readonly container = new Container();
    private sprites = new Map<number, Sprite>();
    private spritesheets = new Map<string, Spritesheet>();
    private entityPositions = new Map<number, { tileX: number; tileY: number }>();

    async loadAtlases(): Promise<void> {
        for (const entry of ATLAS_ENTRIES) {
            try {
                const result = await Assets.load(entry.json);
                const key = `${entry.unit}_${entry.anim}`;

                // Assets.load for a spritesheet JSON returns the Spritesheet
                if (result instanceof Spritesheet) {
                    this.spritesheets.set(key, result);
                    const frameCount = Object.keys(result.textures).length;
                    console.log(`Atlas loaded: ${key} (${frameCount} frames)`);
                } else {
                    // v8 might return the spritesheet differently
                    console.warn(`Atlas ${key}: unexpected type`, typeof result, result);
                    // Try to use it anyway if it has textures
                    if (result && result.textures) {
                        this.spritesheets.set(key, result as Spritesheet);
                        console.log(`Atlas ${key}: using as spritesheet (${Object.keys(result.textures).length} frames)`);
                    }
                }
            } catch (err) {
                console.error(`Failed to load atlas: ${entry.json}`, err);
            }
        }
        console.log(`SpritePool: ${this.spritesheets.size} spritesheets loaded`);
    }

    /** Sync sprites to current render buffer data. */
    sync(view: DataView, count: number): void {
        const seen = new Set<number>();

        for (let i = 0; i < count; i++) {
            const entry = BufferReader.readRenderEntry(view, i);
            seen.add(entry.entityId);

            let sprite = this.sprites.get(entry.entityId);
            if (!sprite) {
                sprite = new Sprite();
                sprite.anchor.set(0.5, 0.75);
                this.container.addChild(sprite);
                this.sprites.set(entry.entityId, sprite);
            }

            // Store tile position for event processing (muzzle flash targeting)
            this.entityPositions.set(entry.entityId, { tileX: entry.x, tileY: entry.y });

            this.updateSprite(sprite, entry);
        }

        // Remove sprites for entities no longer in the buffer
        for (const [id, sprite] of this.sprites) {
            if (!seen.has(id)) {
                this.container.removeChild(sprite);
                sprite.destroy();
                this.sprites.delete(id);
                this.entityPositions.delete(id);
            }
        }

        this.container.sortableChildren = true;
    }

    /** Get the sprite for a given entity ID, if it exists. */
    getSprite(entityId: number): Sprite | undefined {
        return this.sprites.get(entityId);
    }

    /** Get the tile position of an entity from the last render buffer sync. */
    getEntityScreenPosition(entityId: number): { tileX: number; tileY: number } | null {
        return this.entityPositions.get(entityId) ?? null;
    }

    getEntityAtScreen(worldX: number, worldY: number): number | null {
        let closest: number | null = null;
        let closestDist = 24;

        for (const [id, sprite] of this.sprites) {
            const dx = worldX - sprite.x;
            const dy = worldY - sprite.y;
            const dist = Math.sqrt(dx * dx + dy * dy);
            if (dist < closestDist) {
                closestDist = dist;
                closest = id;
            }
        }

        return closest;
    }

    /** Get all entity IDs whose sprites fall within the given world-space rectangle. */
    getEntitiesInRect(minX: number, minY: number, maxX: number, maxY: number): number[] {
        const result: number[] = [];
        for (const [id, sprite] of this.sprites) {
            if (sprite.x >= minX && sprite.x <= maxX &&
                sprite.y >= minY && sprite.y <= maxY) {
                result.push(id);
            }
        }
        return result;
    }

    private updateSprite(sprite: Sprite, entry: RenderEntry): void {
        const { sx, sy } = tileToScreen(entry.x, entry.y);
        sprite.x = sx;
        sprite.y = sy;

        const animIdx = (entry.flags >> 2) & 0x03;

        const tex = this.getFrameTexture(entry.spriteId, animIdx, entry.facing, entry.frame);
        if (tex) {
            sprite.texture = tex;
        }

        sprite.scale.set(entry.scale);
        sprite.zIndex = entry.zOrder;
    }

    private getFrameTexture(
        spriteId: number,
        animIdx: number,
        facing: number,
        frame: number,
    ): Texture | null {
        const unitName = UNIT_NAMES[spriteId];
        const animName = ANIM_NAMES[animIdx] ?? 'Idle';
        const dirName = ATLAS_DIR_NAMES[facing] ?? 'S';

        if (!unitName) return null;

        const sheetKey = `${unitName}_${animName}`;
        let sheet = this.spritesheets.get(sheetKey);

        if (!sheet) {
            sheet = this.spritesheets.get(`${unitName}_Idle`);
        }
        if (!sheet) return null;

        const frameKey = `${unitName}_${animName}_${dirName}_${String(frame).padStart(4, '0')}`;
        const tex = sheet.textures[frameKey];

        if (!tex) {
            // Fallback 1: frame 0 of this animation+direction
            const fallbackKey = `${unitName}_${animName}_${dirName}_0000`;
            const fb1 = sheet.textures[fallbackKey];
            if (fb1) return fb1;

            // Fallback 2: static atlas naming (hover_tank, command_post, node use "{unit}_{dir}" only)
            const staticKey = `${unitName}_${dirName}`;
            return sheet.textures[staticKey] ?? null;
        }

        return tex;
    }
}

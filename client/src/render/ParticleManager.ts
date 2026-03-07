import { Container, Graphics } from 'pixi.js';
import { tileToBattleWorld } from './BattleViewProjection';

interface Particle {
    gfx: Graphics;
    lifetime: number;
    maxLifetime: number;
    type: 'muzzleFlash' | 'death' | 'spawn';
    startX: number;
    startY: number;
    endX: number;
    endY: number;
}

const MAX_PARTICLES = 128;

/**
 * Manages temporary visual effects: muzzle flashes, death effects, spawn effects.
 * Uses a pool of Graphics objects for performance.
 */
export class ParticleManager {
    readonly container = new Container();
    private particles: Particle[] = [];
    private pool: Graphics[] = [];

    constructor() {
        // Pre-allocate pool
        for (let i = 0; i < MAX_PARTICLES; i++) {
            const gfx = new Graphics();
            gfx.visible = false;
            this.container.addChild(gfx);
            this.pool.push(gfx);
        }
    }

    private acquire(): Graphics | null {
        const gfx = this.pool.pop();
        if (!gfx) return null;
        gfx.visible = true;
        return gfx;
    }

    private release(gfx: Graphics): void {
        gfx.visible = false;
        gfx.clear();
        gfx.alpha = 1;
        gfx.scale.set(1, 1);
        this.pool.push(gfx);
    }

    /**
     * Muzzle flash: yellow line from attacker to target, fades over 200ms.
     */
    spawnMuzzleFlash(tileX: number, tileY: number, targetTileX: number, targetTileY: number): void {
        const gfx = this.acquire();
        if (!gfx) return;

        const from = tileToBattleWorld(tileX, tileY);
        const to = tileToBattleWorld(targetTileX, targetTileY);

        gfx.clear();
        gfx.moveTo(from.x, from.y);
        gfx.lineTo(to.x, to.y);
        gfx.stroke({ width: 2, color: 0xFFFF00, alpha: 0.9 });

        this.particles.push({
            gfx,
            lifetime: 0,
            maxLifetime: 0.2,
            type: 'muzzleFlash',
            startX: from.x,
            startY: from.y,
            endX: to.x,
            endY: to.y,
        });
    }

    /**
     * Death effect: expanding red ring at position, 400ms duration.
     */
    spawnDeathEffect(tileX: number, tileY: number): void {
        const gfx = this.acquire();
        if (!gfx) return;

        const pos = tileToBattleWorld(tileX, tileY);

        gfx.clear();
        gfx.circle(0, 0, 8);
        gfx.stroke({ width: 2, color: 0xFF3333, alpha: 0.8 });
        gfx.x = pos.x;
        gfx.y = pos.y;

        this.particles.push({
            gfx,
            lifetime: 0,
            maxLifetime: 0.4,
            type: 'death',
            startX: pos.x,
            startY: pos.y,
            endX: 0,
            endY: 0,
        });
    }

    /**
     * Spawn effect: shrinking blue circle at position, 300ms duration.
     */
    spawnSpawnEffect(tileX: number, tileY: number): void {
        const gfx = this.acquire();
        if (!gfx) return;

        const pos = tileToBattleWorld(tileX, tileY);

        gfx.clear();
        gfx.circle(0, 0, 16);
        gfx.stroke({ width: 2, color: 0x4488FF, alpha: 0.8 });
        gfx.x = pos.x;
        gfx.y = pos.y;

        this.particles.push({
            gfx,
            lifetime: 0,
            maxLifetime: 0.3,
            type: 'spawn',
            startX: pos.x,
            startY: pos.y,
            endX: 0,
            endY: 0,
        });
    }

    /**
     * Update all active particles. Call each frame with dt in seconds.
     */
    update(dt: number): void {
        for (let i = this.particles.length - 1; i >= 0; i--) {
            const p = this.particles[i];
            p.lifetime += dt;

            const t = p.lifetime / p.maxLifetime;

            if (t >= 1.0) {
                // Expired — release back to pool
                this.release(p.gfx);
                this.particles.splice(i, 1);
                continue;
            }

            switch (p.type) {
                case 'muzzleFlash':
                    // Fade out
                    p.gfx.alpha = 1.0 - t;
                    break;

                case 'death':
                    // Expand and fade
                    p.gfx.scale.set(1.0 + t * 2.0);
                    p.gfx.alpha = 1.0 - t;
                    break;

                case 'spawn':
                    // Shrink and fade
                    p.gfx.scale.set(1.0 - t * 0.7);
                    p.gfx.alpha = 1.0 - t * 0.5;
                    break;
            }
        }
    }
}

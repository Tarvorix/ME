import { Application, Container } from 'pixi.js';
import { TerrainGenerator } from './TerrainGenerator';
import { TilemapRenderer } from './TilemapRenderer';
import { FogRenderer } from './FogRenderer';
import { ParticleManager } from './ParticleManager';
import { SpritePool } from './SpritePool';
import { CameraController } from './CameraController';
import { InputManager } from '../input/InputManager';
import { BufferReader } from '../bridge/BufferReader';
import { HUD } from '../ui/HUD';
import { HealthBars } from '../ui/HealthBars';
import type { GameBridge } from '../bridge/GameBridge';
import { SIM_TICK_MS, RENDER_ENTRY_SIZE } from '../config';
import { EventType } from '../bridge/types';
import { tileToScreen } from './IsoUtils';
import { SoundManager } from '../audio/SoundManager';

/**
 * Orchestrates the full render pipeline:
 * terrain tilemap, entity sprites, camera, input, fixed-timestep simulation, HUD.
 *
 * Can be initialized with an existing PixiJS Application (for campaign battle viewer)
 * or will create its own Application (for standalone mode).
 */
export class GameRenderer {
    private app!: Application;
    private worldContainer!: Container;
    private terrainGen!: TerrainGenerator;
    private tilemapRenderer!: TilemapRenderer;
    private fogRenderer!: FogRenderer;
    private particleManager!: ParticleManager;
    private healthBars!: HealthBars;
    private spritePool!: SpritePool;
    private camera!: CameraController;
    private input!: InputManager;
    private hud!: HUD;
    private bridge!: GameBridge;
    private soundManager: SoundManager;
    private simAccumulator = 0;
    private ownsApp = false;

    constructor() {
        this.soundManager = new SoundManager();
    }

    /**
     * Initialize the renderer.
     * @param bridge Game bridge (or CampaignBattleAdapter cast as GameBridge).
     * @param existingApp Optional existing PixiJS Application (for battle viewer mode).
     *                    If not provided, creates its own Application.
     */
    async init(bridge: GameBridge, existingApp?: Application): Promise<void> {
        this.bridge = bridge;

        // Initialize sound manager
        this.soundManager.init();

        if (existingApp) {
            // Reuse existing Application (battle viewer mode)
            this.app = existingApp;
            this.ownsApp = false;
        } else {
            // Create PixiJS application (standalone mode)
            this.app = new Application();
            await this.app.init({
                background: '#0a0a1a',
                resizeTo: window,
                antialias: false,
                resolution: window.devicePixelRatio || 1,
                autoDensity: true,
            });
            document.body.appendChild(this.app.canvas);
            this.ownsApp = true;
            console.log(`PixiJS canvas: ${this.app.canvas.width}x${this.app.canvas.height}`);
        }

        // World container holds everything that moves with the camera
        this.worldContainer = new Container();
        this.app.stage.addChild(this.worldContainer);

        // Load terrain textures and build tilemap
        this.terrainGen = new TerrainGenerator();
        await this.terrainGen.load();

        this.tilemapRenderer = new TilemapRenderer();
        await this.tilemapRenderer.build(bridge, this.terrainGen);
        this.worldContainer.addChild(this.tilemapRenderer.container);
        console.log(`Tilemap: ${this.tilemapRenderer.container.children.length} tiles`);

        // Load sprite atlases and create sprite pool
        this.spritePool = new SpritePool();
        await this.spritePool.loadAtlases();
        this.worldContainer.addChild(this.spritePool.container);

        // Health bars (above sprites)
        this.healthBars = new HealthBars();
        this.worldContainer.addChild(this.healthBars.container);

        // Particle effects (above sprites)
        this.particleManager = new ParticleManager();
        this.worldContainer.addChild(this.particleManager.container);

        // Fog of war overlay (above everything so fog hides enemies visually)
        this.fogRenderer = new FogRenderer();
        this.fogRenderer.build(bridge);
        this.worldContainer.addChild(this.fogRenderer.container);

        // Camera
        this.camera = new CameraController(this.worldContainer, this.app.canvas);
        this.camera.centerOnMap(this.app.screen.width, this.app.screen.height);
        console.log(`Camera offset: ${this.worldContainer.x}, ${this.worldContainer.y}`);

        // Input
        this.input = new InputManager(this.app.canvas, this.camera, this.spritePool, bridge);

        // HUD (Preact overlay)
        this.hud = new HUD(bridge);

        // Start render loop
        this.app.ticker.add(this.onFrame, this);
    }

    /**
     * Clean up all renderer resources.
     * Removes ticker callback, destroys containers and event listeners.
     * If this renderer owns the Application, the Application is also destroyed.
     */
    destroy(): void {
        // Remove ticker callback
        this.app.ticker.remove(this.onFrame, this);

        // Destroy input (removes event listeners)
        this.input.destroy();

        // Destroy camera (removes event listeners)
        this.camera.destroy();

        // Clear HUD
        this.hud.destroy();

        // Destroy world container and all children
        if (this.worldContainer.parent) {
            this.worldContainer.parent.removeChild(this.worldContainer);
        }
        this.worldContainer.destroy({ children: true });

        // Only destroy the Application if we created it
        if (this.ownsApp) {
            this.app.destroy(true, { children: true });
        }
    }

    /** Get the PixiJS Application (for sharing with battle viewer). */
    getApp(): Application {
        return this.app;
    }

    private onFrame = (): void => {
        const dt = this.app.ticker.deltaMS;
        this.simAccumulator += dt;

        // Fixed timestep simulation ticks
        while (this.simAccumulator >= SIM_TICK_MS) {
            this.bridge.tick(SIM_TICK_MS);
            this.simAccumulator -= SIM_TICK_MS;
        }

        // Sync sprites to render buffer
        const count = this.bridge.getRenderCount();
        if (count > 0) {
            const view = this.bridge.getRenderBuffer();
            this.spritePool.sync(view, count);

            // Sync health bars from render buffer data
            this.syncHealthBars(view, count);
        }

        // Process events and spawn visual effects
        this.processEvents();

        // Update particle effects
        this.particleManager.update(dt / 1000);

        // Update fog of war overlay (local player = 0 for now)
        this.fogRenderer.update(this.bridge, 0);

        // Update input visuals
        this.input.update();

        // Update HUD
        this.hud.update(this.input.getSelectedEntities());
    };

    private syncHealthBars(view: DataView, count: number): void {
        const entries: Array<{ x: number; y: number; healthPct: number; scale: number }> = [];

        for (let i = 0; i < count; i++) {
            const off = i * RENDER_ENTRY_SIZE;
            const tileX = view.getFloat32(off + 4, true);
            const tileY = view.getFloat32(off + 8, true);
            const healthPct = view.getUint8(off + 16);
            const scale = view.getFloat32(off + 20, true);

            if (healthPct < 100) {
                const { sx, sy } = tileToScreen(tileX, tileY);
                entries.push({ x: sx, y: sy, healthPct, scale });
            }
        }

        this.healthBars.sync(entries);
    }

    private processEvents(): void {
        const eventCount = this.bridge.getEventCount();
        if (eventCount === 0) return;

        const eventView = this.bridge.getEventBuffer();

        for (let i = 0; i < eventCount; i++) {
            const event = BufferReader.readEvent(eventView, i);

            switch (event.eventType) {
                case EventType.Shot: {
                    const targetId = event.payload.getUint32(0, true);
                    const targetPos = this.spritePool.getEntityScreenPosition(targetId);
                    if (targetPos) {
                        this.particleManager.spawnMuzzleFlash(
                            event.x, event.y,
                            targetPos.tileX, targetPos.tileY
                        );
                    }
                    this.soundManager.playShot(event.x, event.y);
                    break;
                }
                case EventType.Death:
                    this.particleManager.spawnDeathEffect(event.x, event.y);
                    this.soundManager.playDeath(event.x, event.y);
                    break;

                case EventType.UnitSpawned:
                    this.particleManager.spawnSpawnEffect(event.x, event.y);
                    this.soundManager.playSpawn(event.x, event.y);
                    break;

                case EventType.ProductionComplete:
                    this.soundManager.playProductionComplete();
                    break;

                case EventType.CaptureComplete:
                    this.soundManager.playCaptureComplete(event.x, event.y);
                    break;

                case EventType.BattleEnd:
                    this.soundManager.playBattleEnd();
                    break;
            }
        }
    }
}

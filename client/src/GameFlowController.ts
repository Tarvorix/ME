import { Application } from 'pixi.js';
import { GameBridge } from './bridge/GameBridge';
import type { CampaignBridge } from './bridge/CampaignBridge';
import { CampaignBattleAdapter } from './bridge/CampaignBattleAdapter';
import { CampaignRenderer } from './render/CampaignRenderer';
import { MinimapRenderer } from './render/MinimapRenderer';
import { GameRenderer } from './render/GameRenderer';
import { CampaignHUD } from './ui/CampaignHUD';
import { CampaignInputManager } from './input/CampaignInputManager';
import { CampaignAiDifficulty, BattleStatus } from './bridge/CampaignTypes';
import type { MatchStats } from './ui/VictoryScreen';
import { CAMPAIGN_TICK_MS } from './config';

type GameMode = 'campaign' | 'battle';

export interface GameEndCallbacks {
    onPlayAgain: () => void;
    onMainMenu: () => void;
}

/**
 * Master game flow controller that manages the two-layer game:
 *  - Campaign mode: strategic map, site management, dispatch, research, production
 *  - Battle mode: real-time tactical combat (viewed within campaign context)
 *
 * The controller owns the PixiJS Application and drives the game loop.
 * It transitions between campaign and battle views, keeping the campaign
 * simulation running at all times (even during battles).
 *
 * Boot flow: init() → creates Application, initializes campaign, starts game loop.
 * Battle flow: enterBattle(index) → hides campaign, shows RTS → returnToCampaign().
 */
export class GameFlowController {
    private app!: Application;
    private gameBridge!: GameBridge;
    private campaignBridge!: CampaignBridge;

    // Campaign mode resources
    private campaignRenderer!: CampaignRenderer;
    private campaignHUD!: CampaignHUD;
    private campaignInput!: CampaignInputManager;
    private minimap!: MinimapRenderer;

    // Battle mode resources (created/destroyed on transitions)
    private battleRenderer: GameRenderer | null = null;
    private currentBattleSiteId = -1;

    // State
    private mode: GameMode = 'campaign';
    private simAccumulator = 0;
    private gameOver = false;

    // Match statistics tracked during play
    private statsBattlesWon = 0;
    private statsBattlesLost = 0;
    private statsUnitsProduced = 0;
    private prevBattleSiteIds: Set<number> = new Set();

    // End-of-game callbacks
    private endCallbacks: GameEndCallbacks | null = null;

    // Campaign options (stored for Play Again)
    private campaignPlayerCount = 2;
    private campaignDifficulty = CampaignAiDifficulty.Normal;

    /**
     * Initialize the full game.
     * Creates the Application, initializes WASM, starts the campaign, and begins the game loop.
     *
     * @param playerCount Number of players (default 2).
     * @param aiDifficulty AI difficulty for non-human players.
     * @param seed Random seed (0 = random).
     */
    async init(
        playerCount = 2,
        aiDifficulty: CampaignAiDifficulty = CampaignAiDifficulty.Normal,
        seed = 0,
    ): Promise<void> {
        // Generate seed if not provided
        if (seed === 0) {
            seed = Math.floor(Math.random() * 0xFFFFFFFF);
        }

        // ── Create PixiJS Application ───────────────────────────────────
        this.app = new Application();
        await this.app.init({
            background: '#0a0a1a',
            resizeTo: window,
            antialias: false,
            resolution: window.devicePixelRatio || 1,
            autoDensity: true,
        });
        const canvasArea = document.getElementById('canvas-area');
        (canvasArea ?? document.body).appendChild(this.app.canvas);

        // ── Initialize WASM ─────────────────────────────────────────────
        this.gameBridge = new GameBridge();
        await this.gameBridge.init();
        this.campaignBridge = this.gameBridge.getCampaignBridge();

        // ── Start Campaign ──────────────────────────────────────────────
        this.campaignBridge.initCampaign(playerCount, seed);
        console.log(`Campaign started: ${playerCount} players, seed=${seed}`);

        // Add AI for all non-human players (player 0 = human)
        for (let i = 1; i < playerCount; i++) {
            this.campaignBridge.addAiPlayer(i, aiDifficulty);
        }
        console.log(`AI added for players 1-${playerCount - 1}, difficulty=${aiDifficulty}`);

        // ── Store campaign options for Play Again ─────────────────────
        this.campaignPlayerCount = playerCount;
        this.campaignDifficulty = aiDifficulty;

        // ── Campaign Renderer ───────────────────────────────────────────
        this.campaignRenderer = new CampaignRenderer();
        await this.campaignRenderer.init(this.app, this.campaignBridge);

        // ── Campaign HUD ────────────────────────────────────────────────
        this.campaignHUD = new CampaignHUD(this.campaignBridge, this.campaignRenderer);

        // Wire battle viewing callback
        this.campaignHUD.setViewBattleCallback((siteId: number) => {
            this.enterBattleAtSite(siteId);
        });

        // ── Campaign Input ──────────────────────────────────────────────
        this.campaignInput = new CampaignInputManager(
            this.app.canvas,
            this.campaignRenderer,
            this.campaignHUD,
        );

        // ── Minimap ─────────────────────────────────────────────────────
        this.minimap = new MinimapRenderer(
            this.campaignBridge,
            this.campaignRenderer,
            this.campaignRenderer.getCamera(),
            this.app.canvas,
        );
        this.app.stage.addChild(this.minimap.container);

        // ── Start Game Loop ─────────────────────────────────────────────
        this.app.ticker.add(this.onFrame, this);

        console.log('GameFlowController initialized — campaign mode active');
    }

    // ── Game Loop ───────────────────────────────────────────────────────

    private onFrame = (): void => {
        const dt = this.app.ticker.deltaMS;

        // Don't tick simulation if game is over
        if (!this.gameOver) {
            // Always run campaign simulation ticks (drives economy, research, dispatch, battles, AI)
            this.simAccumulator += dt;
            while (this.simAccumulator >= CAMPAIGN_TICK_MS) {
                this.campaignBridge.tick();
                this.simAccumulator -= CAMPAIGN_TICK_MS;
            }

            // Track battle outcomes for match statistics
            this.trackBattleStats();
        }

        if (this.mode === 'campaign') {
            // Update campaign visuals and UI
            this.campaignRenderer.update(dt);
            this.campaignHUD.update();
            this.campaignInput.update();

            // Update minimap
            this.minimap.update(this.app.screen.width, this.app.screen.height);

            // Check for victory/defeat
            if (!this.gameOver) {
                this.checkGameEnd();
            }
        } else if (this.mode === 'battle') {
            // Battle renderer handles its own updates via the app ticker
            // We just check if the battle has ended
            this.checkBattleEnd();
        }
    };

    // ── Battle Transitions ──────────────────────────────────────────────

    /**
     * Transition to battle view for a specific active battle.
     * @param battleIndex Index into the active battles array.
     */
    async enterBattle(battleIndex: number): Promise<void> {
        if (this.mode === 'battle') return;

        const battles = this.campaignBridge.getActiveBattles();
        if (battleIndex < 0 || battleIndex >= battles.length) {
            console.warn(`Invalid battle index: ${battleIndex}`);
            return;
        }

        console.log(`Entering battle view: battle index ${battleIndex}`);
        this.mode = 'battle';
        this.currentBattleSiteId = battles[battleIndex].siteId;

        // Hide campaign view and minimap, disable campaign camera
        this.campaignRenderer.hide();
        this.campaignRenderer.disableCamera();
        this.minimap.hide();
        this.campaignInput.destroy();

        // Create battle adapter and renderer
        const adapter = new CampaignBattleAdapter(
            this.campaignBridge,
            this.currentBattleSiteId,
            this.gameBridge.getMemory(),
        );

        this.battleRenderer = new GameRenderer();
        await this.battleRenderer.init(adapter as unknown as GameBridge, this.app);
    }

    /**
     * Return from battle view to campaign map.
     */
    returnToCampaign(): void {
        if (this.mode === 'campaign') return;

        console.log('Returning to campaign view');

        // Destroy battle renderer
        if (this.battleRenderer) {
            this.battleRenderer.destroy();
            this.battleRenderer = null;
        }

        this.currentBattleSiteId = -1;
        this.mode = 'campaign';

        // Show campaign view and minimap, re-enable campaign camera
        this.campaignRenderer.show();
        this.campaignRenderer.enableCamera();
        this.minimap.show();

        // Re-create campaign input (was destroyed when entering battle)
        this.campaignInput = new CampaignInputManager(
            this.app.canvas,
            this.campaignRenderer,
            this.campaignHUD,
        );
    }

    /**
     * Enter battle view for a specific site (finds the battle by site ID).
     * Called by CampaignHUD's "View Battle" button.
     */
    async enterBattleAtSite(siteId: number): Promise<void> {
        const battles = this.campaignBridge.getActiveBattles();
        const index = battles.findIndex(b => b.siteId === siteId);
        if (index >= 0) {
            await this.enterBattle(index);
        }
    }

    // ── State Checks ────────────────────────────────────────────────────

    /**
     * Check if the current battle has ended and auto-return to campaign.
     */
    private checkBattleEnd(): void {
        if (this.currentBattleSiteId < 0) return;

        const battles = this.campaignBridge.getActiveBattles();
        const battle = battles.find((activeBattle) => activeBattle.siteId === this.currentBattleSiteId);
        if (!battle) {
            // Battle no longer in the active list — it ended
            this.returnToCampaign();
            return;
        }

        if (battle.status === BattleStatus.Finished) {
            // Battle finished — wait a moment then return
            // For now, return immediately; Chunk 51 will add transition polish
            this.returnToCampaign();
        }
    }

    /**
     * Track battle outcomes for match statistics.
     * Detects when battles end and determines win/loss for player 0.
     */
    private trackBattleStats(): void {
        const battles = this.campaignBridge.getActiveBattles();
        const currentBattleSiteIds = new Set(battles.map(b => b.siteId));

        // Check for battles that ended (were in prev set but not in current)
        for (const siteId of this.prevBattleSiteIds) {
            if (!currentBattleSiteIds.has(siteId)) {
                // Battle ended - check the site owner to determine win/loss
                const sites = this.campaignBridge.getSites();
                const site = sites.find(s => s.siteId === siteId);
                if (site) {
                    if (site.owner === 0) {
                        this.statsBattlesWon++;
                    } else {
                        this.statsBattlesLost++;
                    }
                }
            }
        }

        this.prevBattleSiteIds = currentBattleSiteIds;
    }

    /**
     * Check for game end conditions (player eliminated or victory).
     */
    private checkGameEnd(): void {
        const eliminated = this.campaignBridge.getEliminatedPlayers();

        let victory = false;
        let defeat = false;

        // Check if human player (player 0) is eliminated
        if (eliminated.includes(0)) {
            defeat = true;
        }

        // Check if all enemies are eliminated (victory)
        const playerCount = this.campaignBridge.getPlayerCount();
        const enemyCount = playerCount - 1;
        const eliminatedEnemies = eliminated.filter(p => p !== 0).length;
        if (eliminatedEnemies >= enemyCount && enemyCount > 0) {
            victory = true;
        }

        if (victory || defeat) {
            this.gameOver = true;

            // Pause the campaign
            this.campaignBridge.setPaused(true);

            // Build match statistics
            const sites = this.campaignBridge.getSites();
            const research = this.campaignBridge.getResearch(0);
            const stats: MatchStats = {
                tickCount: this.campaignBridge.getTickCount(),
                sitesControlled: sites.filter(s => s.owner === 0).length,
                totalSites: sites.length,
                battlesWon: this.statsBattlesWon,
                battlesLost: this.statsBattlesLost,
                unitsProduced: this.statsUnitsProduced,
                researchCompleted: research.completedCount,
                playerCount: playerCount,
            };

            console.log(`Game Over: ${victory ? 'VICTORY' : 'DEFEAT'}`);

            // Show victory/defeat screen via HUD
            this.campaignHUD.setGameEnd(victory, stats);
            this.campaignHUD.setGameEndCallbacks({
                onPlayAgain: () => {
                    if (this.endCallbacks) this.endCallbacks.onPlayAgain();
                },
                onMainMenu: () => {
                    if (this.endCallbacks) this.endCallbacks.onMainMenu();
                },
            });
        }
    }

    // ── Public API ──────────────────────────────────────────────────────

    /** Get current game mode. */
    getMode(): GameMode { return this.mode; }

    /** Get the PixiJS Application. */
    getApp(): Application { return this.app; }

    /** Get the campaign bridge. */
    getCampaignBridge(): CampaignBridge { return this.campaignBridge; }

    /** Get the campaign renderer. */
    getCampaignRenderer(): CampaignRenderer { return this.campaignRenderer; }

    /** Get the campaign HUD. */
    getCampaignHUD(): CampaignHUD { return this.campaignHUD; }

    /** Set callbacks for when the game ends (play again, main menu). */
    setEndCallbacks(callbacks: GameEndCallbacks): void {
        this.endCallbacks = callbacks;
    }

    /** Increment the units produced counter (called by HUD on successful production). */
    incrementUnitsProduced(count: number = 1): void {
        this.statsUnitsProduced += count;
    }

    /** Check if the game has ended. */
    isGameOver(): boolean {
        return this.gameOver;
    }

    /** Get stored campaign player count (for Play Again). */
    getPlayerCount(): number { return this.campaignPlayerCount; }

    /** Get stored campaign difficulty (for Play Again). */
    getDifficulty(): CampaignAiDifficulty { return this.campaignDifficulty; }

    /** Destroy everything. */
    destroy(): void {
        this.app.ticker.remove(this.onFrame, this);

        if (this.battleRenderer) {
            this.battleRenderer.destroy();
        }

        this.campaignInput.destroy();
        this.campaignHUD.destroy();
        this.minimap.destroy();
        this.campaignRenderer.destroy();
        this.app.destroy(true, { children: true });
    }
}

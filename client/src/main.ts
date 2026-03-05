import { GameBridge } from './bridge/GameBridge';
import { NetBridge } from './network/NetBridge';
import { GameRenderer } from './render/GameRenderer';
import { GameFlowController } from './GameFlowController';
import { LobbyScreen } from './ui/LobbyScreen';
import { MainMenu } from './ui/MainMenu';
import type { MainMenuOptions } from './ui/MainMenu';
import { CampaignAiDifficulty } from './bridge/CampaignTypes';

/**
 * Machine Empire boot flow.
 *
 * Default (no params): Show Main Menu → New Campaign or Online.
 * ?mode=online:        Online lobby mode — connect via WebSocket, play multiplayer.
 * ?server=<url>:       Direct server mode — connect to a specific server.
 * ?players=N&difficulty=X&seed=N: Skip menu, start campaign directly with given options.
 *
 * The campaign is THE game. RTS battles happen within the campaign flow.
 * There is no standalone RTS mode — campaign and battles are one integrated game.
 */
async function boot() {
    const params = new URLSearchParams(window.location.search);
    const serverUrl = params.get('server');
    const mode = params.get('mode');

    if (mode === 'online') {
        // ── Online Lobby Mode ───────────────────────────────────────────
        const httpBase = params.get('api') || 'http://localhost:8082';
        const wsBase = params.get('ws') || 'ws://localhost:8080';

        const lobby = new LobbyScreen(httpBase, wsBase, {
            onStartGame: async (_lobbyId: string, wsUrl: string) => {
                lobby.hide();

                const netBridge = new NetBridge(wsUrl);
                netBridge.connect();

                const renderer = new GameRenderer();
                await renderer.init(netBridge as unknown as GameBridge);
                console.log('Online match started via lobby');
            },
        });
        lobby.show();
        console.log('Lobby mode: showing lobby screen');

    } else if (serverUrl) {
        // ── Direct Server Mode ──────────────────────────────────────────
        const netBridge = new NetBridge(serverUrl);
        netBridge.connect();

        const renderer = new GameRenderer();
        await renderer.init(netBridge as unknown as GameBridge);
        console.log('Online mode: connected to', serverUrl);

    } else if (params.has('players') || params.has('difficulty') || params.has('seed')) {
        // ── Direct Campaign Launch (URL params) ──────────────────────────
        const playerCount = parseInt(params.get('players') || '2', 10);
        const difficultyParam = params.get('difficulty') || 'normal';
        const seedParam = params.get('seed');
        const seed = seedParam ? parseInt(seedParam, 10) : 0;

        const difficultyMap: Record<string, CampaignAiDifficulty> = {
            easy: CampaignAiDifficulty.Easy,
            normal: CampaignAiDifficulty.Normal,
            hard: CampaignAiDifficulty.Hard,
        };
        const difficulty = difficultyMap[difficultyParam.toLowerCase()] ?? CampaignAiDifficulty.Normal;

        await startCampaign({ playerCount, difficulty, seed });

    } else {
        // ── Main Menu (Default) ──────────────────────────────────────────
        showMainMenu();
    }
}

/**
 * Show the main menu screen.
 * Called on boot (default) and when returning from a finished game.
 */
function showMainMenu(): void {
    const menu = new MainMenu({
        onNewCampaign: async (options: MainMenuOptions) => {
            menu.hide();
            await startCampaign(options);
        },
        onOnline: () => {
            menu.hide();
            // Navigate to online lobby mode
            window.location.search = '?mode=online';
        },
    });
    menu.show();
    console.log('Main menu displayed');
}

/**
 * Start a campaign game with the given options.
 * Creates a GameFlowController, sets up end-of-game callbacks for
 * Play Again (new game with same settings) and Main Menu (return to menu).
 */
async function startCampaign(options: MainMenuOptions): Promise<void> {
    const controller = new GameFlowController();
    await controller.init(options.playerCount, options.difficulty, options.seed);

    // Set up end-of-game callbacks
    controller.setEndCallbacks({
        onPlayAgain: () => {
            controller.destroy();
            // Start a new game with same player count and difficulty, new random seed
            startCampaign({
                playerCount: controller.getPlayerCount(),
                difficulty: controller.getDifficulty(),
                seed: 0, // 0 = random
            });
        },
        onMainMenu: () => {
            controller.destroy();
            showMainMenu();
        },
    });

    console.log(`Campaign game started: ${options.playerCount} players, difficulty=${options.difficulty}`);

    // Expose controller globally for debugging
    (window as any).__gameController = controller;
}

boot().catch((err) => {
    console.error('Boot failed:', err);
});

// Register service worker for PWA support
if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
        navigator.serviceWorker.register('/sw.js').then((reg) => {
            console.log('Service worker registered:', reg.scope);
        }).catch((err) => {
            console.warn('Service worker registration failed:', err);
        });
    });
}

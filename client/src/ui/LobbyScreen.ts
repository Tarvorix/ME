import { render } from 'preact';
import { html } from 'htm/preact';
import { useState, useEffect, useCallback } from 'preact/hooks';

/**
 * Lobby API types matching server http_api.rs structures.
 */
export interface LobbyInfo {
    id: string;
    name: string;
    player_count: number;
    max_players: number;
    status: string;
    players: LobbyPlayerInfo[];
}

export interface LobbyPlayerInfo {
    slot: number;
    name: string;
    is_ai: boolean;
    ready: boolean;
}

export interface CreateLobbyResponse {
    id: string;
    name: string;
}

/**
 * Callback when player transitions from lobby to game.
 */
export interface LobbyCallbacks {
    onStartGame: (lobbyId: string, wsUrl: string) => void;
}

// ── API client ───────────────────────────────────────────────────────────────

class LobbyApi {
    private baseUrl: string;

    constructor(baseUrl: string) {
        this.baseUrl = baseUrl;
    }

    async listLobbies(): Promise<LobbyInfo[]> {
        const res = await fetch(`${this.baseUrl}/lobbies`);
        if (!res.ok) throw new Error(`List lobbies failed: ${res.status}`);
        return res.json();
    }

    async createLobby(name: string, maxPlayers?: number): Promise<CreateLobbyResponse> {
        const body: Record<string, unknown> = { name };
        if (maxPlayers !== undefined) body.max_players = maxPlayers;
        const res = await fetch(`${this.baseUrl}/lobbies`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body),
        });
        if (!res.ok) throw new Error(`Create lobby failed: ${res.status}`);
        return res.json();
    }

    async joinLobby(lobbyId: string, playerName: string): Promise<LobbyInfo> {
        const res = await fetch(`${this.baseUrl}/lobbies/${lobbyId}/join`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ player_name: playerName }),
        });
        if (!res.ok) {
            if (res.status === 404) throw new Error('Lobby not found');
            if (res.status === 409) throw new Error('Lobby is full');
            throw new Error(`Join lobby failed: ${res.status}`);
        }
        return res.json();
    }

    async addAi(lobbyId: string, difficulty?: string): Promise<LobbyInfo> {
        const res = await fetch(`${this.baseUrl}/lobbies/${lobbyId}/ai`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ difficulty: difficulty ?? 'Normal' }),
        });
        if (!res.ok) {
            if (res.status === 404) throw new Error('Lobby not found');
            if (res.status === 409) throw new Error('Lobby is full');
            throw new Error(`Add AI failed: ${res.status}`);
        }
        return res.json();
    }

    async readyUp(lobbyId: string): Promise<LobbyInfo> {
        const res = await fetch(`${this.baseUrl}/lobbies/${lobbyId}/ready`, {
            method: 'POST',
        });
        if (!res.ok) {
            if (res.status === 404) throw new Error('Lobby not found');
            if (res.status === 400) throw new Error('Already ready or not joined');
            throw new Error(`Ready up failed: ${res.status}`);
        }
        return res.json();
    }
}

// ── Styles ───────────────────────────────────────────────────────────────────

const LOBBY_STYLES = {
    overlay: `
        position: fixed;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        background: radial-gradient(ellipse at center, rgba(10,10,20,0.95) 0%, rgba(5,5,15,1.0) 100%);
        font-family: 'Segoe UI', Tahoma, sans-serif;
        color: #d0d0d0;
        z-index: 200;
    `,
    container: `
        width: 520px;
        max-height: 80vh;
        overflow-y: auto;
        background: rgba(20,20,35,0.95);
        border: 1px solid rgba(80,80,120,0.4);
        border-radius: 8px;
        padding: 24px;
        box-shadow: 0 8px 32px rgba(0,0,0,0.6);
    `,
    title: `
        font-size: 22px;
        font-weight: 700;
        color: #e8e8f0;
        margin-bottom: 20px;
        text-align: center;
        text-transform: uppercase;
        letter-spacing: 2px;
    `,
    subtitle: `
        font-size: 14px;
        color: #888;
        text-transform: uppercase;
        letter-spacing: 1px;
        margin-bottom: 12px;
    `,
    input: `
        width: 100%;
        padding: 8px 12px;
        background: rgba(30,30,50,0.9);
        border: 1px solid rgba(80,80,120,0.4);
        border-radius: 4px;
        color: #e0e0e0;
        font-size: 14px;
        outline: none;
        margin-bottom: 10px;
        box-sizing: border-box;
    `,
    button: `
        display: inline-flex;
        align-items: center;
        justify-content: center;
        padding: 8px 16px;
        background: rgba(50,50,80,0.7);
        border: 1px solid rgba(80,80,120,0.5);
        border-radius: 4px;
        color: #d0d0e0;
        font-size: 13px;
        cursor: pointer;
        transition: background 0.15s;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    `,
    buttonPrimary: `
        display: inline-flex;
        align-items: center;
        justify-content: center;
        padding: 10px 20px;
        background: rgba(40,80,160,0.7);
        border: 1px solid rgba(60,100,200,0.5);
        border-radius: 4px;
        color: #e0e8ff;
        font-size: 14px;
        font-weight: 600;
        cursor: pointer;
        transition: background 0.15s;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    `,
    buttonDanger: `
        display: inline-flex;
        align-items: center;
        justify-content: center;
        padding: 8px 16px;
        background: rgba(120,40,40,0.7);
        border: 1px solid rgba(160,60,60,0.5);
        border-radius: 4px;
        color: #e8c0c0;
        font-size: 13px;
        cursor: pointer;
        transition: background 0.15s;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    `,
    lobbyCard: `
        padding: 12px;
        margin: 6px 0;
        background: rgba(30,30,50,0.6);
        border: 1px solid rgba(70,70,100,0.3);
        border-radius: 6px;
        cursor: pointer;
        transition: background 0.15s, border-color 0.15s;
    `,
    lobbyCardHover: `
        background: rgba(40,40,65,0.8);
        border-color: rgba(80,80,140,0.5);
    `,
    lobbyName: `
        font-size: 15px;
        font-weight: 600;
        color: #e0e0e8;
    `,
    lobbyMeta: `
        font-size: 12px;
        color: #888;
        margin-top: 4px;
    `,
    playerRow: `
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 6px 8px;
        margin: 3px 0;
        background: rgba(30,30,50,0.5);
        border-radius: 4px;
    `,
    playerName: `
        color: #d0d0e0;
        font-size: 13px;
    `,
    playerAi: `
        color: #88aacc;
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    `,
    readyBadge: `
        display: inline-block;
        padding: 2px 8px;
        font-size: 10px;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        border-radius: 3px;
    `,
    error: `
        color: #e88;
        font-size: 12px;
        margin-top: 6px;
        padding: 6px;
        background: rgba(120,40,40,0.2);
        border-radius: 3px;
    `,
    row: `
        display: flex;
        gap: 8px;
        margin-bottom: 10px;
    `,
    spacer: `
        height: 16px;
    `,
    divider: `
        border: none;
        border-top: 1px solid rgba(60,60,90,0.3);
        margin: 16px 0;
    `,
};

// ── Preact Components ────────────────────────────────────────────────────────

type Screen = 'list' | 'create' | 'lobby';

interface LobbyScreenState {
    screen: Screen;
    lobbies: LobbyInfo[];
    currentLobby: LobbyInfo | null;
    currentLobbyId: string;
    playerName: string;
    createName: string;
    createMaxPlayers: number;
    error: string;
    loading: boolean;
    pollTimer: ReturnType<typeof setInterval> | null;
}

function LobbyApp({ api, callbacks, wsBaseUrl }: {
    api: LobbyApi;
    callbacks: LobbyCallbacks;
    wsBaseUrl: string;
}) {
    const [screen, setScreen] = useState<Screen>('list');
    const [lobbies, setLobbies] = useState<LobbyInfo[]>([]);
    const [currentLobby, setCurrentLobby] = useState<LobbyInfo | null>(null);
    const [currentLobbyId, setCurrentLobbyId] = useState('');
    const [playerName, setPlayerName] = useState('Player');
    const [createName, setCreateName] = useState('');
    const [createMaxPlayers, setCreateMaxPlayers] = useState(2);
    const [error, setError] = useState('');
    const [loading, setLoading] = useState(false);

    // Fetch lobbies on list screen
    const refreshLobbies = useCallback(async () => {
        try {
            const list = await api.listLobbies();
            setLobbies(list);
        } catch (err) {
            setError(`Failed to fetch lobbies: ${err}`);
        }
    }, [api]);

    // Poll current lobby for updates
    const refreshCurrentLobby = useCallback(async () => {
        if (!currentLobbyId) return;
        try {
            const list = await api.listLobbies();
            const found = list.find(l => l.id === currentLobbyId);
            if (found) {
                setCurrentLobby(found);
                // Check if all players are ready and we have enough players
                const allReady = found.players.length >= 2 && found.players.every(p => p.ready);
                if (allReady) {
                    const wsUrl = `${wsBaseUrl}?match=${found.id}`;
                    callbacks.onStartGame(found.id, wsUrl);
                }
            }
        } catch {
            // Ignore poll errors
        }
    }, [api, currentLobbyId, wsBaseUrl, callbacks]);

    useEffect(() => {
        if (screen === 'list') {
            refreshLobbies();
            const timer = setInterval(refreshLobbies, 3000);
            return () => clearInterval(timer);
        }
        if (screen === 'lobby' && currentLobbyId) {
            const timer = setInterval(refreshCurrentLobby, 2000);
            return () => clearInterval(timer);
        }
    }, [screen, currentLobbyId, refreshLobbies, refreshCurrentLobby]);

    // ── Handlers ─────────────────────────────────────────────────────────────

    const handleCreate = async () => {
        if (!createName.trim()) {
            setError('Enter a lobby name');
            return;
        }
        setLoading(true);
        setError('');
        try {
            const result = await api.createLobby(createName.trim(), createMaxPlayers);
            // Auto-join the created lobby
            const lobby = await api.joinLobby(result.id, playerName);
            setCurrentLobby(lobby);
            setCurrentLobbyId(result.id);
            setScreen('lobby');
        } catch (err) {
            setError(`${err}`);
        } finally {
            setLoading(false);
        }
    };

    const handleJoin = async (lobbyId: string) => {
        setLoading(true);
        setError('');
        try {
            const lobby = await api.joinLobby(lobbyId, playerName);
            setCurrentLobby(lobby);
            setCurrentLobbyId(lobbyId);
            setScreen('lobby');
        } catch (err) {
            setError(`${err}`);
        } finally {
            setLoading(false);
        }
    };

    const handleAddAi = async () => {
        if (!currentLobbyId) return;
        setLoading(true);
        setError('');
        try {
            const lobby = await api.addAi(currentLobbyId);
            setCurrentLobby(lobby);
        } catch (err) {
            setError(`${err}`);
        } finally {
            setLoading(false);
        }
    };

    const handleReady = async () => {
        if (!currentLobbyId) return;
        setLoading(true);
        setError('');
        try {
            const lobby = await api.readyUp(currentLobbyId);
            setCurrentLobby(lobby);
        } catch (err) {
            setError(`${err}`);
        } finally {
            setLoading(false);
        }
    };

    const handleBack = () => {
        setScreen('list');
        setCurrentLobby(null);
        setCurrentLobbyId('');
        setError('');
    };

    // ── Render ───────────────────────────────────────────────────────────────

    if (screen === 'create') {
        return html`
            <div style=${LOBBY_STYLES.overlay}>
                <div style=${LOBBY_STYLES.container}>
                    <div style=${LOBBY_STYLES.title}>Create Lobby</div>

                    <div style=${LOBBY_STYLES.subtitle}>Your Name</div>
                    <input
                        style=${LOBBY_STYLES.input}
                        value=${playerName}
                        onInput=${(e: Event) => setPlayerName((e.target as HTMLInputElement).value)}
                        placeholder="Your name"
                    />

                    <div style=${LOBBY_STYLES.subtitle}>Lobby Name</div>
                    <input
                        style=${LOBBY_STYLES.input}
                        value=${createName}
                        onInput=${(e: Event) => setCreateName((e.target as HTMLInputElement).value)}
                        placeholder="My Battle Arena"
                    />

                    <div style=${LOBBY_STYLES.subtitle}>Max Players</div>
                    <div style=${LOBBY_STYLES.row}>
                        ${[2, 3, 4].map(n => html`
                            <button
                                style=${n === createMaxPlayers
                                    ? LOBBY_STYLES.buttonPrimary
                                    : LOBBY_STYLES.button}
                                onClick=${() => setCreateMaxPlayers(n)}
                            >${n} Players</button>
                        `)}
                    </div>

                    <div style=${LOBBY_STYLES.spacer}></div>

                    <div style=${LOBBY_STYLES.row}>
                        <button style=${LOBBY_STYLES.button} onClick=${handleBack}>Back</button>
                        <button
                            style=${LOBBY_STYLES.buttonPrimary}
                            onClick=${handleCreate}
                            disabled=${loading}
                        >${loading ? 'Creating...' : 'Create & Join'}</button>
                    </div>

                    ${error ? html`<div style=${LOBBY_STYLES.error}>${error}</div>` : null}
                </div>
            </div>
        `;
    }

    if (screen === 'lobby' && currentLobby) {
        const allReady = currentLobby.players.length >= 2 && currentLobby.players.every(p => p.ready);
        const emptySlots = currentLobby.max_players - currentLobby.player_count;

        return html`
            <div style=${LOBBY_STYLES.overlay}>
                <div style=${LOBBY_STYLES.container}>
                    <div style=${LOBBY_STYLES.title}>${currentLobby.name}</div>
                    <div style=${LOBBY_STYLES.subtitle}>
                        ${currentLobby.player_count} / ${currentLobby.max_players} Players
                    </div>

                    ${currentLobby.players.map(p => html`
                        <div style=${LOBBY_STYLES.playerRow}>
                            <div>
                                <span style=${LOBBY_STYLES.playerName}>${p.name}</span>
                                ${p.is_ai ? html`
                                    <span style=${LOBBY_STYLES.playerAi}> (AI)</span>
                                ` : null}
                            </div>
                            <span style=${`${LOBBY_STYLES.readyBadge}; ${
                                p.ready
                                    ? 'background: rgba(40,120,40,0.4); color: #8c8;'
                                    : 'background: rgba(80,80,80,0.3); color: #888;'
                            }`}>
                                ${p.ready ? 'Ready' : 'Not Ready'}
                            </span>
                        </div>
                    `)}

                    <div style=${LOBBY_STYLES.spacer}></div>

                    <div style=${LOBBY_STYLES.row}>
                        <button style=${LOBBY_STYLES.button} onClick=${handleBack}>Leave</button>
                        ${emptySlots > 0 ? html`
                            <button
                                style=${LOBBY_STYLES.button}
                                onClick=${handleAddAi}
                                disabled=${loading}
                            >Add AI</button>
                        ` : null}
                        <button
                            style=${LOBBY_STYLES.buttonPrimary}
                            onClick=${handleReady}
                            disabled=${loading}
                        >Ready Up</button>
                    </div>

                    ${allReady ? html`
                        <div style=${'text-align: center; color: #8c8; margin-top: 12px; font-size: 14px;'}>
                            All players ready — starting match...
                        </div>
                    ` : null}

                    ${error ? html`<div style=${LOBBY_STYLES.error}>${error}</div>` : null}
                </div>
            </div>
        `;
    }

    // ── List Screen (default) ────────────────────────────────────────────────

    return html`
        <div style=${LOBBY_STYLES.overlay}>
            <div style=${LOBBY_STYLES.container}>
                <div style=${LOBBY_STYLES.title}>Machine Empire</div>

                <div style=${LOBBY_STYLES.subtitle}>Your Name</div>
                <input
                    style=${LOBBY_STYLES.input}
                    value=${playerName}
                    onInput=${(e: Event) => setPlayerName((e.target as HTMLInputElement).value)}
                    placeholder="Your name"
                />

                <hr style=${LOBBY_STYLES.divider} />

                <div style=${'display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;'}>
                    <div style=${LOBBY_STYLES.subtitle}>Available Lobbies</div>
                    <div style=${LOBBY_STYLES.row}>
                        <button style=${LOBBY_STYLES.button} onClick=${refreshLobbies}>Refresh</button>
                        <button
                            style=${LOBBY_STYLES.buttonPrimary}
                            onClick=${() => { setScreen('create'); setError(''); }}
                        >New Lobby</button>
                    </div>
                </div>

                ${lobbies.length === 0 ? html`
                    <div style=${'text-align: center; color: #666; padding: 20px; font-size: 13px;'}>
                        No lobbies available. Create one to get started.
                    </div>
                ` : null}

                ${lobbies.map(lobby => html`
                    <div
                        style=${LOBBY_STYLES.lobbyCard}
                        onClick=${() => handleJoin(lobby.id)}
                        onMouseOver=${(e: Event) => {
                            const el = e.currentTarget as HTMLElement;
                            el.style.cssText = LOBBY_STYLES.lobbyCard + LOBBY_STYLES.lobbyCardHover;
                        }}
                        onMouseOut=${(e: Event) => {
                            const el = e.currentTarget as HTMLElement;
                            el.style.cssText = LOBBY_STYLES.lobbyCard;
                        }}
                    >
                        <div style=${LOBBY_STYLES.lobbyName}>${lobby.name}</div>
                        <div style=${LOBBY_STYLES.lobbyMeta}>
                            ${lobby.player_count}/${lobby.max_players} players
                            — ${lobby.status}
                        </div>
                    </div>
                `)}

                ${error ? html`<div style=${LOBBY_STYLES.error}>${error}</div>` : null}
            </div>
        </div>
    `;
}

// ── Public API ───────────────────────────────────────────────────────────────

/**
 * LobbyScreen: renders and manages the lobby UI overlay.
 * Shows lobby list, create lobby, join/ready flow.
 * Calls onStartGame when all players are ready.
 */
export class LobbyScreen {
    private root: HTMLElement;
    private api: LobbyApi;
    private callbacks: LobbyCallbacks;
    private wsBaseUrl: string;
    private visible = false;

    constructor(httpBaseUrl: string, wsBaseUrl: string, callbacks: LobbyCallbacks) {
        this.api = new LobbyApi(httpBaseUrl);
        this.wsBaseUrl = wsBaseUrl;
        this.callbacks = callbacks;

        // Create or find the lobby root element
        let root = document.getElementById('lobby-root');
        if (!root) {
            root = document.createElement('div');
            root.id = 'lobby-root';
            document.body.appendChild(root);
        }
        this.root = root;
    }

    /** Show the lobby screen. */
    show(): void {
        if (this.visible) return;
        this.visible = true;
        this.render();
    }

    /** Hide the lobby screen. */
    hide(): void {
        if (!this.visible) return;
        this.visible = false;
        render(null, this.root);
    }

    /** Whether the lobby screen is currently visible. */
    isVisible(): boolean {
        return this.visible;
    }

    private render(): void {
        render(
            html`<${LobbyApp}
                api=${this.api}
                callbacks=${this.callbacks}
                wsBaseUrl=${this.wsBaseUrl}
            />`,
            this.root,
        );
    }
}

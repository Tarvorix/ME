import { render } from 'preact';
import { html } from 'htm/preact';
import { CampaignAiDifficulty } from '../bridge/CampaignTypes';

export interface MainMenuOptions {
    playerCount: number;
    difficulty: CampaignAiDifficulty;
    seed: number; // 0 = random
}

export interface MainMenuCallbacks {
    onNewCampaign: (options: MainMenuOptions) => void;
    onOnline: () => void;
}

/**
 * Main menu screen shown on boot.
 * Provides options for New Campaign (local vs AI) and Online mode.
 * Renders into #hud-root as a full-screen overlay.
 */
export class MainMenu {
    private root: HTMLElement;
    private callbacks: MainMenuCallbacks;
    private playerCount = 2;
    private difficulty = CampaignAiDifficulty.Normal;
    private seed = 0;

    constructor(callbacks: MainMenuCallbacks) {
        this.root = document.getElementById('hud-root')!;
        this.callbacks = callbacks;
    }

    show(): void {
        this.renderMenu();
    }

    hide(): void {
        render(null, this.root);
    }

    private renderMenu(): void {
        const difficultyLabels = ['Easy', 'Normal', 'Hard'];
        const currentDiffLabel = difficultyLabels[this.difficulty] ?? 'Normal';

        render(
            html`
                <div style="
                    position: fixed; top: 0; left: 0; right: 0; bottom: 0;
                    display: flex; align-items: center; justify-content: center;
                    background: linear-gradient(180deg, #0a0a1a 0%, #0f0f2a 50%, #0a0a1a 100%);
                    z-index: 400;
                ">
                    <div style="text-align: center; width: 420px; max-width: 90vw">
                        <!-- Title -->
                        <div style="
                            font-size: 48px; font-weight: 700;
                            color: #e0e0e0;
                            letter-spacing: 6px;
                            margin-bottom: 4px;
                            text-shadow: 0 0 20px rgba(68,136,255,0.3);
                        ">MACHINE EMPIRE</div>

                        <div style="
                            font-size: 14px; color: #555;
                            letter-spacing: 3px; text-transform: uppercase;
                            margin-bottom: 48px;
                        ">A War of Iron and Code</div>

                        <!-- Menu Options -->
                        <div style="
                            padding: 24px;
                            background: rgba(15,15,25,0.8);
                            border: 1px solid rgba(100,100,120,0.3);
                            border-radius: 8px;
                            text-align: left;
                        ">
                            <!-- Campaign Settings -->
                            <div style="color: #aaa; font-size: 11px; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 12px">
                                Campaign Settings
                            </div>

                            <!-- Player Count -->
                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px">
                                <span style="color: #ccc; font-size: 13px">Players</span>
                                <div style="display: flex; gap: 6px">
                                    ${[2, 3, 4].map(count => html`
                                        <button
                                            style=${menuOptionBtn(this.playerCount === count)}
                                            onClick=${() => { this.playerCount = count; this.renderMenu(); }}
                                        >${count}</button>
                                    `)}
                                </div>
                            </div>

                            <!-- Difficulty -->
                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px">
                                <span style="color: #ccc; font-size: 13px">AI Difficulty</span>
                                <div style="display: flex; gap: 6px">
                                    ${difficultyLabels.map((label, i) => html`
                                        <button
                                            style=${menuOptionBtn(this.difficulty === i)}
                                            onClick=${() => { this.difficulty = i; this.renderMenu(); }}
                                        >${label}</button>
                                    `)}
                                </div>
                            </div>

                            <!-- Seed -->
                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px">
                                <span style="color: #ccc; font-size: 13px">Map Seed</span>
                                <div style="display: flex; align-items: center; gap: 8px">
                                    <input
                                        type="number"
                                        value=${this.seed || ''}
                                        placeholder="Random"
                                        style="
                                            width: 100px; padding: 4px 8px;
                                            background: rgba(30,30,45,0.8);
                                            border: 1px solid rgba(80,80,100,0.3);
                                            border-radius: 4px;
                                            color: #e0e0e0; font-size: 12px;
                                            outline: none;
                                        "
                                        onInput=${(e: Event) => {
                                            const val = parseInt((e.target as HTMLInputElement).value, 10);
                                            this.seed = isNaN(val) ? 0 : val;
                                        }}
                                    />
                                </div>
                            </div>

                            <hr style="border: none; border-top: 1px solid rgba(80,80,100,0.3); margin: 16px 0" />

                            <!-- Start Campaign Button -->
                            <button
                                style="
                                    width: 100%; padding: 12px;
                                    background: rgba(30,60,100,0.7);
                                    border: 1px solid rgba(68,136,255,0.5);
                                    border-radius: 6px;
                                    color: #ccdeff; font-size: 16px; font-weight: 600;
                                    cursor: pointer;
                                    letter-spacing: 1px;
                                    transition: background 0.2s;
                                "
                                onClick=${() => this.callbacks.onNewCampaign({
                                    playerCount: this.playerCount,
                                    difficulty: this.difficulty,
                                    seed: this.seed,
                                })}
                                onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(40,80,140,0.8)'}
                                onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(30,60,100,0.7)'}
                            >NEW CAMPAIGN</button>

                            <!-- Online Button -->
                            <button
                                style="
                                    width: 100%; padding: 10px; margin-top: 8px;
                                    background: rgba(40,40,55,0.6);
                                    border: 1px solid rgba(80,80,100,0.3);
                                    border-radius: 6px;
                                    color: #aaa; font-size: 14px;
                                    cursor: pointer;
                                    transition: background 0.2s;
                                "
                                onClick=${() => this.callbacks.onOnline()}
                                onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(60,60,80,0.7)'}
                                onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(40,40,55,0.6)'}
                            >ONLINE MULTIPLAYER</button>
                        </div>

                        <!-- Footer -->
                        <div style="margin-top: 24px; font-size: 11px; color: #333">
                            Machine Empire v0.5.0
                        </div>
                    </div>
                </div>
            `,
            this.root,
        );
    }
}

function menuOptionBtn(active: boolean): string {
    return `
        padding: 4px 12px;
        background: ${active ? 'rgba(40,80,140,0.7)' : 'rgba(30,30,45,0.6)'};
        border: 1px solid ${active ? 'rgba(68,136,255,0.5)' : 'rgba(60,60,80,0.3)'};
        border-radius: 4px;
        color: ${active ? '#ccdeff' : '#888'};
        font-size: 12px; font-weight: ${active ? '600' : '400'};
        cursor: pointer;
    `;
}

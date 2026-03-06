import { html } from 'htm/preact';
import { CAMPAIGN_STYLES } from './styles';

export interface MatchStats {
    tickCount: number;
    sitesControlled: number;
    totalSites: number;
    battlesWon: number;
    battlesLost: number;
    unitsProduced: number;
    researchCompleted: number;
    playerCount: number;
}

export interface VictoryScreenProps {
    victory: boolean;
    stats: MatchStats;
    onPlayAgain: () => void;
    onMainMenu: () => void;
}

/**
 * Full-screen victory or defeat overlay.
 * Shows match result, statistics, and action buttons.
 */
export function VictoryScreen({ victory, stats, onPlayAgain, onMainMenu }: VictoryScreenProps) {
    const title = victory ? 'VICTORY' : 'DEFEAT';
    const titleColor = victory ? '#44cc44' : '#cc4444';
    const subtitle = victory
        ? 'All enemy nodes have been destroyed.'
        : 'Your node has been destroyed.';

    // Format time
    const totalSecs = Math.floor(stats.tickCount * 0.05);
    const mins = Math.floor(totalSecs / 60);
    const secs = totalSecs % 60;
    const timeStr = `${mins}:${secs.toString().padStart(2, '0')}`;

    return html`
        <div style="
            position: fixed; top: 0; left: 0; right: 0; bottom: 0;
            display: flex; align-items: center; justify-content: center;
            background: rgba(0,0,0,0.8);
            z-index: 300;
        ">
            <div style="
                padding: 32px 48px; min-width: 400px; max-width: 500px;
                background: rgba(15,15,25,0.96);
                border: 2px solid ${victory ? 'rgba(68,204,68,0.4)' : 'rgba(204,68,68,0.4)'};
                border-radius: 12px; text-align: center;
            ">
                <!-- Title -->
                <div style="
                    font-size: 36px; font-weight: 700;
                    color: ${titleColor};
                    letter-spacing: 4px;
                    margin-bottom: 8px;
                ">${title}</div>

                <div style="font-size: 14px; color: #888; margin-bottom: 24px">
                    ${subtitle}
                </div>

                <!-- Stats -->
                <div style="
                    text-align: left; padding: 16px;
                    background: rgba(30,30,45,0.5);
                    border-radius: 6px; margin-bottom: 24px;
                ">
                    <div style=${CAMPAIGN_STYLES.panelTitle}>Match Statistics</div>

                    ${statRow('Time', timeStr)}
                    ${statRow('Sites Controlled', `${stats.sitesControlled} / ${stats.totalSites}`)}
                    ${statRow('Battles Won', `${stats.battlesWon}`)}
                    ${statRow('Battles Lost', `${stats.battlesLost}`)}
                    ${statRow('Units Produced', `${stats.unitsProduced}`)}
                    ${statRow('Research Completed', `${stats.researchCompleted} / 12`)}
                    ${statRow('Players', `${stats.playerCount}`)}
                </div>

                <!-- Buttons -->
                <div style="display: flex; gap: 12px; justify-content: center">
                    <button
                        style=${CAMPAIGN_STYLES.actionBtnPrimary + '; padding: 10px 24px; font-size: 14px'}
                        onClick=${onPlayAgain}
                        onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(40,80,140,0.8)'}
                        onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(30,60,100,0.7)'}
                    >Play Again</button>
                    <button
                        style=${CAMPAIGN_STYLES.actionBtn + '; padding: 10px 24px; font-size: 14px'}
                        onClick=${onMainMenu}
                        onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(50,80,110,0.7)'}
                        onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(40,60,80,0.6)'}
                    >Main Menu</button>
                </div>
            </div>
        </div>
    `;
}

function statRow(label: string, value: string) {
    return html`
        <div style="display: flex; justify-content: space-between; padding: 3px 0; font-size: 13px">
            <span style="color: #888">${label}</span>
            <span style="color: #e0e0e0; font-weight: 600">${value}</span>
        </div>
    `;
}

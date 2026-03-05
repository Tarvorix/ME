import { html } from 'htm/preact';
import { CAMPAIGN_STYLES } from './styles';
import { HUD_STYLES } from './styles';
import type { CampaignResearchData, CampaignEconomyData } from '../bridge/CampaignTypes';
import {
    TechId, TECH_NAMES, TECH_DESCRIPTIONS,
    getTechDefinition, NO_TECH,
} from '../bridge/CampaignTypes';

export interface ResearchPanelProps {
    research: CampaignResearchData;
    availableTechs: number[];
    economy: CampaignEconomyData;
    onResearch: (techId: number) => void;
    onClose: () => void;
}

/**
 * Research tree layout: 4 columns × 3 tiers.
 * Column 1: Thrall upgrades, Column 2: Sentinel, Column 3: Tank, Column 4: General.
 * Each tier requires the tier above as prerequisite.
 */
const TECH_GRID: TechId[][] = [
    // Tier 1 (top row)
    [TechId.ThrallPlating, TechId.SentinelHeavyWeapons, TechId.HoverTankReactiveArmor, TechId.ImprovedVision],
    // Tier 2 (middle row)
    [TechId.ThrallFireRate, TechId.SentinelShields, TechId.HoverTankSiege, TechId.FastProduction],
    // Tier 3 (bottom row)
    [TechId.ThrallRange, TechId.SentinelStealth, TechId.HoverTankOvercharge, TechId.EconomicEfficiency],
];

const COLUMN_LABELS = ['Thrall', 'Sentinel', 'Hover Tank', 'General'];
const COLUMN_COLORS = ['#ccaa77', '#77aacc', '#cc7777', '#cccc77'];

type TechState = 'locked' | 'available' | 'active' | 'completed';

/**
 * Full-screen overlay showing the 12-tech research tree in a grid layout.
 * Click a tech to start researching. ESC or close button dismisses.
 */
export function ResearchPanel({ research, availableTechs, economy, onResearch, onClose }: ResearchPanelProps) {
    const completedSet = new Set(research.completedTechs);
    const availableSet = new Set(availableTechs);

    function getTechState(techId: TechId): TechState {
        if (completedSet.has(techId)) return 'completed';
        if (research.activeTechId === techId) return 'active';
        if (availableSet.has(techId)) return 'available';
        return 'locked';
    }

    function getCardStyle(state: TechState): string {
        switch (state) {
            case 'completed': return CAMPAIGN_STYLES.techCard + '; ' + CAMPAIGN_STYLES.techCardCompleted;
            case 'active': return CAMPAIGN_STYLES.techCard + '; ' + CAMPAIGN_STYLES.techCardActive;
            case 'available': return CAMPAIGN_STYLES.techCard + '; ' + CAMPAIGN_STYLES.techCardAvailable;
            case 'locked': return CAMPAIGN_STYLES.techCard + '; ' + CAMPAIGN_STYLES.techCardLocked;
        }
    }

    // Active research progress
    const hasActiveResearch = research.activeTechId !== NO_TECH;
    const activePct = hasActiveResearch && research.activeTotalTime > 0
        ? Math.min(100, (research.activeProgress / research.activeTotalTime) * 100)
        : 0;

    return html`
        <div style=${CAMPAIGN_STYLES.researchOverlay} onClick=${(e: Event) => {
            if (e.target === e.currentTarget) onClose();
        }}>
            <div style=${CAMPAIGN_STYLES.researchPanel + '; position: relative'}>
                <!-- Close button -->
                <button style=${CAMPAIGN_STYLES.closeBtn} onClick=${onClose}>x</button>

                <!-- Header -->
                <div style="font-weight: 600; font-size: 16px; color: #e0e0e0; margin-bottom: 4px">
                    Research
                </div>

                <!-- Active Research Progress -->
                ${hasActiveResearch ? html`
                    <div style="margin-bottom: 12px; padding: 8px 10px; background: rgba(30,50,80,0.4); border-radius: 4px; border: 1px solid rgba(68,136,255,0.3)">
                        <div style="display: flex; justify-content: space-between; font-size: 12px; margin-bottom: 4px">
                            <span style="color: #88aadd">Researching: ${TECH_NAMES[research.activeTechId as TechId]}</span>
                            <span style="color: #88aadd">${Math.round(activePct)}%</span>
                        </div>
                        <div style=${HUD_STYLES.progressBar}>
                            <div style=${HUD_STYLES.progressFill + '; width: ' + activePct + '%'}></div>
                        </div>
                        <div style="font-size: 10px; color: #668; margin-top: 2px">
                            ${research.activeProgress.toFixed(0)}s / ${research.activeTotalTime.toFixed(0)}s
                        </div>
                    </div>
                ` : html`
                    <div style="margin-bottom: 12px; font-size: 12px; color: #666">
                        No active research. Select a tech to begin.
                    </div>
                `}

                <!-- Column Labels -->
                <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; margin-bottom: 4px">
                    ${COLUMN_LABELS.map((label, i) => html`
                        <div style=${'text-align: center; font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; color: ' + COLUMN_COLORS[i]}>
                            ${label}
                        </div>
                    `)}
                </div>

                <!-- Tech Grid (3 rows × 4 columns) -->
                ${TECH_GRID.map((row, tierIdx) => html`
                    <div style="margin-bottom: 2px">
                        <div style="font-size: 9px; color: #555; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 2px; margin-left: 2px">
                            Tier ${tierIdx + 1}
                        </div>
                        <div style=${CAMPAIGN_STYLES.researchGrid}>
                            ${row.map((techId) => {
                                const def = getTechDefinition(techId);
                                const state = getTechState(techId);
                                const canClick = state === 'available' && !hasActiveResearch;
                                const canAfford = economy.energyBank >= def.energyCost;

                                return html`
                                    <div
                                        style=${getCardStyle(state)}
                                        onClick=${() => canClick && canAfford && onResearch(techId)}
                                        onMouseOver=${(e: Event) => canClick && ((e.currentTarget as HTMLElement).style.background = 'rgba(50,70,100,0.8)')}
                                        onMouseOut=${(e: Event) => {
                                            if (state === 'available') (e.currentTarget as HTMLElement).style.background = 'rgba(40,50,70,0.7)';
                                        }}
                                    >
                                        <div style=${CAMPAIGN_STYLES.techName}>
                                            ${state === 'completed' ? '[OK] ' : ''}${def.name}
                                        </div>
                                        <div style=${CAMPAIGN_STYLES.techDesc}>${def.description}</div>
                                        ${state !== 'completed' ? html`
                                            <div style=${CAMPAIGN_STYLES.techCost}>
                                                ${def.energyCost}E | ${def.researchTime}s
                                                ${def.requiredRelics > 0 ? html` | ${def.requiredRelics} Relic${def.requiredRelics > 1 ? 's' : ''}` : null}
                                            </div>
                                            ${state === 'available' && !canAfford ? html`
                                                <div style="font-size: 9px; color: #cc4444; margin-top: 2px">Not enough energy</div>
                                            ` : null}
                                        ` : null}
                                    </div>
                                `;
                            })}
                        </div>
                    </div>
                `)}

                <!-- Completed Count -->
                <div style="margin-top: 12px; font-size: 11px; color: #666; text-align: center">
                    ${research.completedCount} / 12 technologies researched
                </div>
            </div>
        </div>
    `;
}

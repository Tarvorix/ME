import { html } from 'htm/preact';
import { CAMPAIGN_STYLES, strainColor } from './styles';
import { HUD_STYLES } from './styles';
import type { CampaignEconomyData } from '../bridge/CampaignTypes';

export interface CampaignResourceBarProps {
    economy: CampaignEconomyData;
    paused: boolean;
    tickCount: number;
    onTogglePause: () => void;
    onOpenResearch: () => void;
    onSelectNode: () => void;
}

/**
 * Top bar showing campaign economy: energy bank, income breakdown,
 * expenses, net rate, strain meter, research/production buttons, and pause control.
 * Renders as content within the top bar grid zone (no position:fixed).
 */
export function CampaignResourceBar({ economy, paused, tickCount, onTogglePause, onOpenResearch, onSelectNode }: CampaignResourceBarProps) {
    const net = economy.netRate;
    const netStr = net >= 0 ? `+${net.toFixed(1)}` : net.toFixed(1);
    const netColor = net >= 0 ? '#44cc44' : '#cc4444';
    const sc = strainColor(economy.strain);
    const strainPct = Math.min(100, Math.max(0, economy.strain));

    // Format tick count as time display (MM:SS)
    const totalSecs = Math.floor(tickCount * 0.05); // 50ms per tick
    const mins = Math.floor(totalSecs / 60);
    const secs = totalSecs % 60;
    const timeStr = `${mins}:${secs.toString().padStart(2, '0')}`;

    return html`
        <!-- Energy Bank -->
        <div style=${CAMPAIGN_STYLES.resourceGroup}>
            <span style=${HUD_STYLES.resourceLabel}>Energy</span>
            <span style=${HUD_STYLES.resourceValue}>${Math.floor(economy.energyBank)}</span>
        </div>

        <!-- Income Breakdown -->
        <div style=${CAMPAIGN_STYLES.resourceGroup}>
            <span style=${HUD_STYLES.resourceLabel}>Income</span>
            <span style=${'font-size: 11px; color: #44cc44'}>${economy.totalIncome.toFixed(1)}/s</span>
            <span style="font-size: 10px; color: #666" title="Node + Mines + Relics">
                (${economy.nodeIncome.toFixed(0)}+${economy.mineIncome.toFixed(0)}+${economy.relicIncome.toFixed(0)})
            </span>
        </div>

        <!-- Expenses Breakdown -->
        <div style=${CAMPAIGN_STYLES.resourceGroup}>
            <span style=${HUD_STYLES.resourceLabel}>Expense</span>
            <span style=${'font-size: 11px; color: #cc8844'}>${economy.totalExpenses.toFixed(1)}/s</span>
            <span style="font-size: 10px; color: #666" title="Garrison + Deployed">
                (G:${economy.garrisonUpkeep.toFixed(0)} D:${economy.deployedUpkeep.toFixed(0)})
            </span>
        </div>

        <!-- Net Rate -->
        <div style=${CAMPAIGN_STYLES.resourceGroup}>
            <span style=${HUD_STYLES.resourceLabel}>Net</span>
            <span style=${'font-weight: 600; font-size: 12px; color: ' + netColor}>${netStr}/s</span>
        </div>

        <!-- Strain Meter -->
        <div style=${HUD_STYLES.strainMeter}>
            <span style=${HUD_STYLES.resourceLabel}>Strain</span>
            <div style=${HUD_STYLES.strainBar}>
                <div style=${HUD_STYLES.strainFill + '; width: ' + strainPct + '%; background: ' + sc}></div>
            </div>
            <span style=${'font-size: 11px; color: ' + sc}>${Math.round(economy.strain)}%</span>
        </div>

        <!-- Spacer to push buttons right -->
        <div style="flex: 1"></div>

        <!-- Research Button -->
        <button
            style=${CAMPAIGN_STYLES.topBarBtn}
            onClick=${onOpenResearch}
            onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(50,70,100,0.8)'}
            onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(40,50,70,0.7)'}
            title="Open Research (R)"
        >RESEARCH</button>

        <!-- Production Button -->
        <button
            style=${CAMPAIGN_STYLES.topBarBtn}
            onClick=${onSelectNode}
            onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(50,70,100,0.8)'}
            onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(40,50,70,0.7)'}
            title="Select Node (show production)"
        >PRODUCTION</button>

        <!-- Time Display -->
        <div style="display: flex; align-items: center; gap: 6px; margin-left: 8px">
            <span style="font-size: 11px; color: #666; font-family: monospace">${timeStr}</span>
        </div>

        <!-- Pause Button -->
        <button
            style=${CAMPAIGN_STYLES.pauseBtn}
            onClick=${onTogglePause}
            onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(80,60,30,0.8)'}
            onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(60,40,20,0.6)'}
        >
            ${paused ? 'RESUME' : 'PAUSE'}
        </button>
    `;
}

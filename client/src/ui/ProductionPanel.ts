import { html } from 'htm/preact';
import { CAMPAIGN_STYLES, strainColor } from './styles';
import { HUD_STYLES } from './styles';
import type { CampaignSiteData, CampaignEconomyData } from '../bridge/CampaignTypes';
import {
    CampaignUnitType, CAMPAIGN_UNIT_NAMES, CAMPAIGN_UNIT_COSTS,
} from '../bridge/CampaignTypes';

export interface ProductionPanelProps {
    forgeSite: CampaignSiteData | null;
    economy: CampaignEconomyData;
    onProduce: (unitType: number) => void;
}

const PRODUCIBLE_UNITS = [
    {
        type: CampaignUnitType.Thrall,
        name: CAMPAIGN_UNIT_NAMES[CampaignUnitType.Thrall],
        cost: CAMPAIGN_UNIT_COSTS[CampaignUnitType.Thrall],
        desc: 'Cheap infantry, causes strain',
        color: '#ccaa77',
    },
    {
        type: CampaignUnitType.Sentinel,
        name: CAMPAIGN_UNIT_NAMES[CampaignUnitType.Sentinel],
        cost: CAMPAIGN_UNIT_COSTS[CampaignUnitType.Sentinel],
        desc: 'Elite cyborg, no strain',
        color: '#77aacc',
    },
    {
        type: CampaignUnitType.HoverTank,
        name: CAMPAIGN_UNIT_NAMES[CampaignUnitType.HoverTank],
        cost: CAMPAIGN_UNIT_COSTS[CampaignUnitType.HoverTank],
        desc: 'Heavy armor, ignores terrain',
        color: '#cc7777',
    },
];

/**
 * Bottom-right production panel for the player's forge.
 * Shows forge garrison, produce buttons with costs, and energy/strain info.
 * Campaign production is instant: deduct energy, add units to forge garrison.
 */
export function ProductionPanel({ forgeSite, economy, onProduce }: ProductionPanelProps) {
    if (!forgeSite) return null;

    const sc = strainColor(economy.strain);
    const totalGarrison = forgeSite.garrisonThralls + forgeSite.garrisonSentinels + forgeSite.garrisonTanks;

    return html`
        <div style=${'padding: 10px 14px; font-size: 12px'}>
            <div style=${CAMPAIGN_STYLES.panelTitle}>Forge Production</div>

            <!-- Forge Garrison -->
            <div style="margin-bottom: 8px">
                <div style="font-size: 11px; color: #888; margin-bottom: 4px">Forge Garrison (${totalGarrison})</div>
                <div style="display: flex; gap: 10px; font-size: 12px">
                    <span style="color: #ccaa77">${forgeSite.garrisonThralls}T</span>
                    <span style="color: #77aacc">${forgeSite.garrisonSentinels}S</span>
                    <span style="color: #cc7777">${forgeSite.garrisonTanks}H</span>
                </div>
            </div>

            <hr style=${CAMPAIGN_STYLES.divider} />

            <!-- Energy Available -->
            <div style="display: flex; justify-content: space-between; margin-bottom: 6px">
                <span style="font-size: 11px; color: #888">Available Energy</span>
                <span style="font-weight: 600; color: #e0e0e0">${Math.floor(economy.energyBank)}</span>
            </div>

            <!-- Strain -->
            <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 8px">
                <span style="font-size: 11px; color: #888">Strain</span>
                <div style=${HUD_STYLES.strainBar}>
                    <div style=${HUD_STYLES.strainFill + '; width: ' + Math.min(100, economy.strain) + '%; background: ' + sc}></div>
                </div>
                <span style=${'font-size: 10px; color: ' + sc}>${Math.round(economy.strain)}%</span>
            </div>

            <hr style=${CAMPAIGN_STYLES.divider} />

            <!-- Produce Buttons -->
            ${PRODUCIBLE_UNITS.map((unit) => {
                const canAfford = economy.energyBank >= unit.cost;
                const opacity = canAfford ? '1' : '0.5';
                const cursor = canAfford ? 'pointer' : 'default';

                return html`
                    <div
                        style=${CAMPAIGN_STYLES.produceBtn + '; opacity: ' + opacity + '; cursor: ' + cursor}
                        onClick=${() => canAfford && onProduce(unit.type)}
                        onMouseOver=${(e: Event) => canAfford && ((e.currentTarget as HTMLElement).style.background = 'rgba(60,60,80,0.8)')}
                        onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(40,40,55,0.6)'}
                    >
                        <div>
                            <span style=${'color: ' + unit.color}>${unit.name}</span>
                            <div style="font-size: 10px; color: #666">${unit.desc}</div>
                        </div>
                        <span style=${HUD_STYLES.buildCost}>${unit.cost}E</span>
                    </div>
                `;
            })}
        </div>
    `;
}

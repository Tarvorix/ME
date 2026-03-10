import { html } from 'htm/preact';
import { CAMPAIGN_STYLES, strainColor } from './styles';
import { HUD_STYLES } from './styles';
import type { CampaignSiteData, ActiveBattleData, CampaignEconomyData, CampaignProductionData } from '../bridge/CampaignTypes';
import {
    SiteType, SITE_TYPE_NAMES, NEUTRAL_OWNER, BattleStatus,
    CampaignUnitType, CAMPAIGN_UNIT_NAMES, CAMPAIGN_UNIT_COSTS, CAMPAIGN_UNIT_BUILD_TIMES,
} from '../bridge/CampaignTypes';
import { PLAYER_COLORS } from '../config';

export interface SitePanelProps {
    site: CampaignSiteData | null;
    playerNodeId: number;
    battles: ActiveBattleData[];
    economy: CampaignEconomyData;
    production: CampaignProductionData;
    onDispatchFrom: (siteId: number) => void;
    onWithdraw: (siteId: number) => void;
    onViewBattle: (siteId: number) => void;
    onProduce: (unitType: number) => void;
    onOpenResearch: () => void;
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
 * Left panel showing details for the selected campaign site.
 * When viewing the player's node, also shows production buttons.
 * Renders as content within the left panel grid zone (no position:fixed).
 */
export function SitePanel({ site, playerNodeId, battles, economy, production, onDispatchFrom, onWithdraw, onViewBattle, onProduce, onOpenResearch }: SitePanelProps) {
    if (!site) {
        return html`
            <div style=${CAMPAIGN_STYLES.leftPanelPlaceholder}>
                Select a site on the map
            </div>
        `;
    }

    const siteName = SITE_TYPE_NAMES[site.siteType as SiteType] ?? 'Unknown';
    const isPlayerOwned = site.owner === 0; // Player 0 is human
    const isNeutral = site.owner === NEUTRAL_OWNER;
    const ownerLabel = isNeutral ? 'Neutral' : isPlayerOwned ? 'You' : `Player ${site.owner + 1}`;
    const ownerColor = isNeutral ? '#888' : isPlayerOwned ? '#4488FF' : '#FF4444';

    const totalGarrison = site.garrisonThralls + site.garrisonSentinels + site.garrisonTanks;

    // Income for this site type
    const incomeStr = site.siteType === SiteType.Node ? '+5.0/s'
        : site.siteType === SiteType.MiningStation ? '+8.0/s'
        : site.siteType === SiteType.RelicSite ? '+3.0/s'
        : '';

    // Check if this site has an active battle
    const activeBattle = battles.find(b => b.siteId === site.siteId);
    const hasBattle = !!activeBattle;

    // Is this the player's node?
    const isPlayerNode = site.siteId === playerNodeId;
    const hasActiveBuild = production.activeUnitType !== 255;
    const activeBuildName = hasActiveBuild
        ? CAMPAIGN_UNIT_NAMES[production.activeUnitType as CampaignUnitType]
        : 'Idle';
    const activeBuildPct = production.activeTotalTime > 0
        ? Math.min(100, (production.activeProgress / production.activeTotalTime) * 100)
        : 0;
    const activeBuildRemaining = production.activeTotalTime > 0
        ? Math.max(0, production.activeTotalTime - production.activeProgress)
        : 0;
    const queuedParts: string[] = [];
    if (production.queuedThralls > 0) queuedParts.push(`${production.queuedThralls}T`);
    if (production.queuedSentinels > 0) queuedParts.push(`${production.queuedSentinels}S`);
    if (production.queuedTanks > 0) queuedParts.push(`${production.queuedTanks}H`);

    return html`
        <div>
            <!-- Site Header -->
            <div style=${CAMPAIGN_STYLES.sitePanelTitle}>
                ${siteName}${isPlayerNode ? ' (Home)' : ''}
                <span style="font-size: 10px; color: #666; margin-left: 6px">#${site.siteId}</span>
            </div>

            <!-- Owner -->
            <div style=${CAMPAIGN_STYLES.sitePanelRow}>
                <span style=${HUD_STYLES.resourceLabel}>Owner</span>
                <span style=${'font-weight: 600; color: ' + ownerColor}>${ownerLabel}</span>
            </div>

            <!-- Income -->
            ${site.owner === 0 && incomeStr ? html`
                <div style=${CAMPAIGN_STYLES.sitePanelRow}>
                    <span style=${HUD_STYLES.resourceLabel}>Income</span>
                    <span style="color: #44cc44; font-size: 12px">${incomeStr}</span>
                </div>
            ` : null}

            <!-- Garrison -->
            <div style="margin-top: 6px">
                <div style=${HUD_STYLES.resourceLabel}>Garrison (${totalGarrison})</div>
                ${totalGarrison > 0 ? html`
                    <div style="display: flex; gap: 12px; margin-top: 4px; font-size: 12px">
                        ${site.garrisonThralls > 0 ? html`
                            <span style="color: #ccaa77">${site.garrisonThralls} <span style="color: #888; font-size: 10px">Thrall</span></span>
                        ` : null}
                        ${site.garrisonSentinels > 0 ? html`
                            <span style="color: #77aacc">${site.garrisonSentinels} <span style="color: #888; font-size: 10px">Sentinel</span></span>
                        ` : null}
                        ${site.garrisonTanks > 0 ? html`
                            <span style="color: #cc7777">${site.garrisonTanks} <span style="color: #888; font-size: 10px">Tank</span></span>
                        ` : null}
                    </div>
                ` : html`
                    <div style="color: #666; font-size: 11px; margin-top: 2px">No garrison</div>
                `}
            </div>

            <!-- Battle Status -->
            ${hasBattle ? html`
                <div style="margin-top: 6px; padding: 6px 8px; background: rgba(120,30,30,0.3); border-radius: 4px; border: 1px solid rgba(200,60,60,0.3)">
                    <div style="color: #ff6666; font-weight: 600; font-size: 11px; text-transform: uppercase">
                        ${activeBattle.status === BattleStatus.Deployment ? 'Deploying' : activeBattle.status === BattleStatus.Active ? 'Battle Active' : 'Battle Ended'}
                    </div>
                    <div style="font-size: 11px; color: #cc8888; margin-top: 2px">
                        P${activeBattle.attacker + 1} vs P${activeBattle.defender + 1}
                        ${activeBattle.winner !== 255 ? html` | Winner: P${activeBattle.winner + 1}` : null}
                    </div>
                    <button
                        style=${CAMPAIGN_STYLES.actionBtn + '; margin-top: 6px; width: 100%; font-size: 11px'}
                        onClick=${() => onViewBattle(site.siteId)}
                    >View Battle</button>
                </div>
            ` : null}

            <!-- Contested indicator -->
            ${site.isContested && !hasBattle ? html`
                <div style="margin-top: 4px; color: #ffaa44; font-size: 11px">
                    Contested
                </div>
            ` : null}

            <!-- Actions -->
            ${isPlayerOwned && !hasBattle ? html`
                <div style=${CAMPAIGN_STYLES.sitePanelActions}>
                    ${totalGarrison > 0 ? html`
                        <button
                            style=${CAMPAIGN_STYLES.actionBtn}
                            onClick=${() => onDispatchFrom(site.siteId)}
                            onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(50,80,110,0.7)'}
                            onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(40,60,80,0.6)'}
                        >Dispatch</button>
                    ` : null}
                    ${totalGarrison > 0 && !isPlayerNode ? html`
                        <button
                            style=${CAMPAIGN_STYLES.actionBtnDanger}
                            onClick=${() => onWithdraw(site.siteId)}
                            onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(100,40,40,0.7)'}
                            onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(80,30,30,0.6)'}
                        >Withdraw</button>
                    ` : null}
                </div>
            ` : null}

            <!-- Production Section (only when viewing player's node) -->
            ${isPlayerNode ? html`
                <hr style=${CAMPAIGN_STYLES.divider + '; margin-top: 12px'} />

                <div style=${CAMPAIGN_STYLES.panelTitle}>Production</div>

                <!-- Energy Available -->
                <div style="display: flex; justify-content: space-between; margin-bottom: 4px">
                    <span style="font-size: 11px; color: #888">Energy</span>
                    <span style="font-weight: 600; color: #e0e0e0; font-size: 12px">${Math.floor(economy.energyBank)}</span>
                </div>

                <!-- Strain -->
                <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 8px">
                    <span style="font-size: 11px; color: #888">Strain</span>
                    <div style=${HUD_STYLES.strainBar}>
                        <div style=${HUD_STYLES.strainFill + '; width: ' + Math.min(100, economy.strain) + '%; background: ' + strainColor(economy.strain)}></div>
                    </div>
                    <span style=${'font-size: 10px; color: ' + strainColor(economy.strain)}>${Math.round(economy.strain)}%</span>
                </div>

                <!-- Active Build -->
                <div style="margin-bottom: 8px">
                    <div style="display: flex; justify-content: space-between; margin-bottom: 2px">
                        <span style="font-size: 11px; color: #888">Active Build</span>
                        <span style=${'font-size: 11px; color: ' + (hasActiveBuild ? '#88bbff' : '#777')}>
                            ${activeBuildName}
                        </span>
                    </div>
                    ${hasActiveBuild ? html`
                        <div style=${HUD_STYLES.progressBar}>
                            <div style=${HUD_STYLES.progressFill + '; width: ' + activeBuildPct + '%'}></div>
                        </div>
                        <div style="display: flex; justify-content: space-between; margin-top: 3px; font-size: 10px; color: #777">
                            <span>${Math.floor(activeBuildPct)}%</span>
                            <span>${activeBuildRemaining.toFixed(1)}s remaining</span>
                        </div>
                    ` : html`
                        <div style="font-size: 10px; color: #666">No unit in production</div>
                    `}
                </div>

                <!-- Queue Summary -->
                <div style="display: flex; justify-content: space-between; margin-bottom: 8px">
                    <span style="font-size: 11px; color: #888">Queued</span>
                    <span style="font-size: 11px; color: #aaa">
                        ${production.queuedCount > 0 ? `${production.queuedCount} (${queuedParts.join(' ')})` : '0'}
                    </span>
                </div>

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
                                <div style="font-size: 10px; color: #666">${unit.desc} • ${CAMPAIGN_UNIT_BUILD_TIMES[unit.type]}s</div>
                            </div>
                            <span style=${HUD_STYLES.buildCost}>${unit.cost}E</span>
                        </div>
                    `;
                })}

                <hr style=${CAMPAIGN_STYLES.divider} />

                <!-- Research Button (in left panel too for convenience) -->
                <button
                    style=${CAMPAIGN_STYLES.actionBtnPrimary + '; width: 100%; margin-top: 4px'}
                    onClick=${onOpenResearch}
                    onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(40,80,140,0.8)'}
                    onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(30,60,100,0.7)'}
                >Research</button>
            ` : null}
        </div>
    `;
}

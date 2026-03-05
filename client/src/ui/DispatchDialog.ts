import { html } from 'htm/preact';
import { CAMPAIGN_STYLES, HUD_STYLES } from './styles';
import type { CampaignSiteData } from '../bridge/CampaignTypes';
import { SITE_TYPE_NAMES, SiteType, CampaignUnitType, CAMPAIGN_UNIT_NAMES } from '../bridge/CampaignTypes';

export interface DispatchCounts {
    thralls: number;
    sentinels: number;
    tanks: number;
}

export interface DispatchDialogProps {
    sourceSite: CampaignSiteData;
    targetSite: CampaignSiteData;
    counts: DispatchCounts;
    onIncrement: (unitType: CampaignUnitType) => void;
    onDecrement: (unitType: CampaignUnitType) => void;
    onSetMax: (unitType: CampaignUnitType) => void;
    onConfirm: () => void;
    onCancel: () => void;
}

/**
 * Modal dialog for dispatching units between campaign sites.
 * Shows source garrison, target, unit selection (+/- controls), and confirm/cancel.
 */
export function DispatchDialog({
    sourceSite, targetSite, counts,
    onIncrement, onDecrement, onSetMax,
    onConfirm, onCancel,
}: DispatchDialogProps) {
    const sourceName = SITE_TYPE_NAMES[sourceSite.siteType as SiteType] ?? 'Site';
    const targetName = SITE_TYPE_NAMES[targetSite.siteType as SiteType] ?? 'Site';

    // Calculate distance and travel time estimate
    const dx = targetSite.x - sourceSite.x;
    const dy = targetSite.y - sourceSite.y;
    const distance = Math.sqrt(dx * dx + dy * dy);
    const estimatedTime = Math.ceil(distance / 5); // matches Rust TRAVEL_SPEED = 5.0

    const totalDispatching = counts.thralls + counts.sentinels + counts.tanks;

    const unitRows = [
        {
            type: CampaignUnitType.Thrall,
            name: CAMPAIGN_UNIT_NAMES[CampaignUnitType.Thrall],
            available: sourceSite.garrisonThralls,
            selected: counts.thralls,
            color: '#ccaa77',
        },
        {
            type: CampaignUnitType.Sentinel,
            name: CAMPAIGN_UNIT_NAMES[CampaignUnitType.Sentinel],
            available: sourceSite.garrisonSentinels,
            selected: counts.sentinels,
            color: '#77aacc',
        },
        {
            type: CampaignUnitType.HoverTank,
            name: CAMPAIGN_UNIT_NAMES[CampaignUnitType.HoverTank],
            available: sourceSite.garrisonTanks,
            selected: counts.tanks,
            color: '#cc7777',
        },
    ];

    return html`
        <div style=${CAMPAIGN_STYLES.dispatchOverlay} onClick=${(e: Event) => {
            if (e.target === e.currentTarget) onCancel();
        }}>
            <div style=${CAMPAIGN_STYLES.dispatchDialog}>
                <!-- Header -->
                <div style="font-weight: 600; font-size: 16px; color: #e0e0e0; margin-bottom: 12px">
                    Dispatch Forces
                </div>

                <!-- Source & Target -->
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px">
                    <div>
                        <div style="font-size: 10px; color: #888; text-transform: uppercase">From</div>
                        <div style="font-weight: 600; color: #e0e0e0; font-size: 14px">${sourceName} #${sourceSite.siteId}</div>
                    </div>
                    <div style="color: #666; font-size: 18px; padding: 0 16px">→</div>
                    <div style="text-align: right">
                        <div style="font-size: 10px; color: #888; text-transform: uppercase">To</div>
                        <div style="font-weight: 600; color: #e0e0e0; font-size: 14px">${targetName} #${targetSite.siteId}</div>
                    </div>
                </div>

                <!-- Travel Time -->
                <div style="text-align: center; margin-bottom: 12px; font-size: 11px; color: #888">
                    Est. travel: ~${estimatedTime}s | Distance: ${distance.toFixed(0)}
                </div>

                <hr style=${CAMPAIGN_STYLES.divider} />

                <!-- Unit Selection Rows -->
                ${unitRows.map((row) => html`
                    <div style=${CAMPAIGN_STYLES.dispatchRow}>
                        <div style="flex: 1">
                            <span style=${'font-weight: 600; color: ' + row.color}>${row.name}</span>
                            <span style="font-size: 10px; color: #666; margin-left: 6px">(${row.available} available)</span>
                        </div>
                        <div style="display: flex; align-items: center; gap: 4px">
                            <button
                                style=${CAMPAIGN_STYLES.dispatchCountBtn}
                                onClick=${() => onDecrement(row.type)}
                                disabled=${row.selected <= 0}
                            >-</button>
                            <span style=${CAMPAIGN_STYLES.dispatchCount}>${row.selected}</span>
                            <button
                                style=${CAMPAIGN_STYLES.dispatchCountBtn}
                                onClick=${() => onIncrement(row.type)}
                                disabled=${row.selected >= row.available}
                            >+</button>
                            <button
                                style=${'padding: 2px 6px; background: rgba(40,40,55,0.6); border: 1px solid rgba(80,80,100,0.3); border-radius: 3px; color: #888; cursor: pointer; font-size: 9px; margin-left: 4px'}
                                onClick=${() => onSetMax(row.type)}
                                title="Send all"
                            >MAX</button>
                        </div>
                    </div>
                `)}

                <hr style=${CAMPAIGN_STYLES.divider} />

                <!-- Summary -->
                <div style="display: flex; justify-content: space-between; align-items: center; margin: 8px 0">
                    <span style="font-size: 12px; color: #aaa">
                        Total: <span style="font-weight: 600; color: #e0e0e0">${totalDispatching}</span> units
                    </span>
                </div>

                <!-- Action Buttons -->
                <div style="display: flex; gap: 8px; justify-content: flex-end; margin-top: 12px">
                    <button
                        style=${CAMPAIGN_STYLES.actionBtn}
                        onClick=${onCancel}
                    >Cancel</button>
                    <button
                        style=${CAMPAIGN_STYLES.actionBtnPrimary + (totalDispatching === 0 ? '; opacity: 0.5; cursor: default' : '')}
                        onClick=${() => totalDispatching > 0 && onConfirm()}
                        onMouseOver=${(e: Event) => totalDispatching > 0 && ((e.currentTarget as HTMLElement).style.background = 'rgba(40,80,140,0.8)')}
                        onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(30,60,100,0.7)'}
                    >Dispatch</button>
                </div>
            </div>
        </div>
    `;
}

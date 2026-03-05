import { html } from 'htm/preact';
import { HUD_STYLES } from './styles';
import { SpriteType } from '../bridge/types';

const UNIT_DISPLAY_NAMES: Record<number, string> = {
    [SpriteType.Thrall]: 'Thrall',
    [SpriteType.Sentinel]: 'Sentinel',
    [SpriteType.HoverTank]: 'Hover Tank',
    [SpriteType.CommandPost]: 'Command Post',
    [SpriteType.Forge]: 'Forge',
};

export interface SelectionInfo {
    count: number;
    /** For single selection: the entity details. */
    singleName?: string;
    singleHp?: number;
    singleMaxHp?: number;
    /** For multi-selection: count per type. */
    typeCounts?: Map<number, number>;
}

/**
 * Bottom-left selection panel: shows selected unit info.
 */
export function SelectionPanel({ info }: { info: SelectionInfo }) {
    if (info.count === 0) return null;

    if (info.count === 1 && info.singleName) {
        const hpPct = info.singleMaxHp ? Math.round((info.singleHp ?? 0) / info.singleMaxHp * 100) : 100;
        const hpColor = hpPct > 60 ? '#44cc44' : hpPct > 30 ? '#cccc44' : '#cc4444';

        return html`
            <div style=${HUD_STYLES.selectionPanel}>
                <div style=${HUD_STYLES.selectionTitle}>Selected</div>
                <div style=${HUD_STYLES.selectionInfo}>
                    <div style="font-weight: 600; margin-bottom: 4px">${info.singleName}</div>
                    <div style="display: flex; align-items: center; gap: 6px">
                        <span style="color: #888; font-size: 11px">HP</span>
                        <div style=${'width: 80px; height: 6px; background: rgba(40,40,50,0.8); border-radius: 3px; overflow: hidden'}>
                            <div style=${'height: 100%; width: ' + hpPct + '%; background: ' + hpColor + '; border-radius: 3px'}></div>
                        </div>
                        <span style="font-size: 11px; color: #aaa">${Math.round(info.singleHp ?? 0)}/${info.singleMaxHp}</span>
                    </div>
                </div>
            </div>
        `;
    }

    // Multi-selection
    const typeEntries: Array<[string, number]> = [];
    if (info.typeCounts) {
        for (const [type, count] of info.typeCounts) {
            const name = UNIT_DISPLAY_NAMES[type] ?? `Unit ${type}`;
            typeEntries.push([name, count]);
        }
    }

    return html`
        <div style=${HUD_STYLES.selectionPanel}>
            <div style=${HUD_STYLES.selectionTitle}>Selected (${info.count})</div>
            <div style=${HUD_STYLES.selectionInfo}>
                ${typeEntries.map(([name, count]) => html`
                    <div style="margin: 2px 0">${count}x ${name}</div>
                `)}
            </div>
        </div>
    `;
}

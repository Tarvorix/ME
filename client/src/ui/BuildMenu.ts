import { html } from 'htm/preact';
import { HUD_STYLES } from './styles';
import { SpriteType } from '../bridge/types';

export interface ProductionLineInfo {
    unitType: number;
    progress: number;
    totalTime: number;
}

export interface BuildMenuProps {
    lines: ProductionLineInfo[];
    onProduce: (unitType: number) => void;
    onCancel: (lineIndex: number) => void;
}

const BUILDABLE_UNITS = [
    { type: SpriteType.Thrall, name: 'Thrall', cost: 30, time: 5, line: 'Infantry' },
    { type: SpriteType.Sentinel, name: 'Sentinel', cost: 120, time: 15, line: 'Infantry' },
    { type: SpriteType.HoverTank, name: 'Hover Tank', cost: 300, time: 30, line: 'Armor' },
];

/**
 * Bottom-right build menu: produce buttons and active production progress.
 */
export function BuildMenu({ lines, onProduce, onCancel }: BuildMenuProps) {
    return html`
        <div style=${HUD_STYLES.buildMenu}>
            <div style=${HUD_STYLES.buildTitle}>Production</div>

            ${BUILDABLE_UNITS.map((unit) => html`
                <div
                    style=${HUD_STYLES.buildButton}
                    onClick=${() => onProduce(unit.type)}
                    onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(60,60,80,0.8)'}
                    onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(40,40,55,0.6)'}
                >
                    <span>${unit.name} <span style="color: #888; font-size: 10px">(${unit.line})</span></span>
                    <span style=${HUD_STYLES.buildCost}>${unit.cost}E / ${unit.time}s</span>
                </div>
            `)}

            ${lines.length > 0 ? html`
                <div style="margin-top: 8px; border-top: 1px solid rgba(80,80,100,0.3); padding-top: 6px">
                    <div style=${HUD_STYLES.selectionTitle}>Active Lines</div>
                    ${lines.map((line, i) => {
                        if (line.unitType === 0 && line.totalTime === 0) return null;
                        const pct = line.totalTime > 0 ? (line.progress / line.totalTime * 100) : 0;
                        const unitName = getUnitName(line.unitType);
                        return html`
                            <div style="margin: 4px 0">
                                <div style="display: flex; justify-content: space-between; align-items: center">
                                    <span style="font-size: 11px">${unitName}</span>
                                    <div style="display: flex; align-items: center; gap: 4px">
                                        <span style="font-size: 10px; color: #888">${Math.round(pct)}%</span>
                                        <button style=${HUD_STYLES.cancelButton} onClick=${() => onCancel(i)}>X</button>
                                    </div>
                                </div>
                                <div style=${HUD_STYLES.progressBar}>
                                    <div style=${HUD_STYLES.progressFill + '; width: ' + pct + '%'}></div>
                                </div>
                            </div>
                        `;
                    })}
                </div>
            ` : null}
        </div>
    `;
}

function getUnitName(unitType: number): string {
    switch (unitType) {
        case SpriteType.Thrall: return 'Thrall';
        case SpriteType.Sentinel: return 'Sentinel';
        case SpriteType.HoverTank: return 'Hover Tank';
        default: return `Unit ${unitType}`;
    }
}

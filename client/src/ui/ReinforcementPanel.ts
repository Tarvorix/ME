import { html } from 'htm/preact';
import { HUD_STYLES } from './styles';

/** Data about available reinforcements read from UIStateBuffer [196-235]. */
export interface ReinforcementData {
    cpAlive: boolean;
    cooldownTicks: number;
    pendingCount: number;
    availableThralls: number;
    availableSentinels: number;
    availableTanks: number;
    pendingRequests: Array<{
        unitType: number;
        count: number;
        ticksRemaining: number;
    }>;
}

interface ReinforcementPanelProps {
    data: ReinforcementData;
    onReinforce: (unitType: number, count: number) => void;
}

const REINFORCE_PANEL_STYLE = `
    position: fixed;
    bottom: 8px;
    left: 180px;
    padding: 8px 12px;
    background: rgba(15,15,25,0.92);
    border: 1px solid rgba(100,100,120,0.3);
    border-radius: 6px;
    font-size: 12px;
    min-width: 180px;
    max-width: 220px;
`;

const REINFORCE_TITLE_STYLE = `
    color: #aaa;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
`;

const REINFORCE_ROW_STYLE = `
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 0;
    border-bottom: 1px solid rgba(60,60,80,0.2);
`;

const REINFORCE_BTN_STYLE = `
    padding: 3px 10px;
    background: rgba(40,70,40,0.6);
    border: 1px solid rgba(80,140,80,0.4);
    border-radius: 3px;
    color: #aaddaa;
    cursor: pointer;
    font-size: 11px;
    transition: background 0.15s;
`;

const REINFORCE_BTN_DISABLED_STYLE = `
    padding: 3px 10px;
    background: rgba(30,30,40,0.4);
    border: 1px solid rgba(60,60,70,0.3);
    border-radius: 3px;
    color: #666;
    cursor: default;
    font-size: 11px;
`;

const COOLDOWN_STYLE = `
    color: #cc8844;
    font-size: 10px;
    text-align: center;
    padding: 4px 0;
`;

const PENDING_STYLE = `
    color: #88aacc;
    font-size: 10px;
    padding: 2px 0;
`;

const CP_DEAD_STYLE = `
    color: #cc4444;
    font-size: 11px;
    text-align: center;
    padding: 8px 0;
`;

const UNIT_NAMES: Record<number, string> = {
    0: 'Thrall',
    1: 'Sentinel',
    2: 'Hover Tank',
};

function getAvailableCount(data: ReinforcementData, unitType: number): number {
    switch (unitType) {
        case 0: return data.availableThralls;
        case 1: return data.availableSentinels;
        case 2: return data.availableTanks;
        default: return 0;
    }
}

export function ReinforcementPanel({ data, onReinforce }: ReinforcementPanelProps) {
    if (!data.cpAlive) {
        return html`
            <div style=${REINFORCE_PANEL_STYLE}>
                <div style=${REINFORCE_TITLE_STYLE}>Reinforcements</div>
                <div style=${CP_DEAD_STYLE}>
                    Command Post destroyed.<br/>
                    No reinforcements available.
                </div>
            </div>
        `;
    }

    const onCooldown = data.cooldownTicks > 0;
    const cooldownSecs = (data.cooldownTicks * 0.05).toFixed(1);

    const unitTypes = [0, 1, 2];
    const hasAnyUnits = unitTypes.some(t => getAvailableCount(data, t) > 0);

    return html`
        <div style=${REINFORCE_PANEL_STYLE}>
            <div style=${REINFORCE_TITLE_STYLE}>Reinforcements</div>

            ${onCooldown ? html`
                <div style=${COOLDOWN_STYLE}>Cooldown: ${cooldownSecs}s</div>
            ` : null}

            ${unitTypes.map(unitType => {
                const count = getAvailableCount(data, unitType);
                const name = UNIT_NAMES[unitType] ?? `Unit ${unitType}`;
                const canRequest = count > 0 && !onCooldown;

                return html`
                    <div style=${REINFORCE_ROW_STYLE}>
                        <span style="color: #ccc;">
                            ${name} <span style="color: #888;">(${count})</span>
                        </span>
                        <button
                            style=${canRequest ? REINFORCE_BTN_STYLE : REINFORCE_BTN_DISABLED_STYLE}
                            onClick=${canRequest ? () => onReinforce(unitType, 1) : null}
                            disabled=${!canRequest}
                            onMouseOver=${canRequest ? (e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(50,90,50,0.8)' : null}
                            onMouseOut=${canRequest ? (e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(40,70,40,0.6)' : null}
                        >+1</button>
                    </div>
                `;
            })}

            ${!hasAnyUnits && !onCooldown ? html`
                <div style="color: #888; font-size: 10px; text-align: center; padding: 4px 0;">
                    No units in garrison
                </div>
            ` : null}

            ${data.pendingRequests.length > 0 ? html`
                <div style="margin-top: 6px; border-top: 1px solid rgba(80,80,100,0.3); padding-top: 4px;">
                    <div style="color: #888; font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 2px;">
                        En Route
                    </div>
                    ${data.pendingRequests.map(req => {
                        const name = UNIT_NAMES[req.unitType] ?? `Unit ${req.unitType}`;
                        const secs = (req.ticksRemaining * 0.05).toFixed(1);
                        return html`
                            <div style=${PENDING_STYLE}>
                                ${req.count}x ${name} — ${secs}s
                            </div>
                        `;
                    })}
                </div>
            ` : null}
        </div>
    `;
}

/** Read reinforcement data from UIStateBuffer bytes [196-235]. */
export function readReinforcementData(uiView: DataView): ReinforcementData {
    const cpAlive = uiView.getUint8(196) !== 0;
    const cooldownTicks = uiView.getUint8(197);
    const pendingCount = uiView.getUint8(198);
    const availableThralls = uiView.getUint32(200, true);
    const availableSentinels = uiView.getUint32(204, true);
    const availableTanks = uiView.getUint32(208, true);

    const pendingRequests: Array<{ unitType: number; count: number; ticksRemaining: number }> = [];
    for (let i = 0; i < Math.min(pendingCount, 3); i++) {
        const base = 212 + i * 8;
        if (base + 8 > uiView.byteLength) break;
        const unitType = uiView.getUint16(base, true);
        const count = uiView.getUint16(base + 2, true);
        const ticksRemaining = uiView.getUint32(base + 4, true);
        if (count > 0) {
            pendingRequests.push({ unitType, count, ticksRemaining });
        }
    }

    return {
        cpAlive,
        cooldownTicks,
        pendingCount,
        availableThralls,
        availableSentinels,
        availableTanks,
        pendingRequests,
    };
}

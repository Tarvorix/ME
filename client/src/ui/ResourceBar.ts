import { html } from 'htm/preact';
import { HUD_STYLES, strainColor } from './styles';
import type { UIState } from '../bridge/types';

/**
 * Top resource bar: energy, income, expense, net rate, strain meter.
 */
export function ResourceBar({ state }: { state: UIState }) {
    const net = state.income - state.expense;
    const netStr = net >= 0 ? `+${net.toFixed(1)}` : net.toFixed(1);
    const netColor = net >= 0 ? '#44cc44' : '#cc4444';
    const sc = strainColor(state.strain);
    const strainPct = Math.min(100, Math.max(0, state.strain));

    return html`
        <div style=${HUD_STYLES.resourceBar}>
            <div style=${HUD_STYLES.resourceItem}>
                <span style=${HUD_STYLES.resourceLabel}>Energy</span>
                <span style=${HUD_STYLES.resourceValue}>${Math.floor(state.energy)}</span>
            </div>
            <div style=${HUD_STYLES.resourceItem}>
                <span style=${HUD_STYLES.resourceLabel}>Income</span>
                <span style=${HUD_STYLES.resourceValue + '; color: #44cc44'}>${state.income.toFixed(1)}/s</span>
            </div>
            <div style=${HUD_STYLES.resourceItem}>
                <span style=${HUD_STYLES.resourceLabel}>Expense</span>
                <span style=${HUD_STYLES.resourceValue + '; color: #cc8844'}>${state.expense.toFixed(1)}/s</span>
            </div>
            <div style=${HUD_STYLES.resourceItem}>
                <span style=${HUD_STYLES.resourceLabel}>Net</span>
                <span style=${HUD_STYLES.resourceValue + '; color: ' + netColor}>${netStr}/s</span>
            </div>
            <div style=${HUD_STYLES.strainMeter}>
                <span style=${HUD_STYLES.resourceLabel}>Strain</span>
                <div style=${HUD_STYLES.strainBar}>
                    <div style=${HUD_STYLES.strainFill + '; width: ' + strainPct + '%; background: ' + sc}></div>
                </div>
                <span style=${'font-size: 11px; color: ' + sc}>${Math.round(state.strain)}%</span>
            </div>
        </div>
    `;
}

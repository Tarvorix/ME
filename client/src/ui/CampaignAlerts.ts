import { html } from 'htm/preact';
import { CAMPAIGN_STYLES } from './styles';

/** Alert types with corresponding visual styles. */
export type AlertType = 'battle' | 'research' | 'capture' | 'warning' | 'info';

/** A single campaign alert/notification. */
export interface AlertData {
    id: number;
    message: string;
    type: AlertType;
    timestamp: number; // Date.now() when created
}

/** How long alerts remain visible in the feed (ms). Much longer than before. */
export const ALERT_DURATION_MS = 60000;

/** Next alert ID counter. */
let nextAlertId = 1;

/** Create a new alert data object. */
export function createAlert(message: string, type: AlertType): AlertData {
    return {
        id: nextAlertId++,
        message,
        type,
        timestamp: Date.now(),
    };
}

export interface CampaignAlertsProps {
    alerts: AlertData[];
}

/**
 * Alert feed rendered inside the right panel.
 * Shows a scrollable list of recent alerts with type-based styling.
 * Battle alerts are large and prominent.
 */
export function CampaignAlerts({ alerts }: CampaignAlertsProps) {
    return html`
        <div style=${CAMPAIGN_STYLES.alertTitle}>Alerts</div>
        ${alerts.length === 0 ? html`
            <div style="color: #444; font-size: 11px; text-align: center; padding: 8px 0">
                No recent alerts
            </div>
        ` : null}
        ${alerts.slice().reverse().map((alert) => {
            const typeStyle = getAlertTypeStyle(alert.type);
            const icon = getAlertIcon(alert.type);
            const isBattle = alert.type === 'battle';

            // Format timestamp as relative time
            const age = Date.now() - alert.timestamp;
            const ageStr = age < 5000 ? 'now'
                : age < 60000 ? `${Math.floor(age / 1000)}s ago`
                : `${Math.floor(age / 60000)}m ago`;

            return html`
                <div
                    key=${alert.id}
                    style=${CAMPAIGN_STYLES.alertItem + '; ' + typeStyle + (isBattle ? '; font-size: 12px; font-weight: 600; padding: 8px 10px' : '')}
                >
                    <div style="display: flex; justify-content: space-between; align-items: flex-start">
                        <div>
                            <span style="margin-right: 4px">${icon}</span>
                            ${alert.message}
                        </div>
                        <span style="font-size: 9px; color: #555; white-space: nowrap; margin-left: 6px">${ageStr}</span>
                    </div>
                </div>
            `;
        })}
    `;
}

function getAlertTypeStyle(type: AlertType): string {
    switch (type) {
        case 'battle': return CAMPAIGN_STYLES.alertBattle;
        case 'research': return CAMPAIGN_STYLES.alertResearch;
        case 'capture': return CAMPAIGN_STYLES.alertCapture;
        case 'warning': return CAMPAIGN_STYLES.alertWarning;
        case 'info': return CAMPAIGN_STYLES.alertInfo;
    }
}

function getAlertIcon(type: AlertType): string {
    switch (type) {
        case 'battle': return '[!]';
        case 'research': return '[R]';
        case 'capture': return '[C]';
        case 'warning': return '[W]';
        case 'info': return '[i]';
    }
}

/**
 * CSS-in-JS styles for the HUD. Dark industrial theme, responsive.
 */

export const HUD_STYLES = {
    resourceBar: `
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        display: flex;
        align-items: center;
        gap: 16px;
        padding: 6px 12px;
        background: linear-gradient(180deg, rgba(15,15,25,0.92) 0%, rgba(15,15,25,0.75) 100%);
        border-bottom: 1px solid rgba(100,100,120,0.3);
        font-size: 13px;
        z-index: 100;
    `,

    resourceItem: `
        display: flex;
        align-items: center;
        gap: 4px;
    `,

    resourceValue: `
        font-weight: 600;
        color: #e8e8e8;
        min-width: 50px;
        text-align: right;
    `,

    resourceLabel: `
        color: #888;
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    `,

    strainMeter: `
        display: flex;
        align-items: center;
        gap: 6px;
        margin-left: auto;
    `,

    strainBar: `
        width: 80px;
        height: 8px;
        background: rgba(40,40,50,0.8);
        border-radius: 4px;
        overflow: hidden;
        border: 1px solid rgba(80,80,100,0.4);
    `,

    strainFill: `
        height: 100%;
        border-radius: 3px;
        transition: width 0.3s ease, background-color 0.3s ease;
    `,

    selectionPanel: `
        position: fixed;
        bottom: 8px;
        left: 8px;
        padding: 8px 12px;
        background: rgba(15,15,25,0.88);
        border: 1px solid rgba(100,100,120,0.3);
        border-radius: 6px;
        font-size: 12px;
        min-width: 160px;
    `,

    selectionTitle: `
        color: #aaa;
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        margin-bottom: 4px;
    `,

    selectionInfo: `
        color: #e0e0e0;
        font-size: 13px;
    `,

    buildMenu: `
        position: fixed;
        bottom: 8px;
        right: 8px;
        padding: 8px;
        background: rgba(15,15,25,0.88);
        border: 1px solid rgba(100,100,120,0.3);
        border-radius: 6px;
        font-size: 12px;
        min-width: 200px;
    `,

    buildTitle: `
        color: #aaa;
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        margin-bottom: 6px;
    `,

    buildButton: `
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 6px 8px;
        margin: 3px 0;
        background: rgba(40,40,55,0.6);
        border: 1px solid rgba(80,80,100,0.3);
        border-radius: 4px;
        color: #e0e0e0;
        cursor: pointer;
        font-size: 12px;
        transition: background 0.15s;
    `,

    buildButtonHover: `
        background: rgba(60,60,80,0.8);
    `,

    buildCost: `
        color: #FFD700;
        font-size: 11px;
    `,

    progressBar: `
        width: 100%;
        height: 4px;
        background: rgba(40,40,50,0.8);
        border-radius: 2px;
        overflow: hidden;
        margin-top: 4px;
    `,

    progressFill: `
        height: 100%;
        background: #4488FF;
        border-radius: 2px;
        transition: width 0.1s linear;
    `,

    cancelButton: `
        padding: 2px 8px;
        background: rgba(120,40,40,0.6);
        border: 1px solid rgba(160,60,60,0.4);
        border-radius: 3px;
        color: #e88;
        cursor: pointer;
        font-size: 10px;
    `,
};

// ── Campaign-specific styles ────────────────────────────────────────────

export const CAMPAIGN_STYLES = {
    // ── Grid Layout ──────────────────────────────────────────────────────

    /** Full-screen grid layout for the campaign HUD. */
    hudGrid: `
        display: grid;
        grid-template-columns: 280px 1fr 240px;
        grid-template-rows: auto 1fr;
        width: 100%;
        height: 100%;
        pointer-events: none;
    `,

    // ── Top Bar ──────────────────────────────────────────────────────────

    /** Top resource bar spanning full width. */
    topBar: `
        grid-column: 1 / -1;
        grid-row: 1;
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 6px 12px;
        background: linear-gradient(180deg, rgba(15,15,25,0.95) 0%, rgba(15,15,25,0.88) 100%);
        border-bottom: 1px solid rgba(100,100,120,0.3);
        font-size: 12px;
        pointer-events: auto;
    `,

    resourceGroup: `
        display: flex; align-items: center; gap: 8px;
        padding: 0 8px;
        border-right: 1px solid rgba(80,80,100,0.2);
    `,

    pauseBtn: `
        padding: 4px 10px; margin-left: 8px;
        background: rgba(60,40,20,0.6);
        border: 1px solid rgba(120,80,40,0.4);
        border-radius: 4px; color: #ffcc88;
        cursor: pointer; font-size: 11px;
        text-transform: uppercase; letter-spacing: 0.5px;
    `,

    /** Top bar button (Research, Production). */
    topBarBtn: `
        padding: 4px 12px;
        background: rgba(40,50,70,0.7);
        border: 1px solid rgba(80,120,160,0.4);
        border-radius: 4px; color: #aaccee;
        cursor: pointer; font-size: 11px;
        text-transform: uppercase; letter-spacing: 0.5px;
        transition: background 0.15s;
    `,

    // ── Left Panel ───────────────────────────────────────────────────────

    /** Left sidebar for site info and production. */
    leftPanel: `
        grid-column: 1;
        grid-row: 2;
        padding: 10px;
        background: linear-gradient(90deg, rgba(10,10,20,0.92) 0%, rgba(10,10,20,0.85) 100%);
        border-right: 1px solid rgba(80,80,100,0.2);
        overflow-y: auto;
        pointer-events: auto;
    `,

    // ── Right Panel ──────────────────────────────────────────────────────

    /** Right sidebar for alerts feed. */
    rightPanel: `
        grid-column: 3;
        grid-row: 2;
        padding: 8px;
        background: linear-gradient(270deg, rgba(10,10,20,0.92) 0%, rgba(10,10,20,0.85) 100%);
        border-left: 1px solid rgba(80,80,100,0.2);
        overflow-y: auto;
        pointer-events: auto;
        display: flex;
        flex-direction: column;
        gap: 4px;
    `,

    // ── Site Panel (inside left panel) ───────────────────────────────────

    sitePanelTitle: `
        font-weight: 600; font-size: 14px; color: #e0e0e0;
        margin-bottom: 6px;
    `,

    sitePanelRow: `
        display: flex; justify-content: space-between;
        align-items: center; padding: 2px 0;
    `,

    sitePanelActions: `
        display: flex; gap: 4px; margin-top: 8px;
        border-top: 1px solid rgba(80,80,100,0.3);
        padding-top: 8px;
    `,

    /** Production button row (inside left panel when forge selected). */
    produceBtn: `
        display: flex; justify-content: space-between; align-items: center;
        padding: 6px 8px; margin: 3px 0;
        background: rgba(40,40,55,0.6);
        border: 1px solid rgba(80,80,100,0.3);
        border-radius: 4px; color: #e0e0e0;
        cursor: pointer; font-size: 12px;
        transition: background 0.15s;
    `,

    // ── Alert Feed (inside right panel) ──────────────────────────────────

    alertItem: `
        padding: 6px 10px;
        background: rgba(20,20,35,0.7);
        border-left: 3px solid rgba(100,100,120,0.6);
        border-radius: 0 4px 4px 0;
        font-size: 11px; color: #ccc;
    `,

    alertBattle: `border-left-color: #FF4444; background: rgba(60,15,15,0.7);`,
    alertResearch: `border-left-color: #4488FF;`,
    alertCapture: `border-left-color: #44CC44;`,
    alertWarning: `border-left-color: #FFCC44;`,
    alertInfo: `border-left-color: #888888;`,

    alertTitle: `
        font-size: 10px; color: #888;
        text-transform: uppercase; letter-spacing: 0.5px;
        margin-bottom: 6px;
    `,

    // ── Battle Notification Banner ───────────────────────────────────────

    /** Full-screen dark overlay behind the battle banner. */
    battleOverlay: `
        position: absolute;
        top: 0; left: 0; right: 0; bottom: 0;
        display: flex; align-items: center; justify-content: center;
        background: rgba(0,0,0,0.6);
        pointer-events: auto;
        z-index: 200;
    `,

    /** The massive battle notification banner. */
    battleBanner: `
        padding: 32px 48px;
        background: linear-gradient(180deg, rgba(40,10,10,0.95) 0%, rgba(25,8,8,0.95) 100%);
        border: 2px solid rgba(255,68,68,0.6);
        border-radius: 12px;
        text-align: center;
        min-width: 400px;
        max-width: 600px;
        box-shadow: 0 0 40px rgba(255,68,68,0.3), inset 0 0 20px rgba(255,68,68,0.1);
    `,

    battleBannerTitle: `
        font-size: 42px; font-weight: 700;
        color: #ff4444;
        letter-spacing: 6px;
        text-shadow: 0 0 20px rgba(255,68,68,0.5);
        margin-bottom: 8px;
    `,

    battleBannerSubtitle: `
        font-size: 16px; color: #cc8888;
        margin-bottom: 24px;
    `,

    battleBannerButtons: `
        display: flex; gap: 12px; justify-content: center;
    `,

    battleBannerViewBtn: `
        padding: 12px 32px;
        background: rgba(180,40,40,0.7);
        border: 2px solid rgba(255,100,100,0.6);
        border-radius: 6px; color: #ffcccc;
        cursor: pointer; font-size: 16px; font-weight: 600;
        letter-spacing: 1px;
        transition: background 0.2s;
    `,

    battleBannerDismissBtn: `
        padding: 12px 24px;
        background: rgba(40,40,55,0.7);
        border: 1px solid rgba(80,80,100,0.4);
        border-radius: 6px; color: #aaa;
        cursor: pointer; font-size: 14px;
        transition: background 0.2s;
    `,

    // ── Overlays (Research, Dispatch, Victory) ───────────────────────────

    /** Research tree overlay (centered). */
    researchOverlay: `
        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
        display: flex; align-items: center; justify-content: center;
        background: rgba(0,0,0,0.6);
        z-index: 200;
        pointer-events: auto;
    `,

    researchPanel: `
        padding: 20px 24px; min-width: 560px; max-width: 640px;
        background: rgba(15,15,25,0.96);
        border: 1px solid rgba(100,100,120,0.4);
        border-radius: 8px; font-size: 12px;
    `,

    researchGrid: `
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        gap: 8px; margin-top: 12px;
    `,

    techCard: `
        padding: 8px 10px;
        background: rgba(30,30,45,0.6);
        border: 1px solid rgba(60,60,80,0.4);
        border-radius: 4px; cursor: pointer;
        transition: background 0.15s, border-color 0.15s;
        min-height: 80px;
    `,

    techCardAvailable: `
        background: rgba(40,50,70,0.7);
        border-color: rgba(100,140,200,0.5);
    `,

    techCardActive: `
        background: rgba(30,50,80,0.7);
        border-color: rgba(68,136,255,0.7);
    `,

    techCardCompleted: `
        background: rgba(30,60,40,0.6);
        border-color: rgba(68,204,68,0.5);
    `,

    techCardLocked: `
        background: rgba(25,25,35,0.5);
        border-color: rgba(50,50,60,0.3);
        opacity: 0.5; cursor: default;
    `,

    techName: `
        font-weight: 600; font-size: 11px; color: #ccc;
        margin-bottom: 4px;
    `,

    techDesc: `
        font-size: 10px; color: #888;
        margin-bottom: 6px;
    `,

    techCost: `
        font-size: 10px; color: #FFD700;
    `,

    /** Dispatch dialog overlay (centered). */
    dispatchOverlay: `
        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
        display: flex; align-items: center; justify-content: center;
        background: rgba(0,0,0,0.6);
        z-index: 200;
        pointer-events: auto;
    `,

    dispatchDialog: `
        padding: 20px 24px; min-width: 360px;
        background: rgba(15,15,25,0.96);
        border: 1px solid rgba(100,100,120,0.4);
        border-radius: 8px; font-size: 12px;
    `,

    dispatchRow: `
        display: flex; justify-content: space-between; align-items: center;
        padding: 6px 0;
        border-bottom: 1px solid rgba(60,60,80,0.3);
    `,

    dispatchCountBtn: `
        width: 28px; height: 28px;
        display: flex; align-items: center; justify-content: center;
        background: rgba(40,40,55,0.6);
        border: 1px solid rgba(80,80,100,0.3);
        border-radius: 4px; color: #e0e0e0;
        cursor: pointer; font-size: 16px; font-weight: bold;
    `,

    dispatchCount: `
        min-width: 36px; text-align: center;
        font-weight: 600; font-size: 14px; color: #e0e0e0;
    `,

    /** Action button (generic). */
    actionBtn: `
        padding: 6px 14px;
        background: rgba(40,60,80,0.6);
        border: 1px solid rgba(80,120,160,0.4);
        border-radius: 4px; color: #ccddee;
        cursor: pointer; font-size: 12px;
        transition: background 0.15s;
    `,

    actionBtnDanger: `
        padding: 6px 14px;
        background: rgba(80,30,30,0.6);
        border: 1px solid rgba(160,60,60,0.4);
        border-radius: 4px; color: #eecccc;
        cursor: pointer; font-size: 12px;
    `,

    actionBtnPrimary: `
        padding: 6px 14px;
        background: rgba(30,60,100,0.7);
        border: 1px solid rgba(68,136,255,0.5);
        border-radius: 4px; color: #ccdeff;
        cursor: pointer; font-size: 12px; font-weight: 600;
    `,

    /** Section divider. */
    divider: `
        border: none;
        border-top: 1px solid rgba(80,80,100,0.3);
        margin: 6px 0;
    `,

    /** Panel header/title style. */
    panelTitle: `
        color: #aaa; font-size: 11px;
        text-transform: uppercase; letter-spacing: 0.5px;
        margin-bottom: 6px;
    `,

    /** Close button for overlays. */
    closeBtn: `
        position: absolute; top: 8px; right: 12px;
        background: none; border: none;
        color: #888; cursor: pointer;
        font-size: 18px; line-height: 1;
    `,

    /** No-selection placeholder in left panel. */
    leftPanelPlaceholder: `
        color: #555; font-size: 12px;
        padding: 20px 0;
        text-align: center;
    `,
};

export function strainColor(strain: number): string {
    if (strain < 30) return '#44cc44';      // Green
    if (strain < 60) return '#cccc44';      // Yellow
    if (strain < 80) return '#cc8844';      // Orange
    return '#cc4444';                        // Red
}

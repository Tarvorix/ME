import { render } from 'preact';
import { html } from 'htm/preact';
import { CAMPAIGN_STYLES } from './styles';
import { CampaignResourceBar } from './CampaignResourceBar';
import { SitePanel } from './SitePanel';
import { ResearchPanel } from './ResearchPanel';
import { DispatchDialog } from './DispatchDialog';
import type { DispatchCounts } from './DispatchDialog';
import { CampaignAlerts, createAlert, ALERT_DURATION_MS } from './CampaignAlerts';
import type { AlertData, AlertType } from './CampaignAlerts';
import { VictoryScreen } from './VictoryScreen';
import type { MatchStats } from './VictoryScreen';
import type { CampaignBridge } from '../bridge/CampaignBridge';
import type { CampaignRenderer } from '../render/CampaignRenderer';
import type { CampaignSiteData, ActiveBattleData, CampaignEconomyData, CampaignProductionData, CampaignResearchData } from '../bridge/CampaignTypes';
import { CampaignUnitType, SITE_TYPE_NAMES, SiteType, TECH_NAMES, TechId, NO_TECH, NEUTRAL_OWNER } from '../bridge/CampaignTypes';

/** Data for the battle notification banner. */
interface BattleNotification {
    siteId: number;
    siteName: string;
    attacker: number;
    defender: number;
}

/**
 * Campaign HUD orchestrator.
 * Renders the ENTIRE campaign UI as a CSS grid layout:
 *   - TOP BAR: resources + Research/Production buttons + pause
 *   - LEFT PANEL: selected site info + node production (when node selected)
 *   - RIGHT PANEL: alert feed
 *   - OVERLAYS: battle notification, research, dispatch, victory
 *
 * All panels are positioned via CSS grid within #hud-root.
 * No position:fixed on individual panels.
 */
export class CampaignHUD {
    private root: HTMLElement;
    private bridge: CampaignBridge;
    private renderer: CampaignRenderer;

    // Panel state
    private showResearch = false;

    // Battle notification state
    private battleNotification: BattleNotification | null = null;

    // Dispatch dialog state
    private dispatchSourceId = -1;
    private dispatchTargetId = -1;
    private dispatchCounts: DispatchCounts = { thralls: 0, sentinels: 0, tanks: 0 };

    // Alert queue
    private alerts: AlertData[] = [];

    // Previous frame state for change detection
    private prevCompletedCount = -1;
    private prevActiveTechId = -1;
    private prevBattleSiteIds: Set<number> = new Set();
    private prevSiteOwners: Map<number, number> = new Map();
    private prevEliminatedCount = 0;

    // External callbacks
    private viewBattleCallback: ((siteId: number) => void) | null = null;

    // Game end state
    private gameEndState: { victory: boolean; stats: MatchStats } | null = null;
    private gameEndCallbacks: { onPlayAgain: () => void; onMainMenu: () => void } | null = null;

    constructor(bridge: CampaignBridge, renderer: CampaignRenderer) {
        this.bridge = bridge;
        this.renderer = renderer;
        this.root = document.getElementById('hud-root')!;
    }

    /**
     * Called every frame to update all campaign UI panels.
     */
    update(): void {
        // Read current state from bridge
        const economy = this.bridge.getEconomy(0);
        const production = this.bridge.getProduction(0);
        const research = this.bridge.getResearch(0);
        const sites = this.bridge.getSites();
        const battles = this.bridge.getActiveBattles();
        const availableTechs = this.bridge.getAvailableTechs(0);
        const paused = this.bridge.isPaused();
        const tickCount = this.bridge.getTickCount();

        // Detect state changes and generate alerts
        this.detectAlerts(research, battles, sites);

        // Clean up expired alerts
        this.cleanupAlerts();

        // Get selected site data
        const selectedSiteId = this.renderer.selectedSiteId;
        const selectedSite = sites.find(s => s.siteId === selectedSiteId) ?? null;

        // Get player's node data
        const nodeId = this.bridge.getPlayerNode(0);

        // Dispatch dialog site data
        const dispatchSource = sites.find(s => s.siteId === this.dispatchSourceId) ?? null;
        const dispatchTarget = sites.find(s => s.siteId === this.dispatchTargetId) ?? null;
        const showDispatch = dispatchSource !== null && dispatchTarget !== null;

        // Detect mobile layout (viewport narrower than 768px)
        const isMobile = window.innerWidth < 768;

        // Render the entire HUD layout
        render(
            html`
                <!-- ── Grid Layout ────────────────────────────────────── -->
                <div style=${isMobile ? CAMPAIGN_STYLES.hudGridMobile : CAMPAIGN_STYLES.hudGrid}>
                    <!-- TOP BAR -->
                    <div style=${isMobile ? CAMPAIGN_STYLES.topBarMobile : CAMPAIGN_STYLES.topBar}>
                        <${CampaignResourceBar}
                            economy=${economy}
                            production=${production}
                            paused=${paused}
                            tickCount=${tickCount}
                            onTogglePause=${this.handleTogglePause}
                            onOpenResearch=${this.handleOpenResearch}
                            onSelectNode=${() => this.handleSelectNode(nodeId)}
                        />
                    </div>

                    <!-- CENTER (transparent, pointer-events: none — clicks pass to canvas) -->

                    <!-- RIGHT PANEL (hidden on mobile) -->
                    <div style=${isMobile ? CAMPAIGN_STYLES.rightPanelMobile : CAMPAIGN_STYLES.rightPanel}>
                        <${CampaignAlerts} alerts=${this.alerts} />
                    </div>

                    <!-- LEFT PANEL (only visible when a site is selected) -->
                    <!-- On mobile: docked to bottom as a sheet (row 3) -->
                    ${selectedSite ? html`
                        <div style=${isMobile ? CAMPAIGN_STYLES.leftPanelMobile : CAMPAIGN_STYLES.leftPanel}>
                            <${SitePanel}
                                site=${selectedSite}
                                playerNodeId=${nodeId}
                                battles=${battles}
                                economy=${economy}
                                production=${production}
                                onDispatchFrom=${this.handleDispatchFrom}
                                onWithdraw=${this.handleWithdraw}
                                onViewBattle=${this.handleViewBattle}
                                onProduce=${this.handleProduce}
                                onOpenResearch=${this.handleOpenResearch}
                            />
                        </div>
                    ` : null}
                </div>

                <!-- ── Battle Notification Overlay ────────────────────── -->
                ${this.battleNotification ? html`
                    <div style=${CAMPAIGN_STYLES.battleOverlay}>
                        <div style=${CAMPAIGN_STYLES.battleBanner}>
                            <div style=${CAMPAIGN_STYLES.battleBannerTitle}>
                                BATTLE!
                            </div>
                            <div style=${CAMPAIGN_STYLES.battleBannerSubtitle}>
                                Your forces are engaged at ${this.battleNotification.siteName}
                            </div>
                            <div style="font-size: 13px; color: #aa7777; margin-bottom: 20px">
                                Player ${this.battleNotification.attacker + 1} vs Player ${this.battleNotification.defender + 1}
                            </div>
                            <div style=${CAMPAIGN_STYLES.battleBannerButtons}>
                                <button
                                    style=${CAMPAIGN_STYLES.battleBannerViewBtn}
                                    onClick=${() => this.handleBattleNotificationView()}
                                    onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(200,50,50,0.8)'}
                                    onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(180,40,40,0.7)'}
                                >VIEW BATTLE</button>
                                <button
                                    style=${CAMPAIGN_STYLES.battleBannerDismissBtn}
                                    onClick=${() => this.handleBattleNotificationDismiss()}
                                    onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(50,50,65,0.8)'}
                                    onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'rgba(40,40,55,0.7)'}
                                >CONTINUE</button>
                            </div>
                        </div>
                    </div>
                ` : null}

                <!-- ── Research Overlay ────────────────────────────────── -->
                ${this.showResearch ? html`
                    <${ResearchPanel}
                        research=${research}
                        availableTechs=${availableTechs}
                        economy=${economy}
                        onResearch=${this.handleResearch}
                        onClose=${this.handleCloseResearch}
                    />
                ` : null}

                <!-- ── Dispatch Dialog Overlay ─────────────────────────── -->
                ${showDispatch ? html`
                    <${DispatchDialog}
                        sourceSite=${dispatchSource}
                        targetSite=${dispatchTarget}
                        counts=${this.dispatchCounts}
                        onIncrement=${this.handleDispatchIncrement}
                        onDecrement=${this.handleDispatchDecrement}
                        onSetMax=${this.handleDispatchMax}
                        onConfirm=${this.handleDispatchConfirm}
                        onCancel=${this.handleDispatchCancel}
                    />
                ` : null}

                <!-- ── Victory/Defeat Screen ──────────────────────────── -->
                ${this.gameEndState ? html`
                    <${VictoryScreen}
                        victory=${this.gameEndState.victory}
                        stats=${this.gameEndState.stats}
                        onPlayAgain=${this.gameEndCallbacks?.onPlayAgain ?? (() => {})}
                        onMainMenu=${this.gameEndCallbacks?.onMainMenu ?? (() => {})}
                    />
                ` : null}
            `,
            this.root,
        );
    }

    // ── Alert Detection ─────────────────────────────────────────────────

    private detectAlerts(
        research: CampaignResearchData,
        battles: ActiveBattleData[],
        sites: CampaignSiteData[],
    ): void {
        // Research completion
        if (this.prevCompletedCount >= 0 && research.completedCount > this.prevCompletedCount) {
            const diff = research.completedCount - this.prevCompletedCount;
            for (let i = 0; i < diff; i++) {
                const techId = research.completedTechs[research.completedTechs.length - 1 - i];
                if (techId !== undefined) {
                    const name = TECH_NAMES[techId as TechId] ?? `Tech ${techId}`;
                    this.addAlert(`Research complete: ${name}`, 'research');
                }
            }
        }
        this.prevCompletedCount = research.completedCount;

        // New battles
        const currentBattleSiteIds = new Set(battles.map(b => b.siteId));
        for (const siteId of currentBattleSiteIds) {
            if (!this.prevBattleSiteIds.has(siteId)) {
                const site = sites.find(s => s.siteId === siteId);
                const battle = battles.find(b => b.siteId === siteId);
                const siteName = site ? (SITE_TYPE_NAMES[site.siteType as SiteType] ?? 'Site') : 'Site';
                this.addAlert(`Battle started at ${siteName} #${siteId}!`, 'battle');

                // Battle notification banner for player 0's battles
                if (battle && (battle.attacker === 0 || battle.defender === 0)) {
                    this.battleNotification = {
                        siteId,
                        siteName: `${siteName} #${siteId}`,
                        attacker: battle.attacker,
                        defender: battle.defender,
                    };
                    // Auto-pause the game
                    this.bridge.setPaused(true);
                }
            }
        }
        // Battles that ended
        for (const siteId of this.prevBattleSiteIds) {
            if (!currentBattleSiteIds.has(siteId)) {
                const site = sites.find(s => s.siteId === siteId);
                const siteName = site ? (SITE_TYPE_NAMES[site.siteType as SiteType] ?? 'Site') : 'Site';
                this.addAlert(`Battle ended at ${siteName} #${siteId}`, 'battle');
            }
        }
        this.prevBattleSiteIds = currentBattleSiteIds;

        // Site ownership changes
        for (const site of sites) {
            const prevOwner = this.prevSiteOwners.get(site.siteId);
            if (prevOwner !== undefined && prevOwner !== site.owner) {
                const siteName = SITE_TYPE_NAMES[site.siteType as SiteType] ?? 'Site';
                if (site.owner === 0) {
                    this.addAlert(`${siteName} #${site.siteId} captured!`, 'capture');
                } else if (prevOwner === 0) {
                    this.addAlert(`${siteName} #${site.siteId} lost!`, 'warning');
                }
            }
            this.prevSiteOwners.set(site.siteId, site.owner);
        }

        // Player elimination
        const eliminated = this.bridge.getEliminatedPlayers();
        if (eliminated.length > this.prevEliminatedCount) {
            for (let i = this.prevEliminatedCount; i < eliminated.length; i++) {
                this.addAlert(`Player ${eliminated[i] + 1} eliminated!`, 'warning');
            }
        }
        this.prevEliminatedCount = eliminated.length;
    }

    private addAlert(message: string, type: AlertType): void {
        this.alerts.push(createAlert(message, type));
        // Cap at 30 alerts max (more than before since we show a history feed)
        if (this.alerts.length > 30) {
            this.alerts.shift();
        }
    }

    private cleanupAlerts(): void {
        const now = Date.now();
        this.alerts = this.alerts.filter(a => now - a.timestamp < ALERT_DURATION_MS);
    }

    // ── Battle Notification Handlers ─────────────────────────────────────

    private handleBattleNotificationView = (): void => {
        if (this.battleNotification && this.viewBattleCallback) {
            const siteId = this.battleNotification.siteId;
            this.battleNotification = null;
            // Unpause so the battle simulation actually runs
            this.bridge.setPaused(false);
            this.viewBattleCallback(siteId);
        }
    };

    private handleBattleNotificationDismiss = (): void => {
        this.battleNotification = null;
        // Unpause when the player dismisses the notification
        this.bridge.setPaused(false);
    };

    // ── Event Handlers ──────────────────────────────────────────────────

    private handleTogglePause = (): void => {
        const paused = this.bridge.isPaused();
        this.bridge.setPaused(!paused);
    };

    private handleOpenResearch = (): void => {
        this.showResearch = true;
    };

    private handleSelectNode = (nodeId: number): void => {
        if (nodeId >= 0) {
            this.renderer.selectedSiteId = nodeId;
        }
    };

    private handleProduce = (unitType: number): void => {
        const success = this.bridge.cmdProduce(0, unitType, 1);
        if (success) {
            const names = ['Thrall', 'Sentinel', 'Hover Tank'];
            this.addAlert(`${names[unitType] ?? 'Unit'} queued`, 'info');
        }
    };

    private handleResearch = (techId: number): void => {
        const success = this.bridge.cmdResearch(0, techId);
        if (success) {
            const name = TECH_NAMES[techId as TechId] ?? `Tech ${techId}`;
            this.addAlert(`Research started: ${name}`, 'research');
        }
        this.showResearch = false;
    };

    private handleWithdraw = (siteId: number): void => {
        const success = this.bridge.cmdWithdraw(0, siteId);
        if (success) {
            this.addAlert('Garrison withdrawn to node', 'info');
        }
    };

    private handleViewBattle = (siteId: number): void => {
        if (this.viewBattleCallback) {
            // Ensure campaign is unpaused so the battle simulation actually runs
            this.bridge.setPaused(false);
            this.viewBattleCallback(siteId);
        }
    };

    // ── Dispatch Dialog ─────────────────────────────────────────────────

    private handleDispatchFrom = (siteId: number): void => {
        this.dispatchSourceId = siteId;
        this.dispatchTargetId = -1;
        this.dispatchCounts = { thralls: 0, sentinels: 0, tanks: 0 };
        this.addAlert('Select dispatch target (click a site)', 'info');
    };

    /** Called by the input manager when a target site is selected for dispatch. */
    setDispatchTarget(targetSiteId: number): void {
        if (this.dispatchSourceId < 0) return;
        if (targetSiteId === this.dispatchSourceId) return;
        this.dispatchTargetId = targetSiteId;
        this.dispatchCounts = { thralls: 0, sentinels: 0, tanks: 0 };
    }

    /** Check if we're waiting for a dispatch target selection. */
    isWaitingForDispatchTarget(): boolean {
        return this.dispatchSourceId >= 0 && this.dispatchTargetId < 0;
    }

    /** Get the dispatch source site ID. */
    getDispatchSourceId(): number {
        return this.dispatchSourceId;
    }

    private handleDispatchIncrement = (unitType: CampaignUnitType): void => {
        const source = this.bridge.getSites().find(s => s.siteId === this.dispatchSourceId);
        if (!source) return;

        switch (unitType) {
            case CampaignUnitType.Thrall:
                if (this.dispatchCounts.thralls < source.garrisonThralls) {
                    this.dispatchCounts = { ...this.dispatchCounts, thralls: this.dispatchCounts.thralls + 1 };
                }
                break;
            case CampaignUnitType.Sentinel:
                if (this.dispatchCounts.sentinels < source.garrisonSentinels) {
                    this.dispatchCounts = { ...this.dispatchCounts, sentinels: this.dispatchCounts.sentinels + 1 };
                }
                break;
            case CampaignUnitType.HoverTank:
                if (this.dispatchCounts.tanks < source.garrisonTanks) {
                    this.dispatchCounts = { ...this.dispatchCounts, tanks: this.dispatchCounts.tanks + 1 };
                }
                break;
        }
    };

    private handleDispatchDecrement = (unitType: CampaignUnitType): void => {
        switch (unitType) {
            case CampaignUnitType.Thrall:
                this.dispatchCounts = { ...this.dispatchCounts, thralls: Math.max(0, this.dispatchCounts.thralls - 1) };
                break;
            case CampaignUnitType.Sentinel:
                this.dispatchCounts = { ...this.dispatchCounts, sentinels: Math.max(0, this.dispatchCounts.sentinels - 1) };
                break;
            case CampaignUnitType.HoverTank:
                this.dispatchCounts = { ...this.dispatchCounts, tanks: Math.max(0, this.dispatchCounts.tanks - 1) };
                break;
        }
    };

    private handleDispatchMax = (unitType: CampaignUnitType): void => {
        const source = this.bridge.getSites().find(s => s.siteId === this.dispatchSourceId);
        if (!source) return;

        switch (unitType) {
            case CampaignUnitType.Thrall:
                this.dispatchCounts = { ...this.dispatchCounts, thralls: source.garrisonThralls };
                break;
            case CampaignUnitType.Sentinel:
                this.dispatchCounts = { ...this.dispatchCounts, sentinels: source.garrisonSentinels };
                break;
            case CampaignUnitType.HoverTank:
                this.dispatchCounts = { ...this.dispatchCounts, tanks: source.garrisonTanks };
                break;
        }
    };

    private handleDispatchConfirm = (): void => {
        const { thralls, sentinels, tanks } = this.dispatchCounts;
        const units: Array<{ unitType: number; count: number }> = [];

        if (thralls > 0) units.push({ unitType: CampaignUnitType.Thrall, count: thralls });
        if (sentinels > 0) units.push({ unitType: CampaignUnitType.Sentinel, count: sentinels });
        if (tanks > 0) units.push({ unitType: CampaignUnitType.HoverTank, count: tanks });

        if (units.length === 0) return;

        // Look up target ownership for diagnostic feedback
        const sites = this.bridge.getSites();
        const targetSite = sites.find(s => s.siteId === this.dispatchTargetId);
        const targetOwner = targetSite ? targetSite.owner : 255;
        const targetName = targetSite ? (SITE_TYPE_NAMES[targetSite.siteType as SiteType] ?? 'Site') : 'Site';

        const result = this.bridge.cmdDispatch(0, this.dispatchSourceId, this.dispatchTargetId, units);
        if (result >= 0) {
            const total = thralls + sentinels + tanks;
            if (targetOwner === NEUTRAL_OWNER) {
                this.addAlert(`${total} units en route to claim neutral ${targetName}`, 'info');
            } else if (targetOwner === 0) {
                this.addAlert(`${total} units en route to reinforce ${targetName}`, 'info');
            } else {
                this.addAlert(`${total} units en route to attack enemy ${targetName}!`, 'warning');
            }
            console.log(`Dispatch order #${result}: ${total} units from site ${this.dispatchSourceId} -> site ${this.dispatchTargetId} (owner=${targetOwner})`);
        } else {
            this.addAlert('Dispatch failed!', 'warning');
            console.warn('Dispatch failed:', { source: this.dispatchSourceId, target: this.dispatchTargetId, units });
        }

        this.dispatchSourceId = -1;
        this.dispatchTargetId = -1;
        this.dispatchCounts = { thralls: 0, sentinels: 0, tanks: 0 };
    };

    private handleDispatchCancel = (): void => {
        this.dispatchSourceId = -1;
        this.dispatchTargetId = -1;
        this.dispatchCounts = { thralls: 0, sentinels: 0, tanks: 0 };
    };

    private handleCloseResearch = (): void => {
        this.showResearch = false;
    };

    // ── Public API ──────────────────────────────────────────────────────

    /** Toggle research panel visibility. */
    toggleResearch(): void {
        this.showResearch = !this.showResearch;
    }

    /** Check if research panel is open. */
    isResearchOpen(): boolean {
        return this.showResearch;
    }

    /** Check if dispatch dialog is open. */
    isDispatchOpen(): boolean {
        return this.dispatchSourceId >= 0 && this.dispatchTargetId >= 0;
    }

    /** Check if any blocking overlay is open (includes battle notification). */
    isOverlayOpen(): boolean {
        return this.showResearch || this.isDispatchOpen() || this.battleNotification !== null;
    }

    /** Close all overlays (research, dispatch). */
    closeOverlays(): void {
        this.showResearch = false;
        this.handleDispatchCancel();
    }

    /** Set the callback for when "View Battle" is clicked on a site with an active battle. */
    setViewBattleCallback(cb: (siteId: number) => void): void {
        this.viewBattleCallback = cb;
    }

    /** Set game end state to show the victory/defeat screen overlay. */
    setGameEnd(victory: boolean, stats: MatchStats): void {
        this.gameEndState = { victory, stats };
    }

    /** Set callbacks for play again / main menu buttons on the victory screen. */
    setGameEndCallbacks(callbacks: { onPlayAgain: () => void; onMainMenu: () => void }): void {
        this.gameEndCallbacks = callbacks;
    }

    /** Check if the game has ended. */
    isGameOver(): boolean {
        return this.gameEndState !== null;
    }

    /** Destroy the HUD (clear the root element). */
    destroy(): void {
        render(null, this.root);
    }
}

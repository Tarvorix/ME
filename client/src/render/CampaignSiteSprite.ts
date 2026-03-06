import { Container, Graphics, Text } from 'pixi.js';
import type { CampaignSiteData } from '../bridge/CampaignTypes';
import { SiteType, NEUTRAL_OWNER } from '../bridge/CampaignTypes';
import { PLAYER_COLORS, NEUTRAL_COLOR } from '../config';

/**
 * PixiJS visual for a single campaign map site.
 * Draws a type-specific icon, owner color ring, garrison counts,
 * battle pulse indicator, and selection/hover highlights.
 * All art is procedural (Graphics-based, no sprite assets needed).
 */
export class CampaignSiteSprite extends Container {
    // Visual layers (bottom to top)
    private ownerRing: Graphics;
    private iconGfx: Graphics;
    private selectionRing: Graphics;
    private hoverRing: Graphics;
    private battlePulse: Graphics;
    private garrisonLabel: Text;
    private nameLabel: Text;

    // State tracking for dirty checks
    private lastOwner = -1;
    private lastSiteType = -1;
    private lastBattleId = -1;
    private lastGarrisonHash = '';
    private _selected = false;
    private _hovered = false;

    /** Public reference to the site ID this sprite represents. */
    readonly siteId: number;

    /** Latest site data from the bridge. */
    siteData: CampaignSiteData;

    constructor(data: CampaignSiteData) {
        super();
        this.siteId = data.siteId;
        this.siteData = data;

        // ── Create layer hierarchy (order = draw order) ──

        // Owner color ring (behind everything)
        this.ownerRing = new Graphics();
        this.addChild(this.ownerRing);

        // Site type icon
        this.iconGfx = new Graphics();
        this.addChild(this.iconGfx);

        // Selection ring (only visible when selected)
        this.selectionRing = new Graphics();
        this.selectionRing.visible = false;
        this.addChild(this.selectionRing);

        // Hover ring (only visible on hover)
        this.hoverRing = new Graphics();
        this.hoverRing.visible = false;
        this.addChild(this.hoverRing);

        // Battle pulse ring (only visible during active battles)
        this.battlePulse = new Graphics();
        this.battlePulse.visible = false;
        this.addChild(this.battlePulse);

        // Garrison text below the icon
        this.garrisonLabel = new Text({
            text: '',
            style: {
                fill: 0xcccccc,
                fontSize: 14,
                fontFamily: 'Arial, Helvetica, sans-serif',
                fontWeight: 'bold',
            },
        });
        this.garrisonLabel.anchor.set(0.5, 0);
        this.addChild(this.garrisonLabel);

        // Site name label below garrison
        this.nameLabel = new Text({
            text: this.getSiteLabel(data.siteType),
            style: {
                fill: 0x888888,
                fontSize: 12,
                fontFamily: 'Arial, Helvetica, sans-serif',
                letterSpacing: 1,
            },
        });
        this.nameLabel.anchor.set(0.5, 0);
        this.addChild(this.nameLabel);

        // Initial draw
        this.drawIcon(data.siteType);
        this.drawOwnerRing(data.owner);
        this.drawSelectionRing(data.siteType);
        this.drawHoverRing(data.siteType);
        this.drawBattlePulse(data.siteType);
        this.updateGarrisonText(data);
        this.updateLabelPositions();
    }

    /**
     * Update this sprite with latest site data from the bridge.
     * Only redraws graphics when the relevant data actually changes.
     */
    updateData(data: CampaignSiteData, animTime: number): void {
        this.siteData = data;

        // Redraw owner ring if ownership changed
        if (data.owner !== this.lastOwner) {
            this.drawOwnerRing(data.owner);
            this.lastOwner = data.owner;
        }

        // Redraw icon if site type changed (shouldn't happen, but handle it)
        if (data.siteType !== this.lastSiteType) {
            this.drawIcon(data.siteType);
            this.drawSelectionRing(data.siteType);
            this.drawHoverRing(data.siteType);
            this.drawBattlePulse(data.siteType);
            this.nameLabel.text = this.getSiteLabel(data.siteType);
            this.updateLabelPositions();
            this.lastSiteType = data.siteType;
        }

        // Battle pulse animation
        const hasBattle = data.battleId !== 0;
        this.battlePulse.visible = hasBattle;
        if (hasBattle) {
            // Pulsing alpha for battle indicator
            const pulse = 0.3 + 0.7 * Math.abs(Math.sin(animTime * 3.0));
            this.battlePulse.alpha = pulse;

            // Redraw if battle changed (new battle started)
            if (data.battleId !== this.lastBattleId) {
                this.drawBattlePulse(data.siteType);
                this.lastBattleId = data.battleId;
            }
        }

        // Update garrison text if counts changed
        const garrisonHash = `${data.garrisonThralls}:${data.garrisonSentinels}:${data.garrisonTanks}`;
        if (garrisonHash !== this.lastGarrisonHash) {
            this.updateGarrisonText(data);
            this.lastGarrisonHash = garrisonHash;
        }
    }

    /** Toggle selection highlight. */
    setSelected(selected: boolean): void {
        if (selected === this._selected) return;
        this._selected = selected;
        this.selectionRing.visible = selected;
    }

    /** Toggle hover highlight. */
    setHovered(hovered: boolean): void {
        if (hovered === this._hovered) return;
        this._hovered = hovered;
        this.hoverRing.visible = hovered;
    }

    // ── Drawing Methods ─────────────────────────────────────────────────

    private drawIcon(siteType: SiteType): void {
        this.iconGfx.clear();

        switch (siteType) {
            case SiteType.Node:
                this.drawNodeIcon();
                break;
            case SiteType.MiningStation:
                this.drawMineIcon();
                break;
            case SiteType.RelicSite:
                this.drawRelicIcon();
                break;
        }
    }

    /**
     * Node icon: Gear/cog shape with 8 teeth and inner axle.
     * Largest icon since it's the player's home base.
     */
    private drawNodeIcon(): void {
        const g = this.iconGfx;
        const numTeeth = 8;
        const innerR = 20;
        const outerR = 32;
        const totalPoints = numTeeth * 2;

        // Gear body (alternating inner/outer radius points)
        const firstAngle = 0;
        g.moveTo(Math.cos(firstAngle) * outerR, Math.sin(firstAngle) * outerR);

        for (let i = 1; i < totalPoints; i++) {
            const angle = (i / totalPoints) * Math.PI * 2;
            const r = i % 2 === 0 ? outerR : innerR;
            g.lineTo(Math.cos(angle) * r, Math.sin(angle) * r);
        }
        g.closePath();
        g.fill({ color: 0x555566 });
        g.stroke({ color: 0x8888aa, width: 1.5 });

        // Inner axle circle
        g.circle(0, 0, 8);
        g.fill({ color: 0x333344 });
        g.stroke({ color: 0x8888aa, width: 1.5 });
    }

    /**
     * Mining Station icon: Hexagon with crosshair pattern inside.
     */
    private drawMineIcon(): void {
        const g = this.iconGfx;
        const r = 26;

        // Hexagon (flat-top orientation)
        const startAngle = -Math.PI / 6;
        g.moveTo(Math.cos(startAngle) * r, Math.sin(startAngle) * r);
        for (let i = 1; i < 6; i++) {
            const angle = (i / 6) * Math.PI * 2 + startAngle;
            g.lineTo(Math.cos(angle) * r, Math.sin(angle) * r);
        }
        g.closePath();
        g.fill({ color: 0x665544 });
        g.stroke({ color: 0xaa9977, width: 1.5 });

        // Pickaxe crosshair inside
        g.moveTo(-10, -10);
        g.lineTo(10, 10);
        g.moveTo(10, -10);
        g.lineTo(-10, 10);
        g.stroke({ color: 0xccbb99, width: 2 });
    }

    /**
     * Relic Site icon: Elongated diamond/crystal shape with inner facet.
     */
    private drawRelicIcon(): void {
        const g = this.iconGfx;
        const h = 30; // half-height
        const w = 18;  // half-width

        // Outer diamond
        g.moveTo(0, -h);
        g.lineTo(w, 0);
        g.lineTo(0, h);
        g.lineTo(-w, 0);
        g.closePath();
        g.fill({ color: 0x445566 });
        g.stroke({ color: 0x77aacc, width: 1.5 });

        // Inner facet (smaller diamond for crystal detail)
        const ih = h * 0.5;
        const iw = w * 0.5;
        g.moveTo(0, -ih);
        g.lineTo(iw, 0);
        g.lineTo(0, ih);
        g.lineTo(-iw, 0);
        g.closePath();
        g.fill({ color: 0x5588aa, alpha: 0.5 });
    }

    /**
     * Owner ring: Colored circle behind the icon showing who owns this site.
     */
    private drawOwnerRing(owner: number): void {
        this.ownerRing.clear();
        const color = owner === NEUTRAL_OWNER
            ? NEUTRAL_COLOR
            : (PLAYER_COLORS[owner] ?? NEUTRAL_COLOR);
        const radius = this.getIconRadius() + 6;

        // Filled glow background
        this.ownerRing.circle(0, 0, radius);
        this.ownerRing.fill({ color, alpha: 0.2 });

        // Solid ring border
        this.ownerRing.circle(0, 0, radius);
        this.ownerRing.stroke({ color, width: 3, alpha: 0.8 });
    }

    /**
     * Selection ring: White ring shown when the site is selected.
     */
    private drawSelectionRing(siteType: SiteType): void {
        this.selectionRing.clear();
        const radius = this.getIconRadius() + 10;

        this.selectionRing.circle(0, 0, radius);
        this.selectionRing.stroke({ color: 0xFFFFFF, width: 3, alpha: 0.9 });

        // Inner glow
        this.selectionRing.circle(0, 0, radius - 2);
        this.selectionRing.stroke({ color: 0xFFFFFF, width: 1, alpha: 0.3 });
    }

    /**
     * Hover ring: Subtle highlight when the mouse hovers over this site.
     */
    private drawHoverRing(siteType: SiteType): void {
        this.hoverRing.clear();
        const radius = this.getIconRadius() + 9;

        this.hoverRing.circle(0, 0, radius);
        this.hoverRing.stroke({ color: 0xCCCCFF, width: 2, alpha: 0.5 });
    }

    /**
     * Battle pulse: Red pulsing ring when a battle is active at this site.
     */
    private drawBattlePulse(siteType: SiteType): void {
        this.battlePulse.clear();
        const radius = this.getIconRadius() + 16;

        // Outer pulse ring
        this.battlePulse.circle(0, 0, radius);
        this.battlePulse.stroke({ color: 0xFF4444, width: 4, alpha: 0.8 });

        // Inner glow
        this.battlePulse.circle(0, 0, radius - 3);
        this.battlePulse.stroke({ color: 0xFF6666, width: 2, alpha: 0.4 });
    }

    /**
     * Update garrison count text showing unit breakdown.
     */
    private updateGarrisonText(data: CampaignSiteData): void {
        const total = data.garrisonThralls + data.garrisonSentinels + data.garrisonTanks;
        if (total > 0) {
            const parts: string[] = [];
            if (data.garrisonThralls > 0) parts.push(`${data.garrisonThralls}T`);
            if (data.garrisonSentinels > 0) parts.push(`${data.garrisonSentinels}S`);
            if (data.garrisonTanks > 0) parts.push(`${data.garrisonTanks}H`);
            this.garrisonLabel.text = parts.join(' ');
            this.garrisonLabel.visible = true;
        } else {
            this.garrisonLabel.text = '';
            this.garrisonLabel.visible = false;
        }
    }

    /**
     * Position labels below the icon based on the icon size.
     */
    private updateLabelPositions(): void {
        const r = this.getIconRadius();
        this.garrisonLabel.y = r + 10;
        this.nameLabel.y = r + 28;
    }

    /**
     * Get the base icon radius for this site type.
     * Node is largest (home base), others are smaller.
     */
    private getIconRadius(): number {
        switch (this.siteData.siteType) {
            case SiteType.Node: return 36;
            case SiteType.MiningStation: return 30;
            case SiteType.RelicSite: return 30;
            default: return 28;
        }
    }

    /**
     * Get the display label for this site type.
     */
    private getSiteLabel(siteType: SiteType): string {
        switch (siteType) {
            case SiteType.Node: return 'NODE';
            case SiteType.MiningStation: return 'MINE';
            case SiteType.RelicSite: return 'RELIC';
            default: return 'SITE';
        }
    }
}

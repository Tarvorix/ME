import type { CampaignBridge } from './CampaignBridge';

/**
 * Adapter that wraps a CampaignBridge's battle-specific methods to present
 * the same interface as GameBridge. This allows GameRenderer and InputManager
 * to operate on a campaign battle without modification.
 *
 * Usage: `new GameRenderer().init(adapter as unknown as GameBridge, app)`
 *
 * The campaign simulation tick drives the battle internally, so tick() is a no-op.
 * All render/event/fog/map queries route to the campaign bridge's battle methods.
 * All unit commands route to the campaign bridge's battle command methods.
 */
export class CampaignBattleAdapter {
    private campaignBridge: CampaignBridge;
    private battleSiteId: number;
    private memory: WebAssembly.Memory;

    constructor(campaignBridge: CampaignBridge, battleSiteId: number, memory: WebAssembly.Memory) {
        this.campaignBridge = campaignBridge;
        this.battleSiteId = battleSiteId;
        this.memory = memory;
    }

    // ── Lifecycle (no-ops for campaign battles) ─────────────────────────

    async init(): Promise<void> { /* no-op */ }

    startGame(_mapWidth: number, _mapHeight: number, _playerCount: number, _seed: number): void {
        /* no-op — battle is already initialized via campaign */
    }

    /** No-op: campaign tick drives battles internally. */
    tick(_deltaMs: number): void {
        /* no-op — the GameFlowController ticks the campaign, which ticks the battle */
    }

    // ── Render Buffer ───────────────────────────────────────────────────

    getRenderBuffer(): DataView {
        const battleIndex = this.resolveBattleIndex();
        if (battleIndex < 0) {
            return new DataView(new ArrayBuffer(0));
        }
        const buf = this.campaignBridge.getBattleRenderBuffer(battleIndex);
        if (buf) return buf;
        // Return empty DataView if no buffer available
        return new DataView(new ArrayBuffer(0));
    }

    getRenderCount(): number {
        const battleIndex = this.resolveBattleIndex();
        return battleIndex >= 0 ? this.campaignBridge.getBattleRenderCount(battleIndex) : 0;
    }

    // ── Map ─────────────────────────────────────────────────────────────

    getMapTile(x: number, y: number): { terrain: number; variant: number } {
        const battleIndex = this.resolveBattleIndex();
        return battleIndex >= 0
            ? this.campaignBridge.getBattleMapTile(battleIndex, x, y)
            : { terrain: 0, variant: 0 };
    }

    getMapWidth(): number {
        const battleIndex = this.resolveBattleIndex();
        return battleIndex >= 0 ? this.campaignBridge.getBattleMapSize(battleIndex).width : 0;
    }

    getMapHeight(): number {
        const battleIndex = this.resolveBattleIndex();
        return battleIndex >= 0 ? this.campaignBridge.getBattleMapSize(battleIndex).height : 0;
    }

    // ── Fog of War ──────────────────────────────────────────────────────

    getFogBuffer(player: number): Uint8Array {
        const battleIndex = this.resolveBattleIndex();
        if (battleIndex < 0) {
            return new Uint8Array(0);
        }
        const buf = this.campaignBridge.getBattleFogBuffer(battleIndex, player);
        if (buf) return buf;
        return new Uint8Array(0);
    }

    getFogBufferLen(): number {
        const battleIndex = this.resolveBattleIndex();
        return battleIndex >= 0 ? this.campaignBridge.getBattleFogLen(battleIndex) : 0;
    }

    // ── UI State ────────────────────────────────────────────────────────

    getUIState(): DataView {
        const battleIndex = this.resolveBattleIndex();
        if (battleIndex < 0) {
            return new DataView(new ArrayBuffer(256));
        }
        const state = this.campaignBridge.getBattleUIState(battleIndex);
        if (state) return state;
        return new DataView(new ArrayBuffer(256));
    }

    // ── Events ──────────────────────────────────────────────────────────

    getEventBuffer(): DataView {
        const battleIndex = this.resolveBattleIndex();
        if (battleIndex < 0) {
            return new DataView(new ArrayBuffer(0));
        }
        const buf = this.campaignBridge.getBattleEventBuffer(battleIndex);
        if (buf) return buf;
        return new DataView(new ArrayBuffer(0));
    }

    getEventCount(): number {
        const battleIndex = this.resolveBattleIndex();
        return battleIndex >= 0 ? this.campaignBridge.getBattleEventCount(battleIndex) : 0;
    }

    // ── Unit Commands ───────────────────────────────────────────────────

    cmdMoveUnit(entityId: number, targetX: number, targetY: number): void {
        const battleIndex = this.resolveBattleIndex();
        if (battleIndex >= 0) {
            this.campaignBridge.battleCmdMove(battleIndex, entityId, targetX, targetY);
        }
    }

    cmdMoveUnits(entityIds: number[], targetX: number, targetY: number): void {
        const battleIndex = this.resolveBattleIndex();
        if (battleIndex >= 0) {
            this.campaignBridge.battleCmdMoveUnits(battleIndex, entityIds, targetX, targetY);
        }
    }

    cmdStopUnit(entityId: number): void {
        const battleIndex = this.resolveBattleIndex();
        if (battleIndex >= 0) {
            this.campaignBridge.battleCmdStop(battleIndex, entityId);
        }
    }

    cmdAttack(entityId: number, targetId: number): void {
        const battleIndex = this.resolveBattleIndex();
        if (battleIndex >= 0) {
            this.campaignBridge.battleCmdAttack(battleIndex, entityId, targetId);
        }
    }

    cmdAttackMove(entityId: number, targetX: number, targetY: number): void {
        const battleIndex = this.resolveBattleIndex();
        if (battleIndex >= 0) {
            this.campaignBridge.battleCmdAttackMove(battleIndex, entityId, targetX, targetY);
        }
    }

    cmdAttackTarget(entityIds: number[], targetId: number): void {
        const battleIndex = this.resolveBattleIndex();
        if (battleIndex >= 0) {
            this.campaignBridge.battleCmdAttackTarget(battleIndex, entityIds, targetId);
        }
    }

    // ── Production (no-ops in campaign battle context) ───────────────────

    cmdProduce(_player: number, _unitType: number): void {
        /* no-op — campaign battle production is handled at campaign level */
    }

    cmdCancelProduction(_player: number, _lineIndex: number): void {
        /* no-op */
    }

    cmdSetRally(_player: number, _x: number, _y: number): void {
        /* no-op */
    }

    // ── Reinforcements ────────────────────────────────────────────────────

    /** Request reinforcements from the campaign Node garrison. */
    cmdReinforce(player: number, unitType: number, count: number): boolean {
        const battleIndex = this.resolveBattleIndex();
        return battleIndex >= 0
            ? this.campaignBridge.battleCmdReinforce(battleIndex, player, unitType, count)
            : false;
    }

    // ── Campaign Bridge (not part of GameBridge interface) ───────────────

    getCampaignBridge(): CampaignBridge {
        return this.campaignBridge;
    }

    getMemory(): WebAssembly.Memory {
        return this.memory;
    }

    private resolveBattleIndex(): number {
        return this.campaignBridge.getActiveBattles().findIndex(
            (battle) => battle.siteId === this.battleSiteId,
        );
    }
}

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
    private battleIndex: number;
    private memory: WebAssembly.Memory;

    constructor(campaignBridge: CampaignBridge, battleIndex: number, memory: WebAssembly.Memory) {
        this.campaignBridge = campaignBridge;
        this.battleIndex = battleIndex;
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
        const buf = this.campaignBridge.getBattleRenderBuffer(this.battleIndex);
        if (buf) return buf;
        // Return empty DataView if no buffer available
        return new DataView(new ArrayBuffer(0));
    }

    getRenderCount(): number {
        return this.campaignBridge.getBattleRenderCount(this.battleIndex);
    }

    // ── Map ─────────────────────────────────────────────────────────────

    getMapTile(x: number, y: number): { terrain: number; variant: number } {
        return this.campaignBridge.getBattleMapTile(this.battleIndex, x, y);
    }

    getMapWidth(): number {
        return this.campaignBridge.getBattleMapSize(this.battleIndex).width;
    }

    getMapHeight(): number {
        return this.campaignBridge.getBattleMapSize(this.battleIndex).height;
    }

    // ── Fog of War ──────────────────────────────────────────────────────

    getFogBuffer(player: number): Uint8Array {
        const buf = this.campaignBridge.getBattleFogBuffer(this.battleIndex, player);
        if (buf) return buf;
        return new Uint8Array(0);
    }

    getFogBufferLen(): number {
        return this.campaignBridge.getBattleFogLen(this.battleIndex);
    }

    // ── UI State ────────────────────────────────────────────────────────

    getUIState(): DataView {
        const state = this.campaignBridge.getBattleUIState(this.battleIndex);
        if (state) return state;
        return new DataView(new ArrayBuffer(256));
    }

    // ── Events ──────────────────────────────────────────────────────────

    getEventBuffer(): DataView {
        const buf = this.campaignBridge.getBattleEventBuffer(this.battleIndex);
        if (buf) return buf;
        return new DataView(new ArrayBuffer(0));
    }

    getEventCount(): number {
        return this.campaignBridge.getBattleEventCount(this.battleIndex);
    }

    // ── Unit Commands ───────────────────────────────────────────────────

    cmdMoveUnit(entityId: number, targetX: number, targetY: number): void {
        this.campaignBridge.battleCmdMove(this.battleIndex, entityId, targetX, targetY);
    }

    cmdMoveUnits(entityIds: number[], targetX: number, targetY: number): void {
        this.campaignBridge.battleCmdMoveUnits(this.battleIndex, entityIds, targetX, targetY);
    }

    cmdStopUnit(entityId: number): void {
        this.campaignBridge.battleCmdStop(this.battleIndex, entityId);
    }

    cmdAttack(entityId: number, targetId: number): void {
        this.campaignBridge.battleCmdAttack(this.battleIndex, entityId, targetId);
    }

    cmdAttackMove(entityId: number, targetX: number, targetY: number): void {
        this.campaignBridge.battleCmdAttackMove(this.battleIndex, entityId, targetX, targetY);
    }

    cmdAttackTarget(entityIds: number[], targetId: number): void {
        this.campaignBridge.battleCmdAttackTarget(this.battleIndex, entityIds, targetId);
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

    // ── Campaign Bridge (not part of GameBridge interface) ───────────────

    getCampaignBridge(): CampaignBridge {
        return this.campaignBridge;
    }

    getMemory(): WebAssembly.Memory {
        return this.memory;
    }
}

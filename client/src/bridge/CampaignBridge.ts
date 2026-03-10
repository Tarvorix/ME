/**
 * TypeScript bridge for campaign WASM exports.
 * Reads campaign byte buffers and exposes typed data to the client.
 *
 * All campaign WASM functions are prefixed with `campaign_` or `campaign_battle_`.
 * Buffer data is little-endian and read via DataView.
 */

import {
    campaign_init, campaign_tick,
    campaign_set_paused, campaign_is_paused,
    campaign_get_tick, campaign_add_ai, campaign_get_player_count,
    campaign_get_site_count, campaign_get_site_data_ptr, campaign_get_site_data_len,
    campaign_get_map_width, campaign_get_map_height, campaign_get_player_node,
    campaign_get_economy_ptr, campaign_get_economy_len,
    campaign_get_production_ptr, campaign_get_production_len,
    campaign_get_research_ptr, campaign_get_research_len,
    campaign_get_available_techs_ptr, campaign_get_available_techs_count,
    campaign_get_dispatch_orders_ptr, campaign_get_dispatch_orders_count,
    campaign_get_active_battle_count, campaign_get_active_battles_ptr,
    campaign_get_battle_render_ptr, campaign_get_battle_render_count,
    campaign_get_battle_event_ptr, campaign_get_battle_event_count,
    campaign_get_battle_fog_ptr, campaign_get_battle_fog_len,
    campaign_get_battle_map_width, campaign_get_battle_map_height,
    campaign_get_battle_map_tile, campaign_get_battle_ui_state_ptr,
    campaign_get_eliminated_players_ptr, campaign_get_eliminated_count,
    campaign_cmd_dispatch, campaign_cmd_research, campaign_cmd_produce, campaign_cmd_withdraw,
    campaign_battle_cmd_move, campaign_battle_cmd_attack, campaign_battle_cmd_attack_move,
    campaign_battle_cmd_move_units, campaign_battle_cmd_attack_target, campaign_battle_cmd_stop,
    campaign_battle_cmd_reinforce,
} from '../pkg/machine_empire_wasm.js';

import { RENDER_ENTRY_SIZE } from '../config';

import type {
    CampaignSiteData, CampaignEconomyData, CampaignProductionData, CampaignResearchData,
    DispatchOrderData, ActiveBattleData,
} from './CampaignTypes';

import {
    SiteType, BattleStatus,
    SITE_ENTRY_BYTES, ECONOMY_ENTRY_BYTES, RESEARCH_ENTRY_BYTES,
    DISPATCH_ENTRY_BYTES, BATTLE_ENTRY_BYTES,
    NEUTRAL_OWNER, NO_TECH, DISPATCH_FAILED,
} from './CampaignTypes';

export class CampaignBridge {
    private memory: WebAssembly.Memory;

    constructor(memory: WebAssembly.Memory) {
        this.memory = memory;
    }

    // ── Lifecycle ────────────────────────────────────────────────────────

    /** Initialize a new campaign game. */
    initCampaign(playerCount: number, seed: number): void {
        campaign_init(playerCount, seed);
    }

    /** Run one campaign tick (economy, research, dispatch, battles, AI). */
    tick(): void {
        campaign_tick();
    }

    /** Pause or unpause the campaign. */
    setPaused(paused: boolean): void {
        campaign_set_paused(paused ? 1 : 0);
    }

    /** Check if the campaign is paused. */
    isPaused(): boolean {
        return campaign_is_paused() !== 0;
    }

    /** Get the current campaign tick count. */
    getTickCount(): number {
        return campaign_get_tick();
    }

    /** Add an AI player. */
    addAiPlayer(playerId: number, difficulty: number): void {
        campaign_add_ai(playerId, difficulty);
    }

    /** Get the number of players in the campaign. */
    getPlayerCount(): number {
        return campaign_get_player_count();
    }

    // ── Map Queries ──────────────────────────────────────────────────────

    /** Get the number of sites on the campaign map. */
    getSiteCount(): number {
        return campaign_get_site_count();
    }

    /** Read all site data from the WASM buffer. */
    getSites(): CampaignSiteData[] {
        const ptr = campaign_get_site_data_ptr();
        const len = campaign_get_site_data_len();
        const count = campaign_get_site_count();

        if (ptr === 0 || len === 0) return [];

        const view = new DataView(this.memory.buffer, ptr, len);
        const sites: CampaignSiteData[] = [];

        for (let i = 0; i < count; i++) {
            const base = i * SITE_ENTRY_BYTES;
            sites.push({
                siteId: view.getUint32(base, true),
                siteType: view.getUint8(base + 4) as SiteType,
                owner: view.getUint8(base + 5),
                isContested: view.getUint8(base + 6) !== 0,
                garrisonCount: view.getUint8(base + 7),
                x: view.getFloat32(base + 8, true),
                y: view.getFloat32(base + 12, true),
                garrisonThralls: view.getUint32(base + 16, true),
                garrisonSentinels: view.getUint32(base + 20, true),
                garrisonTanks: view.getUint32(base + 24, true),
                battleId: view.getUint32(base + 28, true),
            });
        }

        return sites;
    }

    /** Get campaign map dimensions. */
    getMapSize(): { width: number; height: number } {
        return {
            width: campaign_get_map_width(),
            height: campaign_get_map_height(),
        };
    }

    /** Get the node site ID for a player. Returns -1 if invalid. */
    getPlayerNode(player: number): number {
        const id = campaign_get_player_node(player);
        return id === 0xFFFFFFFF ? -1 : id;
    }

    // ── Economy Queries ──────────────────────────────────────────────────

    /** Read economy data for a player. */
    getEconomy(player: number): CampaignEconomyData {
        const ptr = campaign_get_economy_ptr(player);
        const len = campaign_get_economy_len();

        if (ptr === 0) {
            return {
                energyBank: 0, nodeIncome: 0, mineIncome: 0, relicIncome: 0,
                totalIncome: 0, totalExpenses: 0, netRate: 0, strain: 0,
                garrisonUpkeep: 0, deployedUpkeep: 0,
            };
        }

        const view = new DataView(this.memory.buffer, ptr, len);
        return {
            energyBank: view.getFloat32(0, true),
            nodeIncome: view.getFloat32(4, true),
            mineIncome: view.getFloat32(8, true),
            relicIncome: view.getFloat32(12, true),
            totalIncome: view.getFloat32(16, true),
            totalExpenses: view.getFloat32(20, true),
            netRate: view.getFloat32(24, true),
            strain: view.getFloat32(28, true),
            garrisonUpkeep: view.getFloat32(32, true),
            deployedUpkeep: view.getFloat32(36, true),
        };
    }

    /** Read campaign production queue state for a player. */
    getProduction(player: number): CampaignProductionData {
        const ptr = campaign_get_production_ptr(player);
        const len = campaign_get_production_len();

        if (ptr === 0 || len === 0) {
            return {
                activeUnitType: 255,
                activeProgress: 0,
                activeTotalTime: 0,
                queuedCount: 0,
                queuedThralls: 0,
                queuedSentinels: 0,
                queuedTanks: 0,
            };
        }

        const view = new DataView(this.memory.buffer, ptr, len);
        return {
            activeUnitType: view.getUint8(0),
            activeProgress: view.getFloat32(1, true),
            activeTotalTime: view.getFloat32(5, true),
            queuedCount: view.getUint32(9, true),
            queuedThralls: view.getUint32(13, true),
            queuedSentinels: view.getUint32(17, true),
            queuedTanks: view.getUint32(21, true),
        };
    }

    // ── Research Queries ─────────────────────────────────────────────────

    /** Read research state for a player. */
    getResearch(player: number): CampaignResearchData {
        const ptr = campaign_get_research_ptr(player);
        const len = campaign_get_research_len();

        if (ptr === 0) {
            return {
                activeTechId: NO_TECH, activeProgress: 0, activeTotalTime: 0,
                completedCount: 0, completedTechs: [],
            };
        }

        const view = new DataView(this.memory.buffer, ptr, len);
        const activeTechId = view.getUint8(0);
        const activeProgress = view.getFloat32(1, true);
        const activeTotalTime = view.getFloat32(5, true);
        const completedCount = view.getUint8(9);

        const completedTechs: number[] = [];
        for (let i = 0; i < completedCount && i < 12; i++) {
            const techId = view.getUint8(10 + i);
            if (techId !== NO_TECH) {
                completedTechs.push(techId);
            }
        }

        return {
            activeTechId,
            activeProgress,
            activeTotalTime,
            completedCount,
            completedTechs,
        };
    }

    /** Get tech IDs currently available for research. */
    getAvailableTechs(player: number): number[] {
        const ptr = campaign_get_available_techs_ptr(player);
        const count = campaign_get_available_techs_count(player);

        if (ptr === 0 || count === 0) return [];

        const view = new Uint8Array(this.memory.buffer, ptr, 12);
        const techs: number[] = [];
        for (let i = 0; i < count && i < 12; i++) {
            if (view[i] !== NO_TECH) {
                techs.push(view[i]);
            }
        }

        return techs;
    }

    // ── Dispatch Queries ─────────────────────────────────────────────────

    /** Read all active dispatch orders. */
    getDispatchOrders(): DispatchOrderData[] {
        const ptr = campaign_get_dispatch_orders_ptr();
        const count = campaign_get_dispatch_orders_count();

        if (ptr === 0 || count === 0) return [];

        const view = new DataView(this.memory.buffer, ptr, count * DISPATCH_ENTRY_BYTES);
        const orders: DispatchOrderData[] = [];

        for (let i = 0; i < count; i++) {
            const base = i * DISPATCH_ENTRY_BYTES;
            orders.push({
                orderId: view.getUint32(base, true),
                player: view.getUint8(base + 4),
                sourceSite: view.getUint32(base + 8, true),
                targetSite: view.getUint32(base + 12, true),
                travelRemaining: view.getFloat32(base + 16, true),
                totalTime: view.getFloat32(base + 20, true),
                unitCount: view.getUint32(base + 24, true),
            });
        }

        return orders;
    }

    // ── Battle Queries ───────────────────────────────────────────────────

    /** Get the number of active battles. */
    getActiveBattleCount(): number {
        return campaign_get_active_battle_count();
    }

    /** Read all active battle data. */
    getActiveBattles(): ActiveBattleData[] {
        const count = campaign_get_active_battle_count();
        if (count === 0) return [];

        const ptr = campaign_get_active_battles_ptr();
        if (ptr === 0) return [];

        const view = new DataView(this.memory.buffer, ptr, count * BATTLE_ENTRY_BYTES);
        const battles: ActiveBattleData[] = [];

        for (let i = 0; i < count; i++) {
            const base = i * BATTLE_ENTRY_BYTES;
            battles.push({
                siteId: view.getUint32(base, true),
                attacker: view.getUint8(base + 4),
                defender: view.getUint8(base + 5),
                status: view.getUint8(base + 6) as BattleStatus,
                winner: view.getUint8(base + 7),
                tickCount: view.getUint32(base + 8, true),
            });
        }

        return battles;
    }

    /** Get the render buffer for a campaign battle's RTS game. */
    getBattleRenderBuffer(battleIndex: number): DataView | null {
        const ptr = campaign_get_battle_render_ptr(battleIndex);
        const count = campaign_get_battle_render_count(battleIndex);
        if (ptr === 0 || count === 0) return null;
        return new DataView(this.memory.buffer, ptr, count * RENDER_ENTRY_SIZE);
    }

    /** Get the render entity count for a campaign battle. */
    getBattleRenderCount(battleIndex: number): number {
        return campaign_get_battle_render_count(battleIndex);
    }

    /** Get the event buffer for a campaign battle. */
    getBattleEventBuffer(battleIndex: number): DataView | null {
        const ptr = campaign_get_battle_event_ptr(battleIndex);
        const count = campaign_get_battle_event_count(battleIndex);
        if (ptr === 0 || count === 0) return null;
        return new DataView(this.memory.buffer, ptr, count * 32);
    }

    /** Get the event count for a campaign battle. */
    getBattleEventCount(battleIndex: number): number {
        return campaign_get_battle_event_count(battleIndex);
    }

    /** Get the fog buffer for a player in a campaign battle. */
    getBattleFogBuffer(battleIndex: number, player: number): Uint8Array | null {
        const ptr = campaign_get_battle_fog_ptr(battleIndex, player);
        const len = campaign_get_battle_fog_len(battleIndex);
        if (ptr === 0 || len === 0) return null;
        return new Uint8Array(this.memory.buffer, ptr, len);
    }

    /** Get the fog buffer length for a campaign battle. */
    getBattleFogLen(battleIndex: number): number {
        return campaign_get_battle_fog_len(battleIndex);
    }

    /** Get the map dimensions for a campaign battle. */
    getBattleMapSize(battleIndex: number): { width: number; height: number } {
        return {
            width: campaign_get_battle_map_width(battleIndex),
            height: campaign_get_battle_map_height(battleIndex),
        };
    }

    /** Get a map tile from a campaign battle. */
    getBattleMapTile(battleIndex: number, x: number, y: number): { terrain: number; variant: number } {
        const packed = campaign_get_battle_map_tile(battleIndex, x, y);
        return { terrain: packed & 0xFF, variant: (packed >> 8) & 0xFF };
    }

    /** Get the UI state buffer for a campaign battle. */
    getBattleUIState(battleIndex: number): DataView | null {
        const ptr = campaign_get_battle_ui_state_ptr(battleIndex);
        if (ptr === 0) return null;
        return new DataView(this.memory.buffer, ptr, 256);
    }

    /** Get eliminated player IDs. */
    getEliminatedPlayers(): number[] {
        const count = campaign_get_eliminated_count();
        if (count === 0) return [];

        const ptr = campaign_get_eliminated_players_ptr();
        if (ptr === 0) return [];

        const view = new Uint8Array(this.memory.buffer, ptr, 4);
        const eliminated: number[] = [];
        for (let i = 0; i < count && i < 4; i++) {
            if (view[i] !== NEUTRAL_OWNER) {
                eliminated.push(view[i]);
            }
        }

        return eliminated;
    }

    // ── Campaign Commands ────────────────────────────────────────────────

    /**
     * Dispatch units from source site to target site.
     * @param units Array of {unitType, count} pairs.
     * @returns Order ID on success, or -1 on failure.
     */
    cmdDispatch(
        player: number,
        sourceSite: number,
        targetSite: number,
        units: Array<{ unitType: number; count: number }>,
    ): number {
        // Pack units into a flat Uint32Array: [type0, count0, type1, count1, ...]
        // wasm_bindgen handles &[u32] slice parameters natively
        const data = new Uint32Array(units.length * 2);
        for (let i = 0; i < units.length; i++) {
            data[i * 2] = units[i].unitType;
            data[i * 2 + 1] = units[i].count;
        }

        const result = campaign_cmd_dispatch(player, sourceSite, targetSite, data);
        return result === DISPATCH_FAILED ? -1 : result;
    }

    /**
     * Start researching a technology.
     * @param techId TechId enum value (0-11).
     * @returns true on success.
     */
    cmdResearch(player: number, techId: number): boolean {
        return campaign_cmd_research(player, techId) !== 0;
    }

    /**
     * Produce units at the player's node.
     * @param unitType 0=Thrall, 1=Sentinel, 2=HoverTank.
     * @returns true on success.
     */
    cmdProduce(player: number, unitType: number, count: number): boolean {
        return campaign_cmd_produce(player, unitType, count) !== 0;
    }

    /**
     * Withdraw all garrison from a site back to the player's node.
     * @returns true on success.
     */
    cmdWithdraw(player: number, siteId: number): boolean {
        return campaign_cmd_withdraw(player, siteId) !== 0;
    }

    // ── Battle Commands ──────────────────────────────────────────────────

    /** Move a unit in a campaign battle. */
    battleCmdMove(battleIndex: number, entityId: number, targetX: number, targetY: number): void {
        campaign_battle_cmd_move(battleIndex, entityId, targetX, targetY);
    }

    /** Attack a target in a campaign battle. */
    battleCmdAttack(battleIndex: number, entityId: number, targetId: number): void {
        campaign_battle_cmd_attack(battleIndex, entityId, targetId);
    }

    /** Attack-move in a campaign battle. */
    battleCmdAttackMove(battleIndex: number, entityId: number, targetX: number, targetY: number): void {
        campaign_battle_cmd_attack_move(battleIndex, entityId, targetX, targetY);
    }

    /** Move multiple units in a campaign battle. */
    battleCmdMoveUnits(battleIndex: number, entityIds: number[], targetX: number, targetY: number): void {
        // wasm_bindgen handles &[u32] slice parameters natively
        campaign_battle_cmd_move_units(battleIndex, new Uint32Array(entityIds), targetX, targetY);
    }

    /** Attack with multiple units in a campaign battle. */
    battleCmdAttackTarget(battleIndex: number, entityIds: number[], targetId: number): void {
        campaign_battle_cmd_attack_target(battleIndex, new Uint32Array(entityIds), targetId);
    }

    /** Stop a unit in a campaign battle. */
    battleCmdStop(battleIndex: number, entityId: number): void {
        campaign_battle_cmd_stop(battleIndex, entityId);
    }

    /**
     * Request reinforcements in a campaign battle.
     * Units are drawn from the player's Node garrison.
     * @param unitType 0=Thrall, 1=Sentinel, 2=HoverTank.
     * @returns true on success.
     */
    battleCmdReinforce(battleIndex: number, player: number, unitType: number, count: number): boolean {
        return campaign_battle_cmd_reinforce(battleIndex, player, unitType, count) !== 0;
    }
}

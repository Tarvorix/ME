import init, {
    init_game, tick,
    get_render_buffer_ptr, get_render_count,
    get_event_buffer_ptr, get_event_count,
    get_fog_buffer_ptr, get_fog_buffer_len,
    get_ui_state_ptr, get_ui_state_len,
    get_map_width, get_map_height, get_map_tile,
    cmd_move_unit, cmd_stop_unit,
    cmd_attack, cmd_attack_move,
    cmd_produce, cmd_cancel_production, cmd_set_rally,
} from '../pkg/machine_empire_wasm.js';
import { RENDER_ENTRY_SIZE } from '../config';
import { CampaignBridge } from './CampaignBridge';

export class GameBridge {
    private memory!: WebAssembly.Memory;
    private initialized = false;
    private _campaignBridge: CampaignBridge | null = null;

    async init(): Promise<void> {
        const wasm = await init();
        this.memory = wasm.memory;
        this.initialized = true;
        this._campaignBridge = new CampaignBridge(this.memory);
    }

    /** Get the campaign bridge (shares the same WASM module). */
    getCampaignBridge(): CampaignBridge {
        if (!this._campaignBridge) {
            throw new Error('GameBridge not initialized. Call init() first.');
        }
        return this._campaignBridge;
    }

    startGame(mapWidth: number, mapHeight: number, playerCount: number, seed: number): void {
        init_game(mapWidth, mapHeight, playerCount, seed);
    }

    tick(deltaMs: number): void {
        tick(deltaMs);
    }

    getRenderBuffer(): DataView {
        const ptr = get_render_buffer_ptr();
        const count = get_render_count();
        const byteLength = count * RENDER_ENTRY_SIZE;
        return new DataView(this.memory.buffer, ptr, byteLength);
    }

    getRenderCount(): number {
        return get_render_count();
    }

    getMapTile(x: number, y: number): { terrain: number; variant: number } {
        const packed = get_map_tile(x, y);
        return { terrain: packed & 0xFF, variant: (packed >> 8) & 0xFF };
    }

    getMapWidth(): number { return get_map_width(); }
    getMapHeight(): number { return get_map_height(); }

    cmdMoveUnit(entityId: number, targetX: number, targetY: number): void {
        cmd_move_unit(entityId, targetX, targetY);
    }

    cmdMoveUnits(entityIds: number[], targetX: number, targetY: number): void {
        for (const id of entityIds) {
            cmd_move_unit(id, targetX, targetY);
        }
    }

    cmdStopUnit(entityId: number): void {
        cmd_stop_unit(entityId);
    }

    cmdAttack(entityId: number, targetId: number): void {
        cmd_attack(entityId, targetId);
    }

    cmdAttackMove(entityId: number, targetX: number, targetY: number): void {
        cmd_attack_move(entityId, targetX, targetY);
    }

    cmdAttackTarget(entityIds: number[], targetId: number): void {
        for (const id of entityIds) {
            cmd_attack(id, targetId);
        }
    }

    cmdProduce(player: number, unitType: number): void {
        cmd_produce(player, unitType);
    }

    cmdCancelProduction(player: number, lineIndex: number): void {
        cmd_cancel_production(player, lineIndex);
    }

    cmdSetRally(player: number, x: number, y: number): void {
        cmd_set_rally(player, x, y);
    }

    getFogBuffer(player: number): Uint8Array {
        const ptr = get_fog_buffer_ptr(player);
        const len = get_fog_buffer_len();
        return new Uint8Array(this.memory.buffer, ptr, len);
    }

    getFogBufferLen(): number {
        return get_fog_buffer_len();
    }

    getUIState(): DataView {
        const ptr = get_ui_state_ptr();
        const len = get_ui_state_len();
        return new DataView(this.memory.buffer, ptr, len);
    }

    getEventBuffer(): DataView {
        const ptr = get_event_buffer_ptr();
        const count = get_event_count();
        const byteLength = count * 32; // 32 bytes per event
        return new DataView(this.memory.buffer, ptr, byteLength);
    }

    getEventCount(): number {
        return get_event_count();
    }

    getMemory(): WebAssembly.Memory {
        return this.memory;
    }
}

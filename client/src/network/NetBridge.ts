import { GameSocket, ConnectionState } from './Socket';
import { RENDER_ENTRY_SIZE, EVENT_ENTRY_SIZE } from '../config';

/**
 * Parsed entity data from server state.
 */
export interface NetEntity {
    id: number;
    x: number;
    y: number;
    spriteId: number;
    direction: number;
    animState: number;
    animFrame: number;
    healthPct: number;
    scale: number;
    owner: number;
}

/**
 * Parsed event data from server state.
 */
export interface NetEvent {
    eventType: number;
    entityId: number;
    x: number;
    y: number;
    payload: Uint8Array;
}

/**
 * Parsed economy data from server state.
 */
export interface NetEconomy {
    energyBank: number;
    income: number;
    expense: number;
    strain: number;
}

/**
 * Parsed production line data from server state.
 */
export interface NetProductionLine {
    unitType: number;
    progress: number;
    totalTime: number;
}

/**
 * Parsed capture point data from server state.
 */
export interface NetCapturePoint {
    entityId: number;
    x: number;
    y: number;
    pointIndex: number;
    owner: number;
    progress: number;
    capturingPlayer: number;
    contested: boolean;
}

/**
 * Network bridge for receiving server state and sending commands.
 * Acts as an alternative to GameBridge for online mode.
 */
export class NetBridge {
    private socket: GameSocket;
    private entities: NetEntity[] = [];
    private events: NetEvent[] = [];
    private economy: NetEconomy = { energyBank: 0, income: 0, expense: 0, strain: 0 };
    private productionLines: NetProductionLine[] = [];
    private capturePoints: NetCapturePoint[] = [];
    private fogData: Uint8Array = new Uint8Array(0);
    private mapWidth = 64;
    private mapHeight = 64;
    private mapTiles: number[] = [];
    private playerId = 0;
    private gameTick = 0;
    private connected = false;

    // Render buffer for compatibility with GameBridge interface
    private renderBuffer: ArrayBuffer;
    private renderView: DataView;
    private renderCount = 0;

    // Event buffer for compatibility
    private eventBuffer: ArrayBuffer;
    private eventView: DataView;
    private eventCount = 0;

    // UI state buffer for compatibility
    private uiBuffer: ArrayBuffer;
    private uiView: DataView;

    constructor(serverUrl: string) {
        this.renderBuffer = new ArrayBuffer(2048 * RENDER_ENTRY_SIZE);
        this.renderView = new DataView(this.renderBuffer);
        this.eventBuffer = new ArrayBuffer(256 * EVENT_ENTRY_SIZE);
        this.eventView = new DataView(this.eventBuffer);
        this.uiBuffer = new ArrayBuffer(256);
        this.uiView = new DataView(this.uiBuffer);

        this.socket = new GameSocket({
            url: serverUrl,
            onMessage: (data) => this.handleServerMessage(data),
            onStateChange: (state) => this.handleStateChange(state),
        });
    }

    /** Connect to the game server. */
    connect(): void {
        this.socket.connect();
    }

    /** Disconnect from the server. */
    disconnect(): void {
        this.socket.disconnect();
    }

    /** Whether connected to server. */
    isConnected(): boolean {
        return this.connected;
    }

    /** Get current network latency in ms. */
    getLatency(): number {
        return this.socket.getLatency();
    }

    /** Set the local player ID. */
    setPlayerId(id: number): void {
        this.playerId = id;
    }

    // ── GameBridge-compatible interface ──────────────────────────────────────

    /** No-op for network mode (server controls tick). */
    tick(_deltaMs: number): void {
        // Server drives the simulation; client just renders latest state
    }

    getRenderBuffer(): DataView {
        return new DataView(this.renderBuffer, 0, this.renderCount * RENDER_ENTRY_SIZE);
    }

    getRenderCount(): number {
        return this.renderCount;
    }

    getEventBuffer(): DataView {
        return new DataView(this.eventBuffer, 0, this.eventCount * EVENT_ENTRY_SIZE);
    }

    getEventCount(): number {
        return this.eventCount;
    }

    getUIState(): DataView {
        return this.uiView;
    }

    getMapWidth(): number {
        return this.mapWidth;
    }

    getMapHeight(): number {
        return this.mapHeight;
    }

    getMapTile(x: number, y: number): number {
        const idx = y * this.mapWidth + x;
        return this.mapTiles[idx] ?? 0;
    }

    getFogBuffer(): Uint8Array {
        return this.fogData;
    }

    // ── Command methods ─────────────────────────────────────────────────────

    cmdMoveUnits(unitIds: number[], targetX: number, targetY: number): void {
        this.sendCommand({ Move: { unit_ids: unitIds, target_x: targetX, target_y: targetY } });
    }

    cmdStopUnits(unitIds: number[]): void {
        this.sendCommand({ Stop: { unit_ids: unitIds } });
    }

    cmdAttack(unitIds: number[], targetId: number): void {
        this.sendCommand({ Attack: { unit_ids: unitIds, target_id: targetId } });
    }

    cmdAttackMove(unitIds: number[], targetX: number, targetY: number): void {
        this.sendCommand({ AttackMove: { unit_ids: unitIds, target_x: targetX, target_y: targetY } });
    }

    cmdProduce(unitType: number): void {
        this.sendCommand({ Produce: { player: this.playerId, unit_type: unitType } });
    }

    cmdCancelProduction(lineIndex: number): void {
        this.sendCommand({ CancelProduction: { player: this.playerId, line_index: lineIndex } });
    }

    cmdSetRally(x: number, y: number): void {
        this.sendCommand({ SetRally: { player: this.playerId, x, y } });
    }

    // ── Internal ────────────────────────────────────────────────────────────

    private sendCommand(cmd: unknown): void {
        this.socket.send({ Cmd: { cmd } });
    }

    private handleStateChange(state: ConnectionState): void {
        this.connected = state === 'connected';
    }

    private handleServerMessage(data: unknown): void {
        if (!data || typeof data !== 'object') return;

        const msg = data as Record<string, unknown>;

        if ('State' in msg) {
            this.handleStateUpdate(msg.State as Record<string, unknown>);
        } else if ('FullState' in msg) {
            this.handleFullState(msg.FullState as Record<string, unknown>);
        }
    }

    private handleStateUpdate(state: Record<string, unknown>): void {
        // Parse entities
        if (Array.isArray(state.entities)) {
            this.entities = state.entities.map(this.parseEntity);
            this.writeEntitiesToRenderBuffer();
        }

        // Parse events
        if (Array.isArray(state.events)) {
            this.events = state.events.map(this.parseEvent);
            this.writeEventsToEventBuffer();
        }

        // Parse economy
        if (state.economy && typeof state.economy === 'object') {
            const econ = state.economy as Record<string, number>;
            this.economy = {
                energyBank: econ.energy_bank ?? 0,
                income: econ.income ?? 0,
                expense: econ.expense ?? 0,
                strain: econ.strain ?? 0,
            };
            this.writeEconomyToUIBuffer();
        }

        // Parse production
        if (Array.isArray(state.production)) {
            this.productionLines = state.production.map((p: Record<string, number>) => ({
                unitType: p.unit_type ?? 0,
                progress: p.progress ?? 0,
                totalTime: p.total_time ?? 0,
            }));
        }

        // Parse capture points
        if (Array.isArray(state.capture_points)) {
            this.capturePoints = state.capture_points.map((cp: Record<string, unknown>) => ({
                entityId: (cp.entity_id as number) ?? 0,
                x: (cp.x as number) ?? 0,
                y: (cp.y as number) ?? 0,
                pointIndex: (cp.point_index as number) ?? 0,
                owner: (cp.owner as number) ?? 255,
                progress: (cp.progress as number) ?? 0,
                capturingPlayer: (cp.capturing_player as number) ?? 255,
                contested: (cp.contested as boolean) ?? false,
            }));
        }

        // Parse fog
        if (state.fog && state.fog instanceof Uint8Array) {
            this.fogData = state.fog;
        }

        this.gameTick = (state.tick as number) ?? this.gameTick;
    }

    private handleFullState(state: Record<string, unknown>): void {
        // Map data
        if (state.map_width) this.mapWidth = state.map_width as number;
        if (state.map_height) this.mapHeight = state.map_height as number;
        if (Array.isArray(state.map_tiles)) {
            this.mapTiles = state.map_tiles as number[];
        }

        // Then process regular state data
        this.handleStateUpdate(state);
    }

    private parseEntity(e: Record<string, unknown>): NetEntity {
        return {
            id: (e.entity_id as number) ?? 0,
            x: (e.x as number) ?? 0,
            y: (e.y as number) ?? 0,
            spriteId: (e.sprite_id as number) ?? 0,
            direction: (e.direction as number) ?? 0,
            animState: (e.anim_state as number) ?? 0,
            animFrame: (e.anim_frame as number) ?? 0,
            healthPct: (e.health_pct as number) ?? 100,
            scale: (e.scale as number) ?? 1.0,
            owner: (e.owner as number) ?? 0,
        };
    }

    private parseEvent(e: Record<string, unknown>): NetEvent {
        return {
            eventType: (e.event_type as number) ?? 0,
            entityId: (e.entity_id as number) ?? 0,
            x: (e.x as number) ?? 0,
            y: (e.y as number) ?? 0,
            payload: (e.payload as Uint8Array) ?? new Uint8Array(16),
        };
    }

    private writeEntitiesToRenderBuffer(): void {
        this.renderCount = this.entities.length;
        const view = new DataView(this.renderBuffer);

        for (let i = 0; i < this.entities.length && i < 2048; i++) {
            const e = this.entities[i];
            const off = i * RENDER_ENTRY_SIZE;

            view.setUint32(off + 0, e.id, true);           // entity_id
            view.setFloat32(off + 4, e.x, true);            // x
            view.setFloat32(off + 8, e.y, true);            // y
            view.setUint8(off + 12, e.spriteId);             // sprite_id
            view.setUint8(off + 13, e.direction);            // direction
            view.setUint8(off + 14, e.animState);            // anim_state
            view.setUint8(off + 15, e.animFrame);            // anim_frame
            view.setUint8(off + 16, e.healthPct);            // health_pct
            view.setFloat32(off + 20, e.scale, true);        // scale
            view.setUint8(off + 18, e.owner);                // owner
        }
    }

    private writeEventsToEventBuffer(): void {
        this.eventCount = this.events.length;
        const view = new DataView(this.eventBuffer);

        for (let i = 0; i < this.events.length && i < 256; i++) {
            const e = this.events[i];
            const off = i * EVENT_ENTRY_SIZE;

            view.setUint16(off + 0, e.eventType, true);     // event_type
            view.setUint16(off + 2, 0, true);                // reserved
            view.setUint32(off + 4, e.entityId, true);       // entity_id
            view.setFloat32(off + 8, e.x, true);             // x
            view.setFloat32(off + 12, e.y, true);            // y

            // Copy payload
            for (let j = 0; j < 16 && j < e.payload.length; j++) {
                view.setUint8(off + 16 + j, e.payload[j]);
            }
        }
    }

    private writeEconomyToUIBuffer(): void {
        const view = this.uiView;
        view.setFloat32(0, this.economy.energyBank, true);
        view.setFloat32(4, this.economy.income, true);
        view.setFloat32(8, this.economy.expense, true);
        view.setFloat32(12, this.economy.strain, true);
        view.setUint32(20, this.gameTick, true);
    }
}

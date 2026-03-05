import type { RenderEntry, GameEvent } from './types';

export class BufferReader {
    static readRenderEntry(view: DataView, index: number): RenderEntry {
        const off = index * 32;
        return {
            entityId: view.getUint32(off, true),
            x: view.getFloat32(off + 4, true),
            y: view.getFloat32(off + 8, true),
            spriteId: view.getUint16(off + 12, true),
            frame: view.getUint16(off + 14, true),
            healthPct: view.getUint8(off + 16),
            facing: view.getUint8(off + 17),
            owner: view.getUint8(off + 18),
            flags: view.getUint8(off + 19),
            scale: view.getFloat32(off + 20, true),
            zOrder: view.getFloat32(off + 24, true),
        };
    }

    /**
     * Read a single event from the event buffer.
     * Layout: [0-1] event_type u16, [2-3] reserved, [4-7] entity_id u32,
     *         [8-11] x f32, [12-15] y f32, [16-31] payload (16 bytes)
     */
    static readEvent(view: DataView, index: number): GameEvent {
        const off = index * 32;
        return {
            eventType: view.getUint16(off, true),
            entityId: view.getUint32(off + 4, true),
            x: view.getFloat32(off + 8, true),
            y: view.getFloat32(off + 12, true),
            payload: new DataView(view.buffer, view.byteOffset + off + 16, 16),
        };
    }
}

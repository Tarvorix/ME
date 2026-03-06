import { render } from 'preact';
import { html } from 'htm/preact';
import { ResourceBar } from './ResourceBar';
import { SelectionPanel } from './SelectionPanel';
import { BuildMenu } from './BuildMenu';
import type { SelectionInfo } from './SelectionPanel';
import type { ProductionLineInfo } from './BuildMenu';
import type { GameBridge } from '../bridge/GameBridge';
import { readUIState } from '../bridge/types';
import type { UIState } from '../bridge/types';

/**
 * Main HUD orchestrator: renders Preact components into the #hud-root overlay.
 * Updates from the WASM UIStateBuffer each frame.
 */
export class HUD {
    private root: HTMLElement;
    private bridge: GameBridge;
    private selectionInfo: SelectionInfo = { count: 0 };
    private lastUIState: UIState = {
        energy: 0, income: 0, expense: 0, strain: 0, strainPenalty: 0, gameTick: 0,
    };

    constructor(bridge: GameBridge) {
        this.bridge = bridge;
        this.root = document.getElementById('hud-root')!;
    }

    /** Call once per frame to update HUD with latest game state. */
    update(selectedEntities: Set<number>): void {
        // Read UI state from WASM
        const uiView = this.bridge.getUIState();
        this.lastUIState = readUIState(uiView);

        // Build selection info
        this.selectionInfo = this.buildSelectionInfo(selectedEntities);

        // Read production lines from UI state buffer bytes [68-195]
        const productionLines = this.readProductionLines(uiView);

        // Render
        render(
            html`
                <${ResourceBar} state=${this.lastUIState} />
                <${SelectionPanel} info=${this.selectionInfo} />
                <${BuildMenu}
                    lines=${productionLines}
                    onProduce=${(unitType: number) => this.bridge.cmdProduce(0, unitType)}
                    onCancel=${(lineIndex: number) => this.bridge.cmdCancelProduction(0, lineIndex)}
                />
            `,
            this.root,
        );
    }

    private buildSelectionInfo(selectedEntities: Set<number>): SelectionInfo {
        const count = selectedEntities.size;
        if (count === 0) return { count: 0 };

        if (count === 1) {
            const entityId = Array.from(selectedEntities)[0];
            // Read from render buffer to get details
            const renderCount = this.bridge.getRenderCount();
            if (renderCount > 0) {
                const view = this.bridge.getRenderBuffer();
                for (let i = 0; i < renderCount; i++) {
                    const off = i * 32;
                    const id = view.getUint32(off, true);
                    if (id === entityId) {
                        const spriteId = view.getUint16(off + 12, true);
                        const healthPct = view.getUint8(off + 16);
                        const name = this.getUnitName(spriteId);
                        const maxHp = this.getMaxHp(spriteId);
                        return {
                            count: 1,
                            singleName: name,
                            singleHp: maxHp * healthPct / 100,
                            singleMaxHp: maxHp,
                        };
                    }
                }
            }
            return { count: 1, singleName: 'Unknown' };
        }

        // Multi-selection: count by type
        const typeCounts = new Map<number, number>();
        const renderCount = this.bridge.getRenderCount();
        if (renderCount > 0) {
            const view = this.bridge.getRenderBuffer();
            for (let i = 0; i < renderCount; i++) {
                const off = i * 32;
                const id = view.getUint32(off, true);
                if (selectedEntities.has(id)) {
                    const spriteId = view.getUint16(off + 12, true);
                    typeCounts.set(spriteId, (typeCounts.get(spriteId) ?? 0) + 1);
                }
            }
        }

        return { count, typeCounts };
    }

    private readProductionLines(uiView: DataView): ProductionLineInfo[] {
        const lines: ProductionLineInfo[] = [];
        for (let i = 0; i < 8; i++) {
            const offset = 68 + i * 16;
            if (offset + 10 > uiView.byteLength) break;
            const unitType = uiView.getUint16(offset, true);
            const progress = uiView.getFloat32(offset + 2, true);
            const totalTime = uiView.getFloat32(offset + 6, true);
            if (totalTime > 0) {
                lines.push({ unitType, progress, totalTime });
            }
        }
        return lines;
    }

    private getUnitName(spriteId: number): string {
        switch (spriteId) {
            case 0: return 'Thrall';
            case 1: return 'Sentinel';
            case 2: return 'Hover Tank';
            case 3: return 'Command Post';
            case 4: return 'Node';
            default: return `Unit ${spriteId}`;
        }
    }

    /** Clear the HUD overlay. */
    destroy(): void {
        render(null, this.root);
    }

    private getMaxHp(spriteId: number): number {
        switch (spriteId) {
            case 0: return 80;
            case 1: return 200;
            case 2: return 500;
            case 3: return 800;
            case 4: return 2000;
            default: return 100;
        }
    }
}

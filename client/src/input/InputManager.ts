import { SelectionIndicator } from './SelectionIndicator';
import { MoveOrderIndicator } from './MoveOrderIndicator';
import { SelectionBox } from './SelectionBox';
import { TouchHandler } from './TouchHandler';
import type { CameraController } from '../render/CameraController';
import type { SpritePool } from '../render/SpritePool';
import type { GameBridge } from '../bridge/GameBridge';
import { screenToTile } from '../render/IsoUtils';

/**
 * Handles user input: selection (single, multi, box), movement, touch gestures.
 * Left-click: select, Shift+click: toggle, Drag: selection box.
 * Right-click / touch long-press: move order for all selected.
 */
export class InputManager {
    private selectedEntities = new Set<number>();
    private selectionIndicator: SelectionIndicator;
    private moveIndicator: MoveOrderIndicator;
    private selectionBox: SelectionBox;
    private touchHandler: TouchHandler;
    private isDragSelecting = false;

    constructor(
        private canvas: HTMLCanvasElement,
        private camera: CameraController,
        private spritePool: SpritePool,
        private bridge: GameBridge,
    ) {
        this.selectionIndicator = new SelectionIndicator(spritePool);
        this.moveIndicator = new MoveOrderIndicator(spritePool);
        this.selectionBox = new SelectionBox(spritePool);

        // Desktop mouse input
        this.canvas.addEventListener('pointerdown', this.onPointerDown);

        // Touch input
        this.touchHandler = new TouchHandler(this.canvas, {
            onTapSelect: (sx, sy) => {
                const { wx, wy } = this.camera.screenToWorld(sx, sy);
                const entityId = this.spritePool.getEntityAtScreen(wx, wy);
                this.selectedEntities.clear();
                if (entityId !== null) {
                    this.selectedEntities.add(entityId);
                }
            },
            onDragStart: (sx, sy) => {
                const { wx, wy } = this.camera.screenToWorld(sx, sy);
                this.selectionBox.start(wx, wy);
                this.isDragSelecting = true;
            },
            onDragMove: (sx, sy) => {
                const { wx, wy } = this.camera.screenToWorld(sx, sy);
                this.selectionBox.move(wx, wy);
            },
            onDragEnd: (_sx, _sy) => {
                const ids = this.selectionBox.end();
                if (ids.length > 0) {
                    this.selectedEntities.clear();
                    for (const id of ids) {
                        this.selectedEntities.add(id);
                    }
                }
                this.isDragSelecting = false;
            },
            onPan: (dx, dy) => {
                this.camera.pan(dx, dy);
            },
            onZoom: (delta, cx, cy) => {
                this.camera.zoomAt(delta, cx, cy);
            },
            onLongPress: (sx, sy) => {
                // Long press = move order (like right-click)
                this.issueMoveOrder(sx, sy);
            },
        });
    }

    update(): void {
        this.selectionIndicator.update(this.selectedEntities);
        this.moveIndicator.update();
    }

    getSelectedEntities(): Set<number> {
        return this.selectedEntities;
    }

    destroy(): void {
        this.canvas.removeEventListener('pointerdown', this.onPointerDown);
        this.touchHandler.destroy();
    }

    private onPointerDown = (e: PointerEvent): void => {
        // Ignore touch events (handled by TouchHandler)
        if (e.pointerType === 'touch') return;

        if (e.button === 0) {
            // Left-click: select or start drag-select
            const { wx, wy } = this.camera.screenToWorld(e.clientX, e.clientY);
            const entityId = this.spritePool.getEntityAtScreen(wx, wy);

            if (e.shiftKey && entityId !== null) {
                // Shift+click: toggle entity in selection
                if (this.selectedEntities.has(entityId)) {
                    this.selectedEntities.delete(entityId);
                } else {
                    this.selectedEntities.add(entityId);
                }
            } else if (entityId !== null) {
                // Single click on entity: select it
                this.selectedEntities.clear();
                this.selectedEntities.add(entityId);
            } else {
                // Click on empty ground: start drag-select or deselect
                this.startDragSelect(e);
            }
        } else if (e.button === 2) {
            // Right-click: issue move order
            this.issueMoveOrder(e.clientX, e.clientY);
        }
    };

    private startDragSelect(e: PointerEvent): void {
        const { wx, wy } = this.camera.screenToWorld(e.clientX, e.clientY);
        this.selectionBox.start(wx, wy);
        this.isDragSelecting = true;

        const onMove = (moveEvt: PointerEvent): void => {
            const { wx: mwx, wy: mwy } = this.camera.screenToWorld(moveEvt.clientX, moveEvt.clientY);
            this.selectionBox.move(mwx, mwy);
        };

        const onUp = (_upEvt: PointerEvent): void => {
            this.canvas.removeEventListener('pointermove', onMove);
            this.canvas.removeEventListener('pointerup', onUp);

            const ids = this.selectionBox.end();
            if (ids.length > 0) {
                this.selectedEntities.clear();
                for (const id of ids) {
                    this.selectedEntities.add(id);
                }
            } else {
                // No entities in box: deselect all
                this.selectedEntities.clear();
            }
            this.isDragSelecting = false;
        };

        this.canvas.addEventListener('pointermove', onMove);
        this.canvas.addEventListener('pointerup', onUp);
    }

    private issueMoveOrder(screenX: number, screenY: number): void {
        if (this.selectedEntities.size === 0) return;

        const { wx, wy } = this.camera.screenToWorld(screenX, screenY);
        const { tx, ty } = screenToTile(wx, wy);

        const mapW = this.bridge.getMapWidth();
        const mapH = this.bridge.getMapHeight();
        const tileX = Math.floor(tx);
        const tileY = Math.floor(ty);

        if (tileX >= 0 && tileX < mapW && tileY >= 0 && tileY < mapH) {
            // Send move command for each selected entity
            for (const entityId of this.selectedEntities) {
                this.bridge.cmdMoveUnit(entityId, tx, ty);
            }
            this.moveIndicator.show(tx, ty);
        }
    }
}

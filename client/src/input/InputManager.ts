import { SelectionIndicator } from './SelectionIndicator';
import { MoveOrderIndicator } from './MoveOrderIndicator';
import { SelectionBox } from './SelectionBox';
import { TouchHandler } from './TouchHandler';
import type { CameraController } from '../render/CameraController';
import type { SpritePool } from '../render/SpritePool';
import type { GameBridge } from '../bridge/GameBridge';
import { battleWorldToTile } from '../render/BattleViewProjection';

/**
 * Handles user input: selection (single, multi, box), movement, touch gestures.
 * Left-click: select, Shift+click: toggle, Drag: selection box.
 * Right-click / touch long-press: move order for all selected.
 * Defend: selected units hold their current ground and auto-engage nearby enemies.
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
                if (entityId !== null && this.isControllableEntity(entityId)) {
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
                const ids = this.selectionBox.end().filter((id) => this.isControllableEntity(id));
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

    issueDefendOrder(): void {
        if (this.selectedEntities.size === 0) return;

        for (const entityId of this.selectedEntities) {
            const sprite = this.spritePool.getSprite(entityId);
            if (!sprite) continue;

            const { tx, ty } = battleWorldToTile(sprite.x, sprite.y);
            this.bridge.cmdAttackMove(entityId, tx, ty);
        }
    }

    private onPointerDown = (e: PointerEvent): void => {
        // Ignore touch events (handled by TouchHandler)
        if (e.pointerType === 'touch') return;

        if (e.button === 0) {
            // Left-click: select or start drag-select
            const { wx, wy } = this.camera.screenToWorld(e.clientX, e.clientY);
            const entityId = this.spritePool.getEntityAtScreen(wx, wy);

            if (e.shiftKey && entityId !== null && this.isControllableEntity(entityId)) {
                // Shift+click: toggle entity in selection
                if (this.selectedEntities.has(entityId)) {
                    this.selectedEntities.delete(entityId);
                } else {
                    this.selectedEntities.add(entityId);
                }
            } else if (entityId !== null && this.isControllableEntity(entityId)) {
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

            const ids = this.selectionBox.end().filter((id) => this.isControllableEntity(id));
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
        const ids = Array.from(this.selectedEntities);

        // Check if right-clicking on an enemy unit — if so, attack instead of move
        const targetEntityId = this.spritePool.getAttackableEnemyAtScreen(wx, wy, 0);
        if (targetEntityId !== null) {
            this.bridge.cmdAttackTarget(ids, targetEntityId);
            return;
        }

        const { tx, ty } = battleWorldToTile(wx, wy);

        const mapW = this.bridge.getMapWidth();
        const mapH = this.bridge.getMapHeight();
        const tileX = Math.floor(tx);
        const tileY = Math.floor(ty);

        if (tileX >= 0 && tileX < mapW && tileY >= 0 && tileY < mapH) {
            const targets = this.getFormationTargets(ids, tx, ty, mapW, mapH);
            for (const target of targets) {
                this.bridge.cmdMoveUnit(target.entityId, target.tx, target.ty);
            }
            this.moveIndicator.show(tx, ty);
        }
    }

    private getFormationTargets(
        entityIds: number[],
        targetTx: number,
        targetTy: number,
        mapW: number,
        mapH: number,
    ): Array<{ entityId: number; tx: number; ty: number }> {
        if (entityIds.length <= 1) {
            return [{ entityId: entityIds[0], tx: targetTx, ty: targetTy }];
        }

        const units = entityIds.map((entityId) => {
            const sprite = this.spritePool.getSprite(entityId);
            if (sprite) {
                const { tx, ty } = battleWorldToTile(sprite.x, sprite.y);
                return { entityId, tx, ty };
            }
            return { entityId, tx: targetTx, ty: targetTy };
        });

        units.sort((a, b) => (a.ty - b.ty) || (a.tx - b.tx));

        const cols = Math.ceil(Math.sqrt(entityIds.length));
        const rows = Math.ceil(entityIds.length / cols);
        const spacing = 1.1;
        const slots: Array<{ tx: number; ty: number }> = [];

        for (let row = 0; row < rows; row++) {
            for (let col = 0; col < cols; col++) {
                if (slots.length >= entityIds.length) break;
                slots.push({
                    tx: this.clampTile(
                        targetTx + (col - (cols - 1) / 2) * spacing,
                        mapW,
                    ),
                    ty: this.clampTile(
                        targetTy + (row - (rows - 1) / 2) * spacing,
                        mapH,
                    ),
                });
            }
        }

        slots.sort((a, b) => (a.ty - b.ty) || (a.tx - b.tx));

        return units.map((unit, index) => ({
            entityId: unit.entityId,
            tx: slots[index].tx,
            ty: slots[index].ty,
        }));
    }

    private clampTile(value: number, size: number): number {
        return Math.max(0.5, Math.min(size - 0.5, value));
    }

    private isControllableEntity(entityId: number): boolean {
        return this.spritePool.isCommandableEntity(entityId, 0);
    }
}

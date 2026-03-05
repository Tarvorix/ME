export interface RenderEntry {
    entityId: number;
    x: number;
    y: number;
    spriteId: number;
    frame: number;
    healthPct: number;
    facing: number;
    owner: number;
    flags: number;
    scale: number;
    zOrder: number;
}

export enum SpriteType {
    Thrall = 0,
    Sentinel = 1,
    HoverTank = 2,
    CommandPost = 3,
    Forge = 4,
}

export enum Direction {
    S = 0, SW = 1, W = 2, NW = 3,
    N = 4, NE = 5, E = 6, SE = 7,
}

export const DIRECTION_NAMES = ['S', 'SW', 'W', 'NW', 'N', 'NE', 'E', 'SE'];

export enum AnimState {
    Idle = 0, Move = 1, Attack = 2, Death = 3,
}

export const ANIM_NAMES: Record<number, string> = {
    0: 'Idle',
    1: 'Move',
    2: 'Shoot',
    3: 'Death',
};

export const UNIT_NAMES: Record<number, string> = {
    0: 'Thrall',
    1: 'Sentinel',
    2: 'hover_tank',
    3: 'command_post',
    4: 'forge',
};

export enum EventType {
    Shot = 0,
    Death = 1,
    UnitSpawned = 2,
    BuildingPlaced = 3,
    ProductionComplete = 4,
    CaptureProgress = 5,
    CaptureComplete = 6,
    BattleEnd = 7,
}

export interface GameEvent {
    eventType: number;
    entityId: number;
    x: number;
    y: number;
    payload: DataView;
}

export interface UIState {
    energy: number;
    income: number;
    expense: number;
    strain: number;
    strainPenalty: number;
    gameTick: number;
}

export function readUIState(view: DataView): UIState {
    return {
        energy: view.getFloat32(0, true),
        income: view.getFloat32(4, true),
        expense: view.getFloat32(8, true),
        strain: view.getFloat32(12, true),
        strainPenalty: view.getFloat32(16, true),
        gameTick: view.getUint32(20, true),
    };
}

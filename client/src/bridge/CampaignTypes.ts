/**
 * Campaign data types matching the WASM buffer formats.
 * All buffer layouts are little-endian.
 */

// ── Site Types ───────────────────────────────────────────────────────────

export enum SiteType {
    Node = 0,
    MiningStation = 1,
    RelicSite = 2,
}

export const SITE_TYPE_NAMES: Record<SiteType, string> = {
    [SiteType.Node]: 'Node',
    [SiteType.MiningStation]: 'Mining Station',
    [SiteType.RelicSite]: 'Relic Site',
};

export interface CampaignSiteData {
    siteId: number;
    siteType: SiteType;
    owner: number;         // 255 = neutral
    isContested: boolean;
    garrisonCount: number;
    x: number;
    y: number;
    garrisonThralls: number;
    garrisonSentinels: number;
    garrisonTanks: number;
    battleId: number;      // 0 = no battle
}

// ── Economy ──────────────────────────────────────────────────────────────

export interface CampaignEconomyData {
    energyBank: number;
    nodeIncome: number;
    mineIncome: number;
    relicIncome: number;
    totalIncome: number;
    totalExpenses: number;
    netRate: number;
    strain: number;
    garrisonUpkeep: number;
    deployedUpkeep: number;
}

// ── Research ─────────────────────────────────────────────────────────────

export enum TechId {
    ThrallPlating = 0,
    SentinelHeavyWeapons = 1,
    HoverTankReactiveArmor = 2,
    ImprovedVision = 3,
    ThrallFireRate = 4,
    SentinelShields = 5,
    HoverTankSiege = 6,
    FastProduction = 7,
    ThrallRange = 8,
    SentinelStealth = 9,
    HoverTankOvercharge = 10,
    EconomicEfficiency = 11,
}

export const TECH_NAMES: Record<TechId, string> = {
    [TechId.ThrallPlating]: 'Thrall Plating',
    [TechId.SentinelHeavyWeapons]: 'Sentinel Heavy Weapons',
    [TechId.HoverTankReactiveArmor]: 'Hover Tank Reactive Armor',
    [TechId.ImprovedVision]: 'Improved Vision',
    [TechId.ThrallFireRate]: 'Thrall Fire Rate',
    [TechId.SentinelShields]: 'Sentinel Shields',
    [TechId.HoverTankSiege]: 'Hover Tank Siege',
    [TechId.FastProduction]: 'Fast Production',
    [TechId.ThrallRange]: 'Thrall Range',
    [TechId.SentinelStealth]: 'Sentinel Stealth',
    [TechId.HoverTankOvercharge]: 'Hover Tank Overcharge',
    [TechId.EconomicEfficiency]: 'Economic Efficiency',
};

export const TECH_DESCRIPTIONS: Record<TechId, string> = {
    [TechId.ThrallPlating]: '+20% HP for Thralls',
    [TechId.SentinelHeavyWeapons]: '+20% damage for Sentinels',
    [TechId.HoverTankReactiveArmor]: '+15% HP for Hover Tanks',
    [TechId.ImprovedVision]: '+2 vision range for all units',
    [TechId.ThrallFireRate]: '-20% attack cooldown for Thralls',
    [TechId.SentinelShields]: '+30% HP for Sentinels',
    [TechId.HoverTankSiege]: '+30% attack range for Hover Tanks',
    [TechId.FastProduction]: '-15% production time for all units',
    [TechId.ThrallRange]: '+2 attack range for Thralls',
    [TechId.SentinelStealth]: 'Sentinels not visible in fog unless adjacent',
    [TechId.HoverTankOvercharge]: '+50% speed burst for Hover Tanks',
    [TechId.EconomicEfficiency]: '+20% income from all sources',
};

export interface TechDefinition {
    id: TechId;
    name: string;
    description: string;
    tier: number;           // 1, 2, or 3
    energyCost: number;
    researchTime: number;   // seconds
    requiredRelics: number;
    prerequisite: TechId | null;
}

/** Get static tech definition. */
export function getTechDefinition(id: TechId): TechDefinition {
    const tier = id <= 3 ? 1 : id <= 7 ? 2 : 3;
    const [cost, time, relics] = tier === 1 ? [200, 60, 1] : tier === 2 ? [500, 120, 1] : [1000, 180, 2];

    const prereqMap: Partial<Record<TechId, TechId>> = {
        [TechId.ThrallFireRate]: TechId.ThrallPlating,
        [TechId.SentinelShields]: TechId.SentinelHeavyWeapons,
        [TechId.HoverTankSiege]: TechId.HoverTankReactiveArmor,
        [TechId.FastProduction]: TechId.ImprovedVision,
        [TechId.ThrallRange]: TechId.ThrallFireRate,
        [TechId.SentinelStealth]: TechId.SentinelShields,
        [TechId.HoverTankOvercharge]: TechId.HoverTankSiege,
        [TechId.EconomicEfficiency]: TechId.FastProduction,
    };

    return {
        id,
        name: TECH_NAMES[id],
        description: TECH_DESCRIPTIONS[id],
        tier,
        energyCost: cost,
        researchTime: time,
        requiredRelics: relics,
        prerequisite: prereqMap[id] ?? null,
    };
}

export interface CampaignResearchData {
    activeTechId: number;       // 255 = none
    activeProgress: number;     // 0.0 to researchTime
    activeTotalTime: number;    // total time for active research
    completedCount: number;
    completedTechs: number[];   // array of TechId values
}

// ── Dispatch ─────────────────────────────────────────────────────────────

export interface DispatchOrderData {
    orderId: number;
    player: number;
    sourceSite: number;
    targetSite: number;
    travelRemaining: number;
    totalTime: number;
    unitCount: number;
}

// ── Battles ──────────────────────────────────────────────────────────────

export enum BattleStatus {
    Deployment = 0,
    Active = 1,
    Finished = 2,
}

export interface ActiveBattleData {
    siteId: number;
    attacker: number;
    defender: number;
    status: BattleStatus;
    winner: number;         // 255 = no winner yet
    tickCount: number;
}

// ── Unit Types (campaign context) ────────────────────────────────────────

export enum CampaignUnitType {
    Thrall = 0,
    Sentinel = 1,
    HoverTank = 2,
}

export const CAMPAIGN_UNIT_NAMES: Record<CampaignUnitType, string> = {
    [CampaignUnitType.Thrall]: 'Thrall',
    [CampaignUnitType.Sentinel]: 'Sentinel',
    [CampaignUnitType.HoverTank]: 'Hover Tank',
};

export const CAMPAIGN_UNIT_COSTS: Record<CampaignUnitType, number> = {
    [CampaignUnitType.Thrall]: 30,
    [CampaignUnitType.Sentinel]: 120,
    [CampaignUnitType.HoverTank]: 300,
};

export const CAMPAIGN_UNIT_BUILD_TIMES: Record<CampaignUnitType, number> = {
    [CampaignUnitType.Thrall]: 5,
    [CampaignUnitType.Sentinel]: 15,
    [CampaignUnitType.HoverTank]: 30,
};

// ── AI Difficulty ────────────────────────────────────────────────────────

export enum CampaignAiDifficulty {
    Easy = 0,
    Normal = 1,
    Hard = 2,
}

// ── Constants ────────────────────────────────────────────────────────────

/** Neutral/unowned player ID sentinel. */
export const NEUTRAL_OWNER = 255;

/** No active battle sentinel. */
export const NO_BATTLE = 0;

/** No active tech sentinel. */
export const NO_TECH = 255;

/** Dispatch failure sentinel. */
export const DISPATCH_FAILED = 0xFFFFFFFF;

/** Buffer entry sizes in bytes. */
export const SITE_ENTRY_BYTES = 32;
export const ECONOMY_ENTRY_BYTES = 40;
export const RESEARCH_ENTRY_BYTES = 22;
export const DISPATCH_ENTRY_BYTES = 28;
export const BATTLE_ENTRY_BYTES = 12;

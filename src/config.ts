export const SEC_PER_YEAR = 6; // ゲーム内 1 年あたりの秒数（等速時）
export const START_YEAR = -770;
export const WARRING_YEAR = -475;
export const SPLIT_YEAR = -403;
export const CAVALRY_YEAR = -307;
export const END_YEAR = -221;

export type Era = 'spring' | 'warring';
export type ResKey = 'food' | 'wood' | 'metal';
export interface Res {
  food: number;
  wood: number;
  metal: number;
}
export const RES_NAMES: Record<ResKey, string> = { food: '糧', wood: '木', metal: '金' };

export type UnitKind = 'worker' | 'infantry' | 'chariot' | 'archer' | 'cavalry' | 'ram';
export const MILITARY_KINDS: UnitKind[] = ['infantry', 'chariot', 'archer', 'cavalry', 'ram'];
/** 城（町の中心）で生産できるユニット */
export const CITY_UNITS: UnitKind[] = ['worker', 'infantry'];
/** 兵営で生産できるユニット */
export const BARRACKS_UNITS: UnitKind[] = ['infantry', 'chariot', 'archer', 'cavalry', 'ram'];
export const MAX_QUEUE = 5;

export type BuildingKind = 'storehouse' | 'barracks' | 'tower' | 'market';
export const BUILDING_KINDS: BuildingKind[] = ['storehouse', 'barracks', 'tower', 'market'];
export interface BuildingStat {
  name: string;
  cost: Res;
  hp: number;
  time: number;
  desc: string;
}
export const BUILDING: Record<BuildingKind, BuildingStat> = {
  storehouse: { name: '倉庫', cost: { food: 0, wood: 100, metal: 0 }, hp: 600, time: 15, desc: '資源の集積所。工人の運搬距離が短くなる' },
  barracks: { name: '兵営', cost: { food: 50, wood: 150, metal: 0 }, hp: 1100, time: 25, desc: '戦車・弓兵・騎兵・衝車を訓練する' },
  tower: { name: '望楼', cost: { food: 0, wood: 60, metal: 100 }, hp: 800, time: 20, desc: '近づく敵を射る防御施設' },
  market: { name: '市', cost: { food: 0, wood: 200, metal: 0 }, hp: 900, time: 30, desc: '資源を交易する（相場は天下共通）' },
};
export const FOREST_AMOUNT = 400;
export const MINE_AMOUNT = 1500;

export interface UnitStat {
  name: string;
  short: string;
  atk: number;
  def: number;
  range: number;
  speed: number;
  siege: number;
  cost: Res;
  time: number;
}

export const UNIT: Record<UnitKind, UnitStat> = {
  worker: { name: '工人', short: '工', atk: 0, def: 0.6, range: 0.8, speed: 1.5, siege: 0, cost: { food: 50, wood: 0, metal: 0 }, time: 6 },
  infantry: { name: '歩兵', short: '歩', atk: 1.0, def: 1.0, range: 1.0, speed: 1.4, siege: 0.6, cost: { food: 50, wood: 0, metal: 30 }, time: 8 },
  chariot: { name: '戦車', short: '車', atk: 1.35, def: 1.15, range: 1.0, speed: 2.0, siege: 0.35, cost: { food: 60, wood: 60, metal: 40 }, time: 10 },
  archer: { name: '弓兵', short: '弓', atk: 0.85, def: 0.7, range: 4.0, speed: 1.4, siege: 0.25, cost: { food: 40, wood: 50, metal: 20 }, time: 8 },
  cavalry: { name: '騎兵', short: '騎', atk: 1.25, def: 0.95, range: 1.0, speed: 2.8, siege: 0.3, cost: { food: 70, wood: 0, metal: 40 }, time: 10 },
  ram: { name: '衝車', short: '衝', atk: 0.12, def: 1.5, range: 1.0, speed: 0.9, siege: 5.0, cost: { food: 20, wood: 160, metal: 40 }, time: 14 },
};

/** 兵種相性（攻撃側 → 防御側） */
export function counterMult(att: UnitKind, def: UnitKind): number {
  if (att === 'chariot' && (def === 'infantry' || def === 'archer')) return 1.3;
  if (att === 'infantry' && def === 'cavalry') return 1.3;
  if (att === 'infantry' && def === 'chariot') return 1.15;
  if (att === 'archer' && def === 'infantry') return 1.2;
  if (att === 'archer' && def === 'cavalry') return 0.8;
  if (att === 'cavalry' && def === 'archer') return 1.6;
  if (att === 'cavalry' && def === 'chariot') return 1.1;
  if (def === 'worker') return 1.5;
  if (def === 'ram') return att === 'archer' ? 0.4 : att === 'infantry' || att === 'cavalry' ? 1.6 : 1.2;
  return 1;
}

export const DMG_RATE = 0.06;
export const UPKEEP_PER_ABILITY = 0.0007; // 糧/秒 per 能力1
export const ABILITY_STEP = 25;
export const MIN_ABILITY = 20;
export const CONVERT_RATIO = 2 / 3;
export const CITY_MIN_SPACING = 7;
export const BUILD_TIME = 25;
export const NEW_CITY_COST: Res = { food: 150, wood: 200, metal: 0 };

export function abilityCap(era: Era): number {
  return era === 'spring' ? 250 : 400;
}
export function cityLevelCap(era: Era): number {
  return era === 'spring' ? 3 : 5;
}
export function cityMaxHp(level: number): number {
  return 600 + 400 * level;
}
export function cityRadius(level: number): number {
  return 6 + 2 * level;
}
export function cityUpgradeCost(level: number): Res {
  return { food: 150 * level, wood: 200 * level, metal: 100 * level };
}

export function yearLabel(y: number): string {
  return y < 0 ? `前${-y}年` : `${y}年`;
}

export function addRes(a: Res, b: Res, k = 1): void {
  a.food += b.food * k;
  a.wood += b.wood * k;
  a.metal += b.metal * k;
}
export function canAfford(a: Res, b: Res): boolean {
  return a.food >= b.food - 1e-6 && a.wood >= b.wood - 1e-6 && a.metal >= b.metal - 1e-6;
}
export function resStr(r: Res): string {
  const parts: string[] = [];
  if (r.food) parts.push(`糧${Math.ceil(r.food)}`);
  if (r.wood) parts.push(`木${Math.ceil(r.wood)}`);
  if (r.metal) parts.push(`金${Math.ceil(r.metal)}`);
  return parts.join(' ') || '無料';
}

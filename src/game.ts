import {
  ABILITY_STEP, BARRACKS_UNITS, BUILDING, BUILD_TIME, CAVALRY_YEAR, CITY_MIN_SPACING, CITY_UNITS, CONVERT_RATIO, DMG_RATE,
  END_YEAR, MAX_QUEUE, MIN_ABILITY, NEW_CITY_COST, SEC_PER_YEAR, SPLIT_YEAR, START_YEAR, UNIT, UPKEEP_PER_ABILITY, WARRING_YEAR,
  abilityCap, addRes, canAfford, cityLevelCap, cityMaxHp, cityRadius, cityUpgradeCost, counterMult, resStr,
  type BuildingKind, type Era, type Res, type ResKey, type UnitKind,
} from './config';
import { STATE_DEFS, SEVEN_HEROES, type StateDef } from './data/states';
import { GameMap, MH, MW, T_FOREST, T_HILL, T_MOUNTAIN, T_PLAIN, T_STEPPE, latToY, lonToX } from './map';
import { mulberry32 } from './rng';
import { aiThink, aiYearly } from './ai';

export interface State {
  id: number;
  def: StateDef;
  name: string;
  color: string;
  rgb: [number, number, number];
  bias: number;
  aggr: number;
  alive: boolean;
  everAlive: boolean;
  isPlayer: boolean;
  res: Res;
  aggro: number;
  hegemonCooldownUntil: number;
  aiTimer: number;
  starving: boolean;
  warTarget: number;
  army: Set<number>;
  peaceOffers: Set<number>;
  eliminatedYear: number | null;
  capturedCities: number;
}

export interface QueueItem {
  kind: UnitKind | 'upgrade';
  progress: number;
  time: number;
  cost: Res;
}

/** 城と建物の共通部分 */
export interface Structure {
  id: number;
  owner: number;
  x: number;
  y: number;
  hp: number;
  /** 建設進捗 0..1 (1=完成) */
  built: number;
  queue: QueueItem[];
  rally: { x: number; y: number } | null;
  lastHitTime: number;
  destroyed: boolean;
}

export interface City extends Structure {
  isCity: true;
  name: string;
  level: number;
  capital: boolean;
  lastAttacker: number;
}

export interface Building extends Structure {
  isCity: false;
  kind: BuildingKind;
}

export type Struct = City | Building;

export type Order =
  | { type: 'idle' }
  | { type: 'move'; x: number; y: number; attackMove: boolean }
  | { type: 'attack'; unitId?: number; structId?: number }
  | { type: 'gather'; tx: number; ty: number; res: ResKey; phase: 'toRes' | 'gathering' | 'toDrop'; dropId: number }
  | { type: 'build'; structId: number };

export interface Unit {
  id: number;
  owner: number;
  kind: UnitKind;
  x: number;
  y: number;
  ability: number;
  maxAbility: number;
  order: Order;
  path: { x: number; y: number }[];
  carry: number;
  carryType: ResKey;
  homeX: number;
  homeY: number;
  engageId: number; // 自動交戦中の目標 (unit id, 負なら -structId-1)
  repath: number;
  lastHitTime: number;
  dead: boolean;
}

export interface War {
  a: number;
  b: number;
  since: number;
  forced: boolean;
}

export interface GEvent {
  kind: 'hegemon' | 'hezong';
  target: number;
  caller: number;
  members: number[];
  startYear: number;
  endYear: number;
  title: string;
}

export interface Msg {
  text: string;
  year: number;
  kind: 'info' | 'war' | 'event' | 'good' | 'bad';
  player: boolean;
}

export interface GameOver {
  victory: boolean;
  reason: string;
  ranking: { name: string; color: string; power: number; cities: number; alive: boolean }[];
}

export type FogMode = 'none' | 'explored' | 'full';

const STRATEGISTS = ['蘇秦', '公孫衍', '信陵君', '春申君', '龐煖', '孟嘗君', '平原君'];

function hexToRgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function structMaxHp(s: Struct): number {
  return s.isCity ? cityMaxHp(s.level) : BUILDING[s.kind].hp;
}
export function structName(s: Struct): string {
  return s.isCity ? s.name : BUILDING[s.kind].name;
}

export class Game {
  map: GameMap;
  rng: () => number;
  time = 0;
  year = START_YEAR;
  era: Era = 'spring';
  states: State[] = [];
  cities: City[] = [];
  buildings: Building[] = [];
  units: Unit[] = [];
  unitById = new Map<number, Unit>();
  cityById = new Map<number, City>();
  structById = new Map<number, Struct>();
  wars = new Map<string, War>();
  event: GEvent | null = null;
  lastEventEnd = -9999;
  lastHezongEnd = -9999;
  playerId: number;
  messages: Msg[] = [];
  nextId = 1;
  territoryVersion = 0;
  over: GameOver | null = null;
  onMessage: ((m: Msg) => void) | null = null;
  onEvent: ((title: string, body: string) => void) | null = null;
  /** 天下共通の市場相場（糧換算） */
  prices: Record<ResKey, number> = { food: 1, wood: 1, metal: 1.4 };
  /** 戦場の霧（プレイヤー視点） */
  fogMode: FogMode;
  visible = new Uint8Array(MW * MH);
  explored = new Uint8Array(MW * MH);
  visVersion = 0;
  private visTimer = 0;
  private grid: Unit[][] = [];
  private readonly cell = 6;
  private gcols = Math.ceil(MW / 6);
  private grows = Math.ceil(MH / 6);
  private territoryDirty = true;

  constructor(playerKey: string | null, seed = 20240, fogMode: FogMode = 'explored') {
    this.rng = mulberry32(seed);
    this.fogMode = playerKey ? fogMode : 'none';
    const cityTiles: [number, number][] = [];
    for (const d of STATE_DEFS) for (const c of d.cities) cityTiles.push([Math.floor(lonToX(c.lon)), Math.floor(latToY(c.lat))]);
    this.map = new GameMap(seed, cityTiles);
    this.map.changed.length = 0;
    for (let i = 0; i < this.gcols * this.grows; i++) this.grid.push([]);

    STATE_DEFS.forEach((d, id) => {
      const st: State = {
        id, def: d, name: d.name, color: d.color, rgb: hexToRgb(d.color), bias: d.bias, aggr: d.aggr,
        alive: d.cities.length > 0, everAlive: d.cities.length > 0, isPlayer: d.key === playerKey,
        res: { food: 220, wood: 200, metal: 110 }, aggro: 0, hegemonCooldownUntil: -9999,
        aiTimer: this.rng() * 1.5, starving: false, warTarget: -1, army: new Set(), peaceOffers: new Set(),
        eliminatedYear: null, capturedCities: 0,
      };
      this.states.push(st);
      d.cities.forEach((c, k) => {
        const city = this.addCity(id, c.name, Math.floor(lonToX(c.lon)) + 0.5, Math.floor(latToY(c.lat)) + 0.5, c.level ?? 1, 1);
        city.capital = k === 0;
      });
    });
    this.playerId = this.states.findIndex((s) => s.isPlayer);
    this.recomputeTerritory();
    // 初期ユニットと建物
    for (const s of this.states) {
      if (!s.alive) continue;
      const cs = this.ownCities(s.id);
      const nWorkers = 2 + cs.length + (s.def.major ? 1 : 0);
      const nSoldiers = s.def.major ? 3 : 2;
      for (let k = 0; k < nWorkers; k++) {
        const u = this.spawnUnit(s.id, 'worker', cs[k % cs.length]);
        this.autoGather(u);
      }
      for (let k = 0; k < nSoldiers; k++) this.spawnUnit(s.id, k === 0 ? 'chariot' : 'infantry', cs[0]);
      if (s.def.major) {
        const site = this.findBuildingSite(s.id, cs[0].x, cs[0].y, 2, 4);
        if (site) this.addBuilding(s.id, 'barracks', site[0] + 0.5, site[1] + 0.5, 1);
      }
    }
    this.updateVisibility(true);
  }

  // ---------- 参照系 ----------
  get player(): State | null {
    return this.playerId >= 0 ? this.states[this.playerId] : null;
  }
  ownCities(sid: number, includeUnbuilt = false): City[] {
    return this.cities.filter((c) => c.owner === sid && !c.destroyed && (includeUnbuilt || c.built >= 1));
  }
  ownBuildings(sid: number, kind?: BuildingKind, includeUnbuilt = false): Building[] {
    return this.buildings.filter((b) => b.owner === sid && !b.destroyed && (!kind || b.kind === kind) && (includeUnbuilt || b.built >= 1));
  }
  allStructs(): Struct[] {
    return [...this.cities, ...this.buildings];
  }
  unitsOf(sid: number): Unit[] {
    return this.units.filter((u) => u.owner === sid && !u.dead);
  }
  workerCount(sid: number): number {
    let n = 0;
    for (const u of this.units) if (u.owner === sid && !u.dead && u.kind === 'worker') n++;
    return n;
  }
  soldierCount(sid: number): number {
    let n = 0;
    for (const u of this.units) if (u.owner === sid && !u.dead && u.kind !== 'worker') n++;
    return n;
  }
  idleWorkers(sid: number): Unit[] {
    return this.units.filter((u) => u.owner === sid && !u.dead && u.kind === 'worker' && u.order.type === 'idle');
  }
  queuedCount(sid: number, military: boolean): number {
    let n = 0;
    const count = (q: QueueItem[]): void => {
      for (const it of q) if (it.kind !== 'upgrade' && (it.kind === 'worker') !== military) n++;
    };
    for (const c of this.cities) if (c.owner === sid && !c.destroyed) count(c.queue);
    for (const b of this.buildings) if (b.owner === sid && !b.destroyed) count(b.queue);
    return n;
  }
  workerCap(sid: number): number {
    return Math.min(14, 2 + 2 * this.ownCities(sid).length);
  }
  soldierCap(sid: number): number {
    const cs = this.ownCities(sid);
    let lv = 0;
    for (const c of cs) lv += c.level - 1;
    return Math.min(20, 3 + 2 * cs.length + lv);
  }
  milStrength(sid: number): number {
    let s = 0;
    for (const u of this.units) if (u.owner === sid && !u.dead && u.kind !== 'worker') s += u.ability * UNIT[u.kind].atk;
    return s;
  }
  power(sid: number): number {
    let p = 0;
    for (const c of this.cities) if (c.owner === sid && !c.destroyed && c.built >= 1) p += 100 + 150 * c.level;
    for (const b of this.buildings) if (b.owner === sid && !b.destroyed && b.built >= 1) p += 40;
    for (const u of this.units) {
      if (u.owner !== sid || u.dead) continue;
      p += u.kind === 'worker' ? u.ability * 0.3 : u.ability * 0.6;
    }
    const r = this.states[sid].res;
    return p + (r.food + r.wood + r.metal) * 0.05;
  }
  upkeepPerSec(sid: number): number {
    let s = 0;
    for (const u of this.units) if (u.owner === sid && !u.dead && u.kind !== 'worker') s += u.ability * UPKEEP_PER_ABILITY;
    return s;
  }
  kindAvailable(kind: UnitKind): boolean {
    if (kind === 'cavalry') return this.year >= CAVALRY_YEAR;
    return true;
  }
  unitName(kind: UnitKind): string {
    if (kind === 'archer' && this.era === 'warring') return '弩兵';
    return UNIT[kind].name;
  }
  unitAtk(u: Unit): number {
    let a = UNIT[u.kind].atk;
    if (this.era === 'warring') {
      a *= 1.15; // 鉄器の普及（全国一律）
      if (u.kind === 'archer') a *= 1.3;
      if (u.kind === 'chariot') a *= 0.85;
    }
    return a;
  }
  gatherMult(): number {
    return this.era === 'warring' ? 1.2 : 1;
  }
  producible(s: Struct): UnitKind[] {
    if (s.isCity) return CITY_UNITS;
    if (s.kind === 'barracks') return BARRACKS_UNITS;
    return [];
  }
  /** プレイヤーから見えるか */
  isVisible(x: number, y: number): boolean {
    if (this.fogMode === 'none') return true;
    const tx = Math.floor(x);
    const ty = Math.floor(y);
    if (!this.map.inBounds(tx, ty)) return false;
    return this.visible[ty * MW + tx] === 1;
  }
  isExplored(x: number, y: number): boolean {
    if (this.fogMode !== 'full') return true;
    const tx = Math.floor(x);
    const ty = Math.floor(y);
    if (!this.map.inBounds(tx, ty)) return false;
    return this.explored[ty * MW + tx] === 1;
  }

  // ---------- 外交 ----------
  warKey(a: number, b: number): string {
    return a < b ? `${a}-${b}` : `${b}-${a}`;
  }
  atWar(a: number, b: number): boolean {
    if (a === b || a < 0 || b < 0) return false;
    return this.wars.has(this.warKey(a, b));
  }
  enemiesOf(sid: number): number[] {
    const out: number[] = [];
    for (const w of this.wars.values()) {
      if (w.a === sid) out.push(w.b);
      else if (w.b === sid) out.push(w.a);
    }
    return out;
  }
  declareWar(a: number, b: number, forced = false): void {
    if (a === b || this.atWar(a, b)) return;
    if (!this.states[a].alive || !this.states[b].alive) return;
    this.wars.set(this.warKey(a, b), { a, b, since: this.year, forced });
    this.states[a].peaceOffers.delete(b);
    this.states[b].peaceOffers.delete(a);
    if (!forced) {
      this.addAggro(a, b, 15);
      this.msg(`${this.states[a].name} が ${this.states[b].name} に宣戦布告`, 'war', a === this.playerId || b === this.playerId);
    }
  }
  makePeace(a: number, b: number, quiet = false): void {
    if (!this.atWar(a, b)) return;
    this.wars.delete(this.warKey(a, b));
    this.states[a].peaceOffers.delete(b);
    this.states[b].peaceOffers.delete(a);
    if (this.states[a].warTarget === b) this.states[a].warTarget = -1;
    if (this.states[b].warTarget === a) this.states[b].warTarget = -1;
    if (!quiet) this.msg(`${this.states[a].name} と ${this.states[b].name} が講和`, 'info', a === this.playerId || b === this.playerId);
  }
  getWar(a: number, b: number): War | undefined {
    return this.wars.get(this.warKey(a, b));
  }
  isForcedWar(a: number, b: number): boolean {
    const ev = this.event;
    if (!ev) return false;
    return (ev.target === a && ev.members.includes(b)) || (ev.target === b && ev.members.includes(a));
  }
  /** プレイヤーからの講和申し入れ */
  requestPeace(from: number, to: number): boolean {
    if (!this.atWar(from, to)) return false;
    if (this.isForcedWar(from, to)) {
      this.msg(`${this.states[to].name}: 盟約により講和はできぬ`, 'bad', true);
      return false;
    }
    const w = this.getWar(from, to)!;
    const dur = this.year - w.since;
    const mine = this.milStrength(from) + 50;
    const theirs = this.milStrength(to) + 50;
    if (this.states[to].peaceOffers.has(from) || (dur >= 3 && theirs < mine * 1.5) || dur >= 12) {
      this.makePeace(from, to);
      return true;
    }
    this.msg(`${this.states[to].name} は講和を拒否した`, 'bad', true);
    return false;
  }
  minCityDistance(a: number, b: number): number {
    let best = Infinity;
    const ca = this.ownCities(a);
    const cb = this.ownCities(b);
    for (const x of ca) for (const y of cb) best = Math.min(best, Math.hypot(x.x - y.x, x.y - y.y));
    return best;
  }
  addAggro(attacker: number, victim: number, amount: number): void {
    if (this.era !== 'spring' || attacker < 0) return;
    const ev = this.event;
    if (ev && ev.target === victim && ev.members.includes(attacker)) return;
    if (ev && ev.target === attacker) return;
    this.states[attacker].aggro += amount;
  }

  // ---------- 市場 ----------
  tradeRate(from: ResKey, to: ResKey): number {
    return (this.prices[from] / this.prices[to]) * 0.75;
  }
  /** 市で from を amount 売って to を得る */
  trade(sid: number, from: ResKey, to: ResKey, amount = 100): boolean {
    if (from === to || !this.ownBuildings(sid, 'market').length) return false;
    const s = this.states[sid];
    if (s.res[from] < amount) return false;
    s.res[from] -= amount;
    s.res[to] += amount * this.tradeRate(from, to);
    this.prices[from] = Math.max(0.3, this.prices[from] * 0.96);
    this.prices[to] = Math.min(4, this.prices[to] * 1.04);
    return true;
  }

  // ---------- メッセージ ----------
  msg(text: string, kind: Msg['kind'] = 'info', player = false): void {
    const m: Msg = { text, year: this.year, kind, player };
    this.messages.push(m);
    if (this.messages.length > 200) this.messages.shift();
    this.onMessage?.(m);
  }

  // ---------- 生成 ----------
  addCity(owner: number, name: string, x: number, y: number, level: number, built: number): City {
    const c: City = {
      isCity: true, id: this.nextId++, name, owner, x, y, level, hp: built >= 1 ? cityMaxHp(level) : 50, built, capital: false,
      queue: [], rally: null, lastAttacker: -1, lastHitTime: -99, destroyed: false,
    };
    this.cities.push(c);
    this.cityById.set(c.id, c);
    this.structById.set(c.id, c);
    this.territoryDirty = true;
    return c;
  }

  addBuilding(owner: number, kind: BuildingKind, x: number, y: number, built: number): Building {
    const b: Building = {
      isCity: false, id: this.nextId++, kind, owner, x, y, hp: built >= 1 ? BUILDING[kind].hp : 30, built,
      queue: [], rally: null, lastHitTime: -99, destroyed: false,
    };
    this.buildings.push(b);
    this.structById.set(b.id, b);
    return b;
  }

  spawnUnit(owner: number, kind: UnitKind, at: { x: number; y: number }, ability = 100): Unit {
    let x = at.x;
    let y = at.y;
    for (let t = 0; t < 10; t++) {
      const a = this.rng() * Math.PI * 2;
      const r = 1 + this.rng() * 1.2;
      const nx = at.x + Math.cos(a) * r;
      const ny = at.y + Math.sin(a) * r;
      if (this.map.passable(nx, ny)) {
        x = nx;
        y = ny;
        break;
      }
    }
    return this.createUnit(owner, kind, x, y, ability, ability);
  }

  createUnit(owner: number, kind: UnitKind, x: number, y: number, ability: number, maxAbility: number): Unit {
    const u: Unit = {
      id: this.nextId++, owner, kind, x, y, ability, maxAbility, order: { type: 'idle' }, path: [], carry: 0, carryType: 'food',
      homeX: x, homeY: y, engageId: 0, repath: 0, lastHitTime: -99, dead: false,
    };
    this.units.push(u);
    this.unitById.set(u.id, u);
    return u;
  }

  // ---------- 生産 ----------
  enqueue(p: Struct, kind: UnitKind | 'upgrade'): boolean {
    if (p.queue.length >= MAX_QUEUE || p.built < 1 || p.destroyed) return false;
    const sid = p.owner;
    const s = this.states[sid];
    if (kind === 'upgrade') {
      if (!p.isCity || p.queue.some((q) => q.kind === 'upgrade')) return false;
      if (p.level >= cityLevelCap(this.era)) return false;
      const cost = cityUpgradeCost(p.level);
      if (!canAfford(s.res, cost)) return false;
      addRes(s.res, cost, -1);
      p.queue.push({ kind, progress: 0, time: 20 + 10 * p.level, cost });
      return true;
    }
    if (!this.kindAvailable(kind) || !this.producible(p).includes(kind)) return false;
    const st = UNIT[kind];
    if (kind === 'worker') {
      if (this.workerCount(sid) + this.queuedCount(sid, false) >= this.workerCap(sid)) return false;
    } else if (this.soldierCount(sid) + this.queuedCount(sid, true) >= this.soldierCap(sid)) return false;
    if (!canAfford(s.res, st.cost)) return false;
    addRes(s.res, st.cost, -1);
    // 兵営での訓練は早い
    const time = !p.isCity && kind !== 'ram' ? st.time * 0.75 : st.time;
    p.queue.push({ kind, progress: 0, time, cost: { ...st.cost } });
    return true;
  }
  cancelQueue(p: Struct, index: number): void {
    const it = p.queue[index];
    if (!it) return;
    p.queue.splice(index, 1);
    addRes(this.states[p.owner].res, it.cost, 1);
  }

  // ---------- コマンド ----------
  moveUnits(units: Unit[], x: number, y: number, attackMove: boolean): void {
    const n = units.length;
    const cols = Math.ceil(Math.sqrt(n));
    units.forEach((u, k) => {
      const ox = n > 1 ? ((k % cols) - (cols - 1) / 2) * 0.9 : 0;
      const oy = n > 1 ? (Math.floor(k / cols) - (cols - 1) / 2) * 0.9 : 0;
      let tx = x + ox;
      let ty = y + oy;
      if (!this.map.passable(tx, ty)) {
        tx = x;
        ty = y;
      }
      u.order = { type: 'move', x: tx, y: ty, attackMove: u.kind === 'worker' ? false : attackMove };
      u.engageId = 0;
      u.path = this.map.findPath(u.x, u.y, tx, ty) ?? [];
    });
  }

  attackTarget(units: Unit[], target: { unit?: Unit; struct?: Struct }): void {
    for (const u of units) {
      if (u.kind === 'worker') continue;
      if (u.kind === 'ram' && target.unit) continue;
      u.order = target.unit ? { type: 'attack', unitId: target.unit.id } : { type: 'attack', structId: target.struct!.id };
      u.engageId = 0;
      u.path = [];
      u.repath = 0;
    }
  }

  gather(u: Unit, tx: number, ty: number): boolean {
    const res = this.map.gatherType(tx, ty);
    if (!res || u.kind !== 'worker') return false;
    const own = this.map.owner[ty * MW + tx];
    if (own !== -1 && own !== u.owner) return false;
    u.order = { type: 'gather', tx, ty, res, phase: 'toRes', dropId: -1 };
    if (u.carryType !== res) u.carry = 0;
    u.carryType = res;
    u.path = this.map.findPath(u.x, u.y, tx + 0.5, ty + 0.5) ?? [];
    return true;
  }

  /** 最寄りの採集地を探す */
  findResourceTile(sid: number, res: ResKey, x: number, y: number, maxR = 16, randomize = true): [number, number] | null {
    const cx = Math.floor(x);
    const cy = Math.floor(y);
    const found: [number, number, number][] = [];
    for (let r = 0; r <= maxR; r++) {
      for (let dy = -r; dy <= r; dy++) {
        for (let dx = -r; dx <= r; dx++) {
          if (Math.max(Math.abs(dx), Math.abs(dy)) !== r) continue;
          const tx = cx + dx;
          const ty = cy + dy;
          if (!this.map.inBounds(tx, ty)) continue;
          const i = ty * MW + tx;
          if (this.map.owner[i] !== sid) continue;
          if (this.map.gatherType(tx, ty) !== res) continue;
          found.push([tx, ty, this.map.gatherYield(tx, ty) - r * 0.01]);
        }
      }
      if (found.length >= 6) break;
    }
    if (!found.length) return null;
    found.sort((a, b) => b[2] - a[2]);
    const top = found.slice(0, Math.min(found.length, 4));
    const pick = randomize ? top[Math.floor(this.rng() * top.length)] : top[0];
    return [pick[0], pick[1]];
  }

  autoGather(u: Unit, res: ResKey = 'food', near?: { x: number; y: number }): boolean {
    const ref = near ?? this.nearestDrop(u.owner, u.x, u.y) ?? { x: u.x, y: u.y };
    const order: ResKey[] = [res, 'food', 'wood', 'metal'];
    for (const r of order) {
      const t = this.findResourceTile(u.owner, r, ref.x, ref.y);
      if (t && this.gather(u, t[0], t[1])) return true;
    }
    return false;
  }

  nearestOwnCity(sid: number, x: number, y: number): City | null {
    let best: City | null = null;
    let bd = Infinity;
    for (const c of this.cities) {
      if (c.owner !== sid || c.destroyed || c.built < 1) continue;
      const d = (c.x - x) ** 2 + (c.y - y) ** 2;
      if (d < bd) {
        bd = d;
        best = c;
      }
    }
    return best;
  }
  /** 資源を降ろせる最寄りの施設（城・倉庫） */
  nearestDrop(sid: number, x: number, y: number): Struct | null {
    let best: Struct | null = this.nearestOwnCity(sid, x, y);
    let bd = best ? (best.x - x) ** 2 + (best.y - y) ** 2 : Infinity;
    for (const b of this.buildings) {
      if (b.owner !== sid || b.destroyed || b.built < 1 || b.kind !== 'storehouse') continue;
      const d = (b.x - x) ** 2 + (b.y - y) ** 2;
      if (d < bd) {
        bd = d;
        best = b;
      }
    }
    return best;
  }

  canTrain(u: Unit): boolean {
    return u.maxAbility + ABILITY_STEP <= abilityCap(this.era) && this.map.ownerAt(u.x, u.y) === u.owner;
  }
  trainCost(u: Unit): Res {
    if (u.kind === 'worker') return { food: Math.round(u.maxAbility * 0.4), wood: Math.round(u.maxAbility * 0.3), metal: 0 };
    if (u.kind === 'ram') return { food: 0, wood: Math.round(u.maxAbility * 0.5), metal: Math.round(u.maxAbility * 0.3) };
    return { food: Math.round(u.maxAbility * 0.3), wood: 0, metal: Math.round(u.maxAbility * 0.5) };
  }
  train(u: Unit): boolean {
    if (!this.canTrain(u)) return false;
    const s = this.states[u.owner];
    const cost = this.trainCost(u);
    if (!canAfford(s.res, cost)) return false;
    addRes(s.res, cost, -1);
    u.maxAbility += ABILITY_STEP;
    u.ability += ABILITY_STEP;
    return true;
  }
  reinforceCost(u: Unit): Res {
    const miss = Math.max(0, u.maxAbility - u.ability);
    return { food: Math.round(miss * 0.5), wood: 0, metal: Math.round(miss * 0.3) };
  }
  reinforce(u: Unit): boolean {
    if (u.kind === 'worker' || u.ability >= u.maxAbility - 0.5) return false;
    if (this.map.ownerAt(u.x, u.y) !== u.owner) return false;
    const s = this.states[u.owner];
    const cost = this.reinforceCost(u);
    if (!canAfford(s.res, cost)) return false;
    addRes(s.res, cost, -1);
    u.ability = u.maxAbility;
    return true;
  }
  /** 軍事ユニット → 生産ユニット (能力 2/3) */
  disband(u: Unit): Unit | null {
    if (u.kind === 'worker' || u.dead) return null;
    if (this.workerCount(u.owner) >= this.workerCap(u.owner)) return null;
    const ab = Math.max(MIN_ABILITY, Math.round(u.ability * CONVERT_RATIO));
    this.killUnit(u);
    return this.createUnit(u.owner, 'worker', u.x, u.y, ab, ab);
  }
  conscriptCost(kind: UnitKind): Res {
    const c = UNIT[kind].cost;
    return { food: 0, wood: c.wood, metal: c.metal };
  }
  /** 生産ユニット → 軍事ユニット (能力 2/3) */
  conscript(u: Unit, kind: UnitKind): Unit | null {
    if (u.kind !== 'worker' || kind === 'worker' || u.dead || !this.kindAvailable(kind)) return null;
    if (this.soldierCount(u.owner) >= this.soldierCap(u.owner)) return null;
    const s = this.states[u.owner];
    const cost = this.conscriptCost(kind);
    if (!canAfford(s.res, cost)) return null;
    addRes(s.res, cost, -1);
    const ab = Math.max(MIN_ABILITY, Math.round(u.maxAbility * CONVERT_RATIO));
    this.killUnit(u);
    return this.createUnit(u.owner, kind, u.x, u.y, ab, ab);
  }

  canFoundCity(sid: number, tx: number, ty: number): string | null {
    if (!this.map.inBounds(tx, ty)) return '範囲外';
    const t = this.map.terrain[ty * MW + tx];
    if (t !== T_PLAIN && t !== T_STEPPE && t !== T_FOREST && t !== T_HILL) return 'この地形には築城できない';
    const own = this.map.owner[ty * MW + tx];
    if (own !== -1 && own !== sid) return '他国の領土には築城できない';
    for (const c of this.cities) {
      if (c.destroyed) continue;
      if (Math.hypot(c.x - (tx + 0.5), c.y - (ty + 0.5)) < CITY_MIN_SPACING) return '他の城に近すぎる';
    }
    for (const b of this.buildings) if (!b.destroyed && Math.hypot(b.x - (tx + 0.5), b.y - (ty + 0.5)) < 1.5) return '建物と重なっている';
    if (!canAfford(this.states[sid].res, NEW_CITY_COST)) return `資源不足 (${resStr(NEW_CITY_COST)})`;
    return null;
  }
  foundCity(sid: number, tx: number, ty: number, builders: Unit[]): City | null {
    if (this.canFoundCity(sid, tx, ty)) return null;
    addRes(this.states[sid].res, NEW_CITY_COST, -1);
    const names = ['新邑', '新城', '東邑', '西邑', '南邑', '北邑', '新田', '下邑', '上邑', '平邑'];
    const name = names[Math.floor(this.rng() * names.length)];
    const c = this.addCity(sid, name, tx + 0.5, ty + 0.5, 1, 0);
    this.map.prepareCitySite(tx, ty, 1);
    this.map.recomputeCost();
    for (const b of builders) this.orderBuild(b, c);
    return c;
  }

  canPlaceBuilding(sid: number, kind: BuildingKind, tx: number, ty: number): string | null {
    if (!this.map.inBounds(tx, ty)) return '範囲外';
    const i = ty * MW + tx;
    const t = this.map.terrain[i];
    if (t !== T_PLAIN && t !== T_STEPPE && t !== T_HILL && t !== T_FOREST) return 'この地形には建てられない';
    if (this.map.mine[i]) return '鉱山の上には建てられない';
    if (this.map.owner[i] !== sid) return '自国の領土にしか建てられない';
    for (const s of this.allStructs()) {
      if (s.destroyed) continue;
      if (Math.hypot(s.x - (tx + 0.5), s.y - (ty + 0.5)) < (s.isCity ? 2.5 : 1.8)) return '他の建物に近すぎる';
    }
    if (kind === 'market' && this.ownBuildings(sid, 'market', true).length) return '市は一国に一つまで';
    if (kind === 'barracks' && this.ownBuildings(sid, 'barracks', true).length >= Math.max(1, this.ownCities(sid).length)) return '兵営は城の数まで';
    if (kind === 'tower' && this.ownBuildings(sid, 'tower', true).length >= 2 * this.ownCities(sid).length) return '望楼は城の数×2まで';
    if (!canAfford(this.states[sid].res, BUILDING[kind].cost)) return `資源不足 (${resStr(BUILDING[kind].cost)})`;
    return null;
  }
  placeBuilding(sid: number, kind: BuildingKind, tx: number, ty: number, builders: Unit[]): Building | null {
    if (this.canPlaceBuilding(sid, kind, tx, ty)) return null;
    addRes(this.states[sid].res, BUILDING[kind].cost, -1);
    const b = this.addBuilding(sid, kind, tx + 0.5, ty + 0.5, 0);
    for (const w of builders) this.orderBuild(w, b);
    return b;
  }
  /** 近くの建設可能地点を探す */
  findBuildingSite(sid: number, x: number, y: number, rmin: number, rmax: number, kind: BuildingKind = 'storehouse'): [number, number] | null {
    for (let t = 0; t < 40; t++) {
      const a = this.rng() * Math.PI * 2;
      const r = rmin + this.rng() * (rmax - rmin);
      const tx = Math.floor(x + Math.cos(a) * r);
      const ty = Math.floor(y + Math.sin(a) * r);
      const res = this.states[sid].res;
      const saved = { ...res };
      addRes(res, BUILDING[kind].cost, 1); // 費用チェックを除外して地点だけ判定
      const err = this.canPlaceBuilding(sid, kind, tx, ty);
      Object.assign(res, saved);
      if (!err || err.startsWith('兵営は') || err.startsWith('市は')) {
        if (!err) return [tx, ty];
      }
    }
    return null;
  }
  orderBuild(u: Unit, s: Struct): void {
    if (u.kind !== 'worker') return;
    u.order = { type: 'build', structId: s.id };
    u.path = this.map.findPath(u.x, u.y, s.x + 0.8, s.y + 0.8) ?? [];
  }
  setRally(s: Struct, x: number, y: number): void {
    s.rally = { x, y };
  }

  // ---------- 内部処理 ----------
  killUnit(u: Unit): void {
    if (u.dead) return;
    u.dead = true;
    this.unitById.delete(u.id);
    this.states[u.owner]?.army.delete(u.id);
  }

  recomputeTerritory(): void {
    const owner = this.map.owner;
    owner.fill(-1);
    const score = new Float32Array(MW * MH).fill(Infinity);
    for (const c of this.cities) {
      if (c.destroyed || c.built < 1) continue;
      const R = cityRadius(c.level);
      const x0 = Math.max(0, Math.floor(c.x - R));
      const x1 = Math.min(MW - 1, Math.ceil(c.x + R));
      const y0 = Math.max(0, Math.floor(c.y - R));
      const y1 = Math.min(MH - 1, Math.ceil(c.y + R));
      for (let y = y0; y <= y1; y++) {
        for (let x = x0; x <= x1; x++) {
          const i = y * MW + x;
          if (this.map.cost[i] === Infinity) continue;
          const d = Math.hypot(x + 0.5 - c.x, y + 0.5 - c.y);
          if (d > R) continue;
          const sc = d / R;
          if (sc < score[i]) {
            score[i] = sc;
            owner[i] = c.owner;
          }
        }
      }
    }
    this.territoryDirty = false;
    this.territoryVersion++;
  }

  /** プレイヤーの視界を更新 */
  updateVisibility(force = false): void {
    if (this.fogMode === 'none' || this.playerId < 0) return;
    const pid = this.playerId;
    const vis = this.visible;
    vis.fill(0);
    const mark = (x: number, y: number, r: number): void => {
      const x0 = Math.max(0, Math.floor(x - r));
      const x1 = Math.min(MW - 1, Math.floor(x + r));
      const y0 = Math.max(0, Math.floor(y - r));
      const y1 = Math.min(MH - 1, Math.floor(y + r));
      const r2 = r * r;
      for (let ty = y0; ty <= y1; ty++) {
        for (let tx = x0; tx <= x1; tx++) {
          if ((tx + 0.5 - x) ** 2 + (ty + 0.5 - y) ** 2 <= r2) vis[ty * MW + tx] = 1;
        }
      }
    };
    for (const c of this.cities) if (c.owner === pid && !c.destroyed) mark(c.x, c.y, c.built >= 1 ? cityRadius(c.level) + 2 : 4);
    for (const b of this.buildings) if (b.owner === pid && !b.destroyed) mark(b.x, b.y, b.kind === 'tower' ? 9 : 4);
    for (const u of this.units) if (u.owner === pid && !u.dead) mark(u.x, u.y, u.kind === 'worker' ? 5 : u.kind === 'cavalry' ? 8 : 6.5);
    const ex = this.explored;
    for (let i = 0; i < vis.length; i++) if (vis[i]) ex[i] = 1;
    this.visVersion++;
    void force;
  }

  private buildGrid(): void {
    for (const b of this.grid) b.length = 0;
    for (const u of this.units) {
      if (u.dead) continue;
      const cx = Math.min(this.gcols - 1, Math.max(0, Math.floor(u.x / this.cell)));
      const cy = Math.min(this.grows - 1, Math.max(0, Math.floor(u.y / this.cell)));
      this.grid[cy * this.gcols + cx].push(u);
    }
  }

  forEachNear(x: number, y: number, r: number, fn: (u: Unit) => void): void {
    const x0 = Math.max(0, Math.floor((x - r) / this.cell));
    const x1 = Math.min(this.gcols - 1, Math.floor((x + r) / this.cell));
    const y0 = Math.max(0, Math.floor((y - r) / this.cell));
    const y1 = Math.min(this.grows - 1, Math.floor((y + r) / this.cell));
    for (let cy = y0; cy <= y1; cy++) for (let cx = x0; cx <= x1; cx++) for (const u of this.grid[cy * this.gcols + cx]) fn(u);
  }

  /** 1 ステップ進める (dt 秒) */
  update(dt: number): void {
    if (this.over) return;
    this.time += dt;
    const newYear = START_YEAR + Math.floor(this.time / SEC_PER_YEAR);
    while (this.year < newYear) {
      this.year++;
      this.yearly();
      if (this.over) return;
    }
    if (this.territoryDirty) this.recomputeTerritory();
    this.buildGrid();
    this.updateStructs(dt);
    for (const u of this.units) {
      if (u.dead) continue;
      if (u.kind === 'worker') this.updateWorker(u, dt);
      else this.updateSoldier(u, dt);
    }
    this.updateEconomy(dt);
    // 死亡ユニット・破壊された建物の除去
    let w = 0;
    for (const u of this.units) {
      if (!u.dead) this.units[w++] = u;
      else this.unitById.delete(u.id);
    }
    this.units.length = w;
    if (this.buildings.some((b) => b.destroyed)) {
      for (const b of this.buildings) if (b.destroyed) this.structById.delete(b.id);
      this.buildings = this.buildings.filter((b) => !b.destroyed);
    }
    if (this.territoryDirty) this.recomputeTerritory();
    // 相場は少しずつ元に戻る
    for (const k of ['food', 'wood', 'metal'] as ResKey[]) {
      const base = k === 'metal' ? 1.4 : 1;
      this.prices[k] += (base - this.prices[k]) * 0.002 * dt;
    }
    // 視界
    this.visTimer -= dt;
    if (this.visTimer <= 0) {
      this.visTimer = 0.4;
      this.updateVisibility();
    }
    // AI
    for (const s of this.states) {
      if (!s.alive || s.isPlayer) continue;
      s.aiTimer -= dt;
      if (s.aiTimer <= 0) {
        s.aiTimer += 1.5;
        aiThink(this, s);
      }
    }
  }

  private updateEconomy(dt: number): void {
    for (const s of this.states) {
      if (!s.alive) continue;
      const cs = this.ownCities(s.id);
      for (const c of cs) {
        s.res.food += 0.2 * c.level * s.bias * dt;
        s.res.wood += 0.06 * c.level * s.bias * dt;
        s.res.metal += 0.04 * c.level * s.bias * dt;
      }
      const up = this.upkeepPerSec(s.id) * dt;
      s.res.food -= up;
      if (s.res.food < 0) {
        s.res.food = 0;
        if (!s.starving) {
          s.starving = true;
          if (s.isPlayer) this.msg('兵糧が尽きた！ 兵が逃亡している', 'bad', true);
        }
        for (const u of this.units) {
          if (u.owner === s.id && !u.dead && u.kind !== 'worker') {
            u.ability -= u.ability * 0.02 * dt;
            u.maxAbility = Math.max(u.ability, u.maxAbility - u.maxAbility * 0.005 * dt);
            if (u.ability < MIN_ABILITY * 0.5) this.killUnit(u);
          }
        }
      } else if (s.res.food > 5) s.starving = false;
    }
  }

  private onUnitProduced(p: Struct, u: Unit): void {
    const r = p.rally;
    if (u.kind === 'worker') {
      if (r) {
        const tx = Math.floor(r.x);
        const ty = Math.floor(r.y);
        const target = this.structById.get(this.structAtTile(tx, ty) ?? -1);
        if (target && target.owner === u.owner && target.built < 1) this.orderBuild(u, target);
        else if (!this.gather(u, tx, ty)) this.moveUnits([u], r.x, r.y, false);
      } else this.autoGather(u);
    } else if (r) {
      this.moveUnits([u], r.x, r.y, true);
    }
  }

  structAtTile(tx: number, ty: number): number | null {
    for (const s of this.allStructs()) {
      if (s.destroyed) continue;
      if (Math.abs(s.x - (tx + 0.5)) < 1 && Math.abs(s.y - (ty + 0.5)) < 1) return s.id;
    }
    return null;
  }

  private updateStructs(dt: number): void {
    const structs: Struct[] = [...this.cities, ...this.buildings];
    for (const c of structs) {
      if (c.destroyed || c.built < 1) continue;
      const s = this.states[c.owner];
      const maxHp = structMaxHp(c);
      // 生産キュー
      const q = c.queue[0];
      if (q) {
        q.progress += dt;
        if (q.progress >= q.time) {
          c.queue.shift();
          const k = q.kind;
          if (k === 'upgrade') {
            if (c.isCity) {
              c.level++;
              c.hp += 400;
              this.territoryDirty = true;
              if (s.isPlayer) this.msg(`${c.name} を Lv${c.level} に強化した`, 'good', true);
            }
          } else {
            const u = this.spawnUnit(c.owner, k, c, this.era === 'warring' ? 110 : 100);
            this.onUnitProduced(c, u);
          }
        }
      }
      // 防衛射撃（城・望楼）
      const isTower = !c.isCity && c.kind === 'tower';
      let shot = false;
      if (c.isCity || isTower) {
        const range = isTower ? 5 : 3.5;
        const base = c.isCity ? 3 + 3 * c.level : 7;
        const dmg = base * s.bias * (this.era === 'warring' ? 1.2 : 1) * dt;
        let bestU: Unit | null = null;
        let bd = Infinity;
        this.forEachNear(c.x, c.y, range, (u) => {
          if (u.dead || !this.atWar(u.owner, c.owner)) return;
          const d = Math.hypot(u.x - c.x, u.y - c.y);
          if (d < range && d < bd) {
            bd = d;
            bestU = u;
          }
        });
        if (bestU) {
          const bu = bestU as Unit;
          this.damageUnit(bu, (dmg * (bu.kind === 'ram' ? 0.3 : 1)) / UNIT[bu.kind].def, c.owner);
          shot = true;
        }
      }
      const underAttack = this.time - c.lastHitTime < 4;
      if (!underAttack && !shot && c.hp < maxHp) c.hp = Math.min(maxHp, c.hp + (c.isCity ? 4 + c.level : 2) * dt);
    }
  }

  damageUnit(u: Unit, amount: number, _attacker: number): void {
    if (u.dead) return;
    u.ability -= amount;
    u.lastHitTime = this.time;
    if (u.ability < MIN_ABILITY * 0.5) this.killUnit(u);
  }

  damageStruct(st: Struct, amount: number, attacker: number): void {
    if (st.destroyed) return;
    st.hp -= amount / this.states[st.owner].bias; // 国力の補正（七雄の城は堅い）
    st.lastHitTime = this.time;
    if (st.isCity) {
      st.lastAttacker = attacker;
      this.addAggro(attacker, st.owner, amount * 0.03);
      if (st.hp <= 0) this.captureCity(st, attacker);
    } else {
      this.addAggro(attacker, st.owner, amount * 0.01);
      if (st.hp <= 0) {
        st.destroyed = true;
        st.queue.length = 0;
        const pl = st.owner === this.playerId || attacker === this.playerId;
        if (pl) this.msg(`${this.states[attacker].name} が ${this.states[st.owner].name} の${BUILDING[st.kind].name}を破壊`, attacker === this.playerId ? 'good' : 'bad', true);
      }
    }
  }

  captureCity(c: City, attacker: number): void {
    const old = c.owner;
    const os = this.states[old];
    const as = this.states[attacker];
    if (c.built < 1) {
      c.destroyed = true;
      this.cityById.delete(c.id);
      this.structById.delete(c.id);
      this.msg(`${as.name} が ${os.name} の築城中の ${c.name} を破壊`, 'war', old === this.playerId || attacker === this.playerId);
      return;
    }
    c.owner = attacker;
    c.level = Math.max(1, c.level - 1);
    c.hp = cityMaxHp(c.level) * 0.3;
    for (const q of c.queue) addRes(os.res, q.cost, 1);
    c.queue = [];
    c.rally = null;
    const wasCapital = c.capital;
    c.capital = false;
    as.capturedCities++;
    this.territoryDirty = true;
    this.addAggro(attacker, old, 30);
    // 周辺の建物も接収
    for (const b of this.buildings) {
      if (b.destroyed || b.owner !== old) continue;
      if (Math.hypot(b.x - c.x, b.y - c.y) <= cityRadius(c.level + 1)) {
        b.owner = attacker;
        b.queue = [];
        b.rally = null;
        b.hp = Math.max(b.hp, 1);
      }
    }
    const pl = old === this.playerId || attacker === this.playerId;
    this.msg(`${as.name} が ${os.name} の ${c.name} を攻略`, pl ? (attacker === this.playerId ? 'good' : 'bad') : 'war', pl);
    for (const u of this.units) {
      if (u.dead || u.owner !== old) continue;
      if (u.order.type === 'gather') u.order.dropId = -1;
    }
    const remaining = this.ownCities(old, true);
    const built = remaining.filter((x) => x.built >= 1);
    if (built.length === 0) {
      this.eliminate(old, attacker);
      for (const x of remaining) {
        x.destroyed = true;
        this.cityById.delete(x.id);
        this.structById.delete(x.id);
      }
    } else if (wasCapital) {
      built.sort((a, b) => b.level - a.level);
      built[0].capital = true;
      if (old === this.playerId) this.msg(`都を ${built[0].name} に遷した`, 'bad', true);
    }
  }

  eliminate(sid: number, by: number): void {
    const s = this.states[sid];
    if (!s.alive) return;
    s.alive = false;
    s.eliminatedYear = this.year;
    for (const u of this.units) if (u.owner === sid) this.killUnit(u);
    for (const b of this.buildings) if (b.owner === sid) b.destroyed = true;
    for (const k of [...this.wars.keys()]) {
      const w = this.wars.get(k)!;
      if (w.a === sid || w.b === sid) this.wars.delete(k);
    }
    for (const o of this.states) o.peaceOffers.delete(sid);
    this.msg(by >= 0 ? `${s.name} は ${this.states[by].name} に滅ぼされた` : `${s.name} は滅亡した`, sid === this.playerId ? 'bad' : 'event', true);
    if (this.event) {
      if (this.event.target === sid) this.endEvent(true);
      else this.event.members = this.event.members.filter((m) => m !== sid);
    }
    if (sid === this.playerId) this.finish(false, `${s.name} は滅亡した…`);
    else this.checkUnification();
  }

  private checkUnification(): void {
    const alive = this.states.filter((s) => s.alive);
    if (alive.length === 1) {
      const w = alive[0];
      this.finish(w.isPlayer, `${w.name} が天下を統一した！`);
    }
  }

  private finish(victory: boolean, reason: string): void {
    if (this.over) return;
    const ranking = this.states
      .filter((s) => s.everAlive)
      .map((s) => ({ name: s.name, color: s.color, power: Math.round(s.alive ? this.power(s.id) : 0), cities: this.ownCities(s.id).length, alive: s.alive }))
      .sort((a, b) => b.power - a.power);
    this.over = { victory, reason, ranking };
  }

  // ---------- ユニット ----------
  private stepMove(u: Unit, dt: number): boolean {
    if (!u.path.length) return true;
    const wp = u.path[0];
    const dx = wp.x - u.x;
    const dy = wp.y - u.y;
    const dist = Math.hypot(dx, dy);
    let cost = this.map.costAt(u.x, u.y);
    if (!isFinite(cost)) cost = 1;
    let sp = UNIT[u.kind].speed / cost;
    if (u.kind === 'chariot' || u.kind === 'cavalry' || u.kind === 'ram') {
      const t = this.map.terrainAt(u.x, u.y);
      if (t === T_FOREST || t === T_HILL || t === T_MOUNTAIN) sp *= u.kind === 'cavalry' ? 0.75 : 0.55;
    }
    const step = sp * dt;
    if (step >= dist) {
      u.x = wp.x;
      u.y = wp.y;
      u.path.shift();
      return u.path.length === 0;
    }
    u.x += (dx / dist) * step;
    u.y += (dy / dist) * step;
    return false;
  }

  private moveToward(u: Unit, tx: number, ty: number, dt: number): void {
    const last = u.path.length ? u.path[u.path.length - 1] : null;
    u.repath -= dt;
    if (!last || (Math.hypot(last.x - tx, last.y - ty) > 1.5 && u.repath <= 0)) {
      const d = Math.hypot(tx - u.x, ty - u.y);
      if (d < 3 && this.map.passable(tx, ty)) u.path = [{ x: tx, y: ty }];
      else u.path = this.map.findPath(u.x, u.y, tx, ty, d > 60 ? 40000 : 12000) ?? [];
      u.repath = 1 + this.rng() * 0.5;
    }
    this.stepMove(u, dt);
  }

  private updateWorker(u: Unit, dt: number): void {
    const o = u.order;
    const s = this.states[u.owner];
    if (o.type === 'move') {
      if (this.stepMove(u, dt)) u.order = { type: 'idle' };
      return;
    }
    if (o.type === 'build') {
      const c = this.structById.get(o.structId);
      if (!c || c.destroyed || c.owner !== u.owner || (c.built >= 1 && c.hp >= structMaxHp(c))) {
        u.order = { type: 'idle' };
        if (!s.isPlayer || c?.built === 1) this.autoGather(u);
        return;
      }
      if (Math.hypot(c.x - u.x, c.y - u.y) > 1.8) {
        if (this.stepMove(u, dt) && Math.hypot(c.x - u.x, c.y - u.y) > 1.8) u.path = this.map.findPath(u.x, u.y, c.x + 0.8, c.y + 0.8) ?? [];
        return;
      }
      if (c.built >= 1) {
        // 修理
        c.hp = Math.min(structMaxHp(c), c.hp + 12 * (u.ability / 100) * dt);
        return;
      }
      const btime = c.isCity ? BUILD_TIME : BUILDING[c.kind].time;
      c.built += ((u.ability / 100) * dt) / btime;
      c.hp = Math.max(c.hp, 30 + (structMaxHp(c) * 0.5) * c.built);
      if (c.built >= 1) {
        c.built = 1;
        c.hp = structMaxHp(c);
        if (c.isCity) {
          this.territoryDirty = true;
          if (!this.ownCities(u.owner).some((x) => x.capital)) c.capital = true;
          this.msg(`${s.name} が ${c.name} を築いた`, s.isPlayer ? 'good' : 'info', s.isPlayer);
        } else if (s.isPlayer) this.msg(`${BUILDING[c.kind].name}が完成した`, 'good', true);
      }
      return;
    }
    if (o.type === 'gather') {
      if (o.phase === 'toRes') {
        if (this.stepMove(u, dt)) {
          if (Math.hypot(o.tx + 0.5 - u.x, o.ty + 0.5 - u.y) < 0.9) o.phase = 'gathering';
          else u.path = this.map.findPath(u.x, u.y, o.tx + 0.5, o.ty + 0.5) ?? [];
          if (!u.path.length && o.phase !== 'gathering') o.phase = 'gathering';
        }
        return;
      }
      if (o.phase === 'gathering') {
        const own = this.map.owner[o.ty * MW + o.tx];
        if ((own !== -1 && own !== u.owner) || this.map.gatherType(o.tx, o.ty) !== o.res) {
          if (u.carry > 0) {
            o.phase = 'toDrop';
            o.dropId = -1;
          } else if (!this.autoGather(u, o.res, { x: o.tx, y: o.ty })) u.order = { type: 'idle' };
          return;
        }
        const rate = this.map.gatherYield(o.tx, o.ty) * (u.ability / 100) * s.bias * this.gatherMult();
        u.carry += this.map.harvest(o.tx, o.ty, rate * dt);
        const cap = 10 * (u.ability / 100);
        if (u.carry >= cap) {
          u.carry = cap;
          o.phase = 'toDrop';
          o.dropId = -1;
        }
        return;
      }
      // toDrop
      let c = o.dropId >= 0 ? this.structById.get(o.dropId) : undefined;
      if (!c || c.owner !== u.owner || c.destroyed || c.built < 1) {
        const n = this.nearestDrop(u.owner, u.x, u.y);
        if (!n) {
          u.order = { type: 'idle' };
          return;
        }
        c = n;
        o.dropId = n.id;
        u.path = this.map.findPath(u.x, u.y, c.x, c.y) ?? [];
      }
      if (Math.hypot(c.x - u.x, c.y - u.y) < 1.3) {
        s.res[o.res] += u.carry;
        u.carry = 0;
        o.phase = 'toRes';
        u.path = this.map.findPath(u.x, u.y, o.tx + 0.5, o.ty + 0.5) ?? [];
        return;
      }
      if (this.stepMove(u, dt)) u.path = this.map.findPath(u.x, u.y, c.x, c.y) ?? [];
      return;
    }
  }

  private findEnemyNear(u: Unit, r: number, includeStructs: boolean): { unit?: Unit; struct?: Struct } | null {
    if (u.kind !== 'ram') {
      let best: Unit | null = null;
      let bd = r;
      this.forEachNear(u.x, u.y, r, (o) => {
        if (o.dead || o.owner === u.owner || !this.atWar(u.owner, o.owner)) return;
        let d = Math.hypot(o.x - u.x, o.y - u.y);
        if (o.kind === 'worker') d += 1.5; // 兵を優先
        if (d < bd) {
          bd = d;
          best = o;
        }
      });
      if (best) return { unit: best };
    }
    if (includeStructs || u.kind === 'ram') {
      let bc: Struct | null = null;
      let bcd = u.kind === 'ram' ? Math.max(r, 8) : r;
      for (const c of this.allStructs()) {
        if (c.destroyed || c.owner === u.owner || !this.atWar(u.owner, c.owner)) continue;
        // 城を優先
        const d = Math.hypot(c.x - u.x, c.y - u.y) + (c.isCity ? -1 : 0.5);
        if (d < bcd) {
          bcd = d;
          bc = c;
        }
      }
      if (bc) return { struct: bc };
    }
    return null;
  }

  private updateSoldier(u: Unit, dt: number): void {
    const st = UNIT[u.kind];
    const o = u.order;
    const own = this.map.ownerAt(u.x, u.y) === u.owner;
    // 自領で休息していれば回復
    if (own && this.time - u.lastHitTime > 5 && u.ability < u.maxAbility) u.ability = Math.min(u.maxAbility, u.ability + 0.5 * dt);

    let tUnit: Unit | undefined;
    let tStruct: Struct | undefined;
    if (o.type === 'attack') {
      if (o.unitId !== undefined) {
        tUnit = this.unitById.get(o.unitId);
        if (!tUnit || tUnit.dead) tUnit = undefined;
      } else if (o.structId !== undefined) {
        tStruct = this.structById.get(o.structId);
        if (!tStruct || tStruct.destroyed || tStruct.owner === u.owner) tStruct = undefined;
      }
      if (tUnit && !this.atWar(u.owner, tUnit.owner)) tUnit = undefined;
      if (tStruct && !this.atWar(u.owner, tStruct.owner)) tStruct = undefined;
      if (!tUnit && !tStruct) {
        u.order = { type: 'idle' };
        u.homeX = u.x;
        u.homeY = u.y;
        u.path = [];
      } else if (tStruct && u.kind !== 'ram') {
        // 攻城中でも近くの敵兵には反撃
        const near = this.findEnemyNear(u, Math.max(2.5, st.range + 0.5), false);
        if (near?.unit) {
          tUnit = near.unit;
          tStruct = undefined;
        }
      }
    }
    if (!tUnit && !tStruct && (u.order.type === 'idle' || (u.order.type === 'move' && u.order.attackMove))) {
      // 交戦中の目標を維持
      if (u.engageId > 0) {
        const e = this.unitById.get(u.engageId);
        if (e && !e.dead && this.atWar(u.owner, e.owner) && Math.hypot(e.x - u.x, e.y - u.y) < 9) tUnit = e;
        else u.engageId = 0;
      } else if (u.engageId < 0) {
        const c = this.structById.get(-u.engageId - 1);
        if (c && !c.destroyed && c.owner !== u.owner && this.atWar(u.owner, c.owner)) tStruct = c;
        else u.engageId = 0;
      }
      if (!tUnit && !tStruct) {
        const idle = u.order.type === 'idle';
        const r = idle ? Math.max(5, st.range + 1.5) : 6;
        const f = this.findEnemyNear(u, r, !idle || this.map.ownerAt(u.x, u.y) !== u.owner);
        if (f?.unit) {
          tUnit = f.unit;
          u.engageId = f.unit.id;
        } else if (f?.struct) {
          tStruct = f.struct;
          u.engageId = -f.struct.id - 1;
        }
      }
      // 待機中は持ち場から離れすぎない
      if (u.order.type === 'idle' && tUnit && Math.hypot(tUnit.x - u.homeX, tUnit.y - u.homeY) > 10) {
        tUnit = undefined;
        u.engageId = 0;
      }
    }

    if (tUnit || tStruct) {
      const tx = tUnit ? tUnit.x : tStruct!.x;
      const ty = tUnit ? tUnit.y : tStruct!.y;
      const d = Math.hypot(tx - u.x, ty - u.y) - (tStruct ? (tStruct.isCity ? 0.9 : 0.5) : 0);
      if (d <= st.range) {
        if (u.order.type !== 'move') u.path = [];
        const terr = this.map.terrainAt(u.x, u.y);
        if (tUnit) {
          let atk = u.ability * this.unitAtk(u) * DMG_RATE * dt;
          if (u.kind === 'chariot' && (terr === T_FOREST || terr === T_HILL || terr === T_MOUNTAIN)) atk *= 0.7;
          atk *= counterMult(u.kind, tUnit.kind) / UNIT[tUnit.kind].def;
          const tt = this.map.terrainAt(tUnit.x, tUnit.y);
          if (tt === T_HILL || tt === T_MOUNTAIN) atk *= 0.85;
          if (this.map.ownerAt(u.x, u.y) !== u.owner) this.addAggro(u.owner, tUnit.owner, atk * 0.02);
          this.damageUnit(tUnit, atk, u.owner);
        } else {
          let dmg = u.ability * DMG_RATE * st.siege * (this.era === 'warring' ? 1.15 : 1) * dt;
          if (tStruct!.isCity) dmg /= 1 + 0.15 * ((tStruct as City).level - 1);
          this.damageStruct(tStruct!, dmg, u.owner);
        }
        return;
      }
      this.moveToward(u, tx, ty, dt);
      return;
    }

    const ord = u.order;
    if (ord.type === 'move') {
      if (!u.path.length || u.engageId !== 0) {
        u.engageId = 0;
        if (Math.hypot(ord.x - u.x, ord.y - u.y) > 0.5) {
          u.path = this.map.findPath(u.x, u.y, ord.x, ord.y) ?? [];
          if (!u.path.length) u.order = { type: 'idle' };
        } else {
          u.order = { type: 'idle' };
          u.homeX = u.x;
          u.homeY = u.y;
        }
        return;
      }
      if (this.stepMove(u, dt)) {
        u.order = { type: 'idle' };
        u.homeX = u.x;
        u.homeY = u.y;
      }
      return;
    }
    if (ord.type === 'idle') {
      // 持ち場へ戻る
      if (Math.hypot(u.homeX - u.x, u.homeY - u.y) > 1.5) {
        if (!u.path.length) u.path = this.map.findPath(u.x, u.y, u.homeX, u.homeY) ?? [];
        if (this.stepMove(u, dt) && !u.path.length) {
          u.homeX = u.x;
          u.homeY = u.y;
        }
      } else u.path = [];
    }
  }

  // ---------- 年次処理 ----------
  private yearly(): void {
    if (this.year === WARRING_YEAR) {
      this.era = 'warring';
      this.msg('時代は戦国へ。鉄器が普及し、弓兵は弩兵に、攻撃力と生産力が全国で向上した', 'event', true);
      this.onEvent?.('戦国時代の到来', '周の権威は地に落ち、諸国は王を称して天下を争う。\n鉄器の普及により全国の攻撃力・生産力が向上し、弓兵は弩兵となる。\n以後、最も強大な国には合従軍が組まれることがある。');
      if (this.event?.kind === 'hegemon') this.endEvent(false);
    }
    if (this.year === SPLIT_YEAR) this.splitJin();
    if (this.year === CAVALRY_YEAR) {
      this.msg('胡服騎射 — 騎兵が全国で編成可能になった', 'event', true);
    }
    for (const s of this.states) s.aggro *= 0.92;
    // イベント終了
    if (this.event && this.year >= this.event.endYear) this.endEvent(false);
    // 覇者の呼びかけ
    if (this.era === 'spring' && !this.event && this.year - this.lastEventEnd >= 3) {
      let worst: State | null = null;
      for (const s of this.states) {
        if (!s.alive || s.aggro < 70 || this.year < s.hegemonCooldownUntil) continue;
        if (!worst || s.aggro > worst.aggro) worst = s;
      }
      if (worst) this.startHegemon(worst);
    }
    // 合従軍
    if (this.era === 'warring' && !this.event && this.year - this.lastHezongEnd >= 20 && this.year - this.lastEventEnd >= 3) this.checkHezong();
    // AI の年次判断
    for (const s of this.states) if (s.alive && !s.isPlayer) aiYearly(this, s);
    if (this.year >= END_YEAR) {
      const ranking = this.states.filter((s) => s.alive).sort((a, b) => this.power(b.id) - this.power(a.id));
      const top = ranking[0];
      const pl = this.player;
      if (pl && pl.alive && top === pl) this.finish(true, `前221年、${pl.name} は天下第一の国として時代を終えた`);
      else this.finish(false, `前221年、天下第一は ${top?.name ?? '-'}。${pl?.alive ? pl.name + ' は生き残った' : ''}`);
    }
  }

  private startHegemon(target: State): void {
    const members = this.states.filter((s) => s.alive && s.id !== target.id && this.minCityDistance(s.id, target.id) <= 45).map((s) => s.id);
    if (members.length < 2) {
      target.aggro *= 0.5;
      return;
    }
    let caller = members[0];
    for (const m of members) if (this.power(m) > this.power(caller)) caller = m;
    const title = `覇者の呼びかけ — ${this.states[caller].name} が諸侯を糾合し ${target.name} を討つ`;
    this.event = { kind: 'hegemon', target: target.id, caller, members, startYear: this.year, endYear: this.year + 10, title };
    for (const m of members) this.declareWar(m, target.id, true);
    target.aggro = 0;
    target.hegemonCooldownUntil = this.year + 25;
    this.msg(title, 'event', true);
    const names = members.map((m) => this.states[m].name).join('・');
    this.onEvent?.('覇者の呼びかけ', `${target.name} の度重なる侵攻に対し、覇者 ${this.states[caller].name} が「尊王攘夷」を掲げ諸侯に号令した。\n参加国: ${names}\n今後10年間、${target.name} は一斉に攻撃を受ける。`);
  }

  private checkHezong(): void {
    const alive = this.states.filter((s) => s.alive);
    if (alive.length < 3) return;
    const ps = alive.map((s) => ({ s, p: this.power(s.id) })).sort((a, b) => b.p - a.p);
    const total = ps.reduce((a, b) => a + b.p, 0);
    const top = ps[0];
    if (!(top.p >= ps[1].p * 1.15 || top.p >= total * 0.2)) return;
    const members = alive.filter((s) => s.id !== top.s.id && this.minCityDistance(s.id, top.s.id) <= 120).map((s) => s.id);
    if (members.length < 2) return;
    let caller = members[0];
    for (const m of members) if (this.power(m) > this.power(caller)) caller = m;
    const strat = STRATEGISTS[Math.floor(this.rng() * STRATEGISTS.length)];
    const title = `合従軍 — ${strat} の説により ${members.length} か国が ${top.s.name} を攻める`;
    this.event = { kind: 'hezong', target: top.s.id, caller, members, startYear: this.year, endYear: this.year + 12, title };
    for (const m of members) this.declareWar(m, top.s.id, true);
    this.msg(title, 'event', true);
    const names = members.map((m) => this.states[m].name).join('・');
    this.onEvent?.('合従軍', `天下最強の ${top.s.name} に対し、${strat} の合従策がなった。\n盟主: ${this.states[caller].name}\n参加国: ${names}\n今後12年間、${top.s.name} は諸国から一斉に攻撃を受ける。`);
  }

  endEvent(targetFell: boolean): void {
    const ev = this.event;
    if (!ev) return;
    this.event = null;
    this.lastEventEnd = this.year;
    if (ev.kind === 'hezong') this.lastHezongEnd = this.year;
    for (const m of ev.members) {
      const w = this.getWar(m, ev.target);
      if (w && w.forced) this.makePeace(m, ev.target, true);
    }
    const t = this.states[ev.target];
    const label = ev.kind === 'hegemon' ? '覇者の呼びかけ' : '合従軍';
    this.msg(targetFell ? `${label}は ${t.name} の滅亡をもって終結` : `${label}は解散した（${t.name} への包囲が解かれた）`, 'event', true);
  }

  private splitJin(): void {
    const jinId = this.states.findIndex((s) => s.def.key === 'jin');
    const jin = this.states[jinId];
    if (!jin || !jin.alive) {
      this.msg('晋はすでに亡く、三家分晋は起こらなかった', 'event', true);
      return;
    }
    const heirs = this.states.filter((s) => s.def.successorOf === 'jin');
    const cities = this.ownCities(jinId, true);
    const assign = new Map<number, City[]>();
    for (const c of cities) {
      let best = heirs[0];
      let bd = Infinity;
      for (const h of heirs) {
        const [lon, lat] = h.def.seat!;
        const d = Math.hypot(lonToX(lon) - c.x, latToY(lat) - c.y);
        if (d < bd) {
          bd = d;
          best = h;
        }
      }
      if (!assign.has(best.id)) assign.set(best.id, []);
      assign.get(best.id)!.push(c);
    }
    const active = heirs.filter((h) => assign.get(h.id)?.some((c) => c.built >= 1));
    if (!active.length) return;
    const share = 1 / active.length;
    for (const h of active) {
      const cs = assign.get(h.id)!;
      h.alive = true;
      h.everAlive = true;
      h.res = { food: jin.res.food * share, wood: jin.res.wood * share, metal: jin.res.metal * share };
      cs.sort((a, b) => b.level - a.level);
      for (const c of cs) {
        c.owner = h.id;
        c.capital = false;
      }
      cs[0].capital = true;
      // 独立の勢い: 新たな兵と資源
      h.res.food += 400;
      h.res.wood += 300;
      h.res.metal += 300;
      for (let k = 0; k < 3; k++) this.spawnUnit(h.id, k === 0 ? 'archer' : 'infantry', cs[0], 120);
    }
    for (const h of heirs) {
      if (active.includes(h)) continue;
      for (const c of assign.get(h.id) ?? []) {
        c.owner = active[0].id;
        c.capital = false;
      }
    }
    // ユニットを最寄りの継承国へ
    for (const u of this.units) {
      if (u.dead || u.owner !== jinId) continue;
      let best = active[0];
      let bd = Infinity;
      for (const h of active) {
        for (const c of this.cities) {
          if (c.owner !== h.id || c.destroyed) continue;
          const d = Math.hypot(c.x - u.x, c.y - u.y);
          if (d < bd) {
            bd = d;
            best = h;
          }
        }
      }
      u.owner = best.id;
      if (u.order.type === 'gather') u.order.dropId = -1;
    }
    for (const b of this.buildings) {
      if (b.destroyed || b.owner !== jinId) continue;
      let best = active[0];
      let bd = Infinity;
      for (const c of this.cities) {
        if (c.destroyed || !active.some((h) => h.id === c.owner)) continue;
        const d = Math.hypot(c.x - b.x, c.y - b.y);
        if (d < bd) {
          bd = d;
          best = this.states[c.owner];
        }
      }
      b.owner = best.id;
    }
    // 戦争の継承
    for (const e of this.enemiesOf(jinId)) {
      const w = this.getWar(jinId, e)!;
      for (const h of active) {
        this.wars.set(this.warKey(h.id, e), { a: h.id, b: e, since: w.since, forced: w.forced });
      }
    }
    for (const k of [...this.wars.keys()]) {
      const w = this.wars.get(k)!;
      if (w.a === jinId || w.b === jinId) this.wars.delete(k);
    }
    if (this.event) {
      if (this.event.target === jinId) this.endEvent(false);
      else if (this.event.members.includes(jinId)) {
        this.event.members = this.event.members.filter((m) => m !== jinId).concat(active.map((h) => h.id));
      }
    }
    jin.alive = false;
    jin.eliminatedYear = this.year;
    if (jin.isPlayer) {
      active.sort((a, b) => this.ownCities(b.id).length - this.ownCities(a.id).length);
      const np = active[0];
      jin.isPlayer = false;
      np.isPlayer = true;
      this.playerId = np.id;
      this.onEvent?.('三家分晋', `晋は韓・趙・魏の三家に分かれた。\nあなたは ${np.name} を率いて戦国の世を戦う。`);
    } else {
      this.onEvent?.('三家分晋', `晋は韓・趙・魏の三家に分かれ、それぞれ諸侯として認められた。`);
    }
    this.msg(`三家分晋 — 晋は ${active.map((h) => h.name).join('・')} に分かれた`, 'event', true);
    this.territoryDirty = true;
  }

  isSevenHero(s: State): boolean {
    return SEVEN_HEROES.includes(s.def.key);
  }
}

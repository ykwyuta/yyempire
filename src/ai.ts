import { BUILDING, NEW_CITY_COST, UNIT, canAfford, cityLevelCap, cityUpgradeCost, type BuildingKind, type ResKey, type UnitKind } from './config';
import type { City, Game, Order, State, Struct, Unit } from './game';
import { MW, T_FOREST, T_HILL, T_PLAIN, T_STEPPE } from './map';

/** 1.5 秒ごとの AI 思考 */
export function aiThink(g: Game, s: State): void {
  const cities = g.ownCities(s.id);
  if (!cities.length) return;
  const units = g.unitsOf(s.id);
  const workers = units.filter((u) => u.kind === 'worker');
  const soldiers = units.filter((u) => u.kind !== 'worker');
  const r = s.res;
  const enemies = g.enemiesOf(s.id);
  const atWar = enemies.length > 0;

  // ---- 工人の生産 ----
  const capW = g.workerCap(s.id);
  if (workers.length + g.queuedCount(s.id, false) < capW && r.food >= 60) {
    const free = cities.filter((c) => c.queue.length === 0);
    if (free.length) g.enqueue(free[Math.floor(g.rng() * free.length)], 'worker');
  }

  // ---- 採集配分 ----
  const upkeep = g.upkeepPerSec(s.id);
  const counts: Record<ResKey, number> = { food: 0, wood: 0, metal: 0 };
  for (const w of workers) if (w.order.type === 'gather') counts[w.order.res]++;
  const foodNeed = 0.45 + (r.food < 150 ? 0.2 : 0) + Math.min(0.25, upkeep * 0.4) - (r.food > 600 ? 0.2 : 0) - (r.food > 1500 ? 0.15 : 0);
  const woodNeed = r.wood > 1000 ? 0.1 : r.wood < 150 ? 0.4 : 0.3;
  const metalNeed = r.metal > 1000 ? 0.1 : 0.3;
  const total = foodNeed + woodNeed + metalNeed;
  const want: Record<ResKey, number> = {
    food: (foodNeed / total) * workers.length,
    wood: (woodNeed / total) * workers.length,
    metal: (metalNeed / total) * workers.length,
  };
  const keys: ResKey[] = ['food', 'wood', 'metal'];
  const pickRes = (): ResKey => {
    let best: ResKey = 'food';
    let bd = -Infinity;
    for (const k of keys) {
      const d = want[k] - counts[k];
      if (d > bd) {
        bd = d;
        best = k;
      }
    }
    return best;
  };
  for (const w of workers) {
    if (w.order.type !== 'idle') continue;
    const k = pickRes();
    if (g.autoGather(w, k)) {
      const o = w.order as Order;
      if (o.type === 'gather') counts[o.res]++;
    }
  }
  {
    let need: ResKey = 'food';
    let over: ResKey = 'food';
    let nd = -Infinity;
    let od = Infinity;
    for (const k of keys) {
      const d = want[k] - counts[k];
      if (d > nd) {
        nd = d;
        need = k;
      }
      if (d < od) {
        od = d;
        over = k;
      }
    }
    if (nd >= 0.9 && od <= -0.5 && need !== over) {
      const w = workers.find((x) => x.order.type === 'gather' && x.order.res === over && x.order.phase !== 'toDrop');
      if (w) g.autoGather(w, need);
    }
  }

  // ---- 建設 ----
  const underConstruction = g.ownCities(s.id, true).some((c) => c.built < 1) || g.ownBuildings(s.id, undefined, true).some((b) => b.built < 1);
  if (!underConstruction && workers.length >= 3 && g.rng() < 0.35) aiConstruct(g, s, cities, workers, atWar);

  // ---- 城の強化 ----
  for (const c of cities) {
    if (c.queue.length || c.level >= cityLevelCap(g.era)) continue;
    const cost = cityUpgradeCost(c.level);
    if (r.food > cost.food + 250 && r.wood > cost.wood + 100 && r.metal > cost.metal + 100 && g.rng() < 0.2) {
      g.enqueue(c, 'upgrade');
      break;
    }
  }

  // ---- 市での交易 ----
  if (g.ownBuildings(s.id, 'market').length && g.rng() < 0.3) {
    const rich = keys.slice().sort((a, b) => r[b] - r[a])[0];
    const poor = keys.slice().sort((a, b) => r[a] - r[b])[0];
    if (r[rich] > 700 && r[poor] < 150) g.trade(s.id, rich, poor, 100);
  }

  // ---- 軍備 ----
  const capS = g.soldierCap(s.id);
  let foodIncome = 0;
  for (const w of workers) if (w.order.type === 'gather' && w.order.res === 'food') foodIncome += 0.28 * (w.ability / 100) * s.bias;
  for (const c of cities) foodIncome += 0.2 * c.level * s.bias;
  const sustainable = foodIncome > upkeep * (atWar ? 1.0 : 1.4) + 0.15 || r.food > 600;
  const desired = atWar ? capS : Math.max(2, Math.round(capS * (0.45 + s.aggr * 0.35)));
  if (soldiers.length + g.queuedCount(s.id, true) < desired && sustainable && r.food > 90) {
    const kind = chooseKind(g, s, soldiers, atWar);
    const producers: Struct[] = [...g.ownBuildings(s.id, 'barracks'), ...cities].filter(
      (p) => p.queue.length < 2 && g.producible(p).includes(kind),
    );
    if (producers.length) g.enqueue(producers[0], kind);
  }

  // 訓練・補充
  for (const u of soldiers) {
    if (u.order.type !== 'idle') continue;
    if (u.ability < u.maxAbility * 0.7 && r.food > 200 && r.metal > 80) g.reinforce(u);
  }
  if (r.food > 350 && r.metal > 200 && g.rng() < 0.4) {
    const cand = soldiers.filter((u) => u.kind !== 'ram' && g.canTrain(u)).sort((a, b) => a.maxAbility - b.maxAbility)[0];
    if (cand) g.train(cand);
  }
  if (r.food > 300 && r.wood > 300 && g.rng() < 0.3) {
    const cand = workers.filter((u) => g.canTrain(u)).sort((a, b) => a.maxAbility - b.maxAbility)[0];
    if (cand) g.train(cand);
  }
  if (s.starving && soldiers.length > 2 && workers.length < capW) {
    const weakest = soldiers.slice().sort((a, b) => a.ability - b.ability)[0];
    const w = g.disband(weakest);
    if (w) g.autoGather(w, 'food');
  }

  armyControl(g, s, cities, soldiers, enemies);
}

function chooseKind(g: Game, s: State, soldiers: Unit[], atWar: boolean): UnitKind {
  const n: Record<string, number> = { infantry: 0, chariot: 0, archer: 0, cavalry: 0, ram: 0 };
  for (const u of soldiers) n[u.kind]++;
  const hasBarracks = g.ownBuildings(s.id, 'barracks').length > 0;
  if (!hasBarracks) return 'infantry';
  const ratios: [UnitKind, number][] = g.era === 'spring'
    ? [['infantry', 0.35], ['chariot', 0.3], ['archer', 0.22]]
    : g.kindAvailable('cavalry')
      ? [['infantry', 0.3], ['archer', 0.27], ['cavalry', 0.22], ['chariot', 0.08]]
      : [['infantry', 0.4], ['archer', 0.32], ['chariot', 0.15]];
  if (atWar) ratios.push(['ram', 0.13]);
  const tot = soldiers.length + 1;
  let best: UnitKind = 'infantry';
  let bd = -Infinity;
  for (const [k, w] of ratios) {
    if (!canAfford(s.res, UNIT[k].cost)) continue;
    const d = w * tot - n[k];
    if (d > bd) {
      bd = d;
      best = k;
    }
  }
  return best;
}

function nearestWorkers(workers: Unit[], x: number, y: number, n: number): Unit[] {
  return workers
    .filter((w) => w.order.type !== 'build')
    .sort((a, b) => Math.hypot(a.x - x, a.y - y) - Math.hypot(b.x - x, b.y - y))
    .slice(0, n);
}

function aiBuild(g: Game, s: State, kind: BuildingKind, near: { x: number; y: number }, workers: Unit[], rmin: number, rmax: number): boolean {
  if (!canAfford(s.res, BUILDING[kind].cost)) return false;
  const site = g.findBuildingSite(s.id, near.x, near.y, rmin, rmax, kind);
  if (!site) return false;
  const b = g.placeBuilding(s.id, kind, site[0], site[1], nearestWorkers(workers, site[0], site[1], kind === 'storehouse' ? 1 : 2));
  return !!b;
}

function aiConstruct(g: Game, s: State, cities: City[], workers: Unit[], atWar: boolean): void {
  const r = s.res;
  const cap = cities.find((c) => c.capital) ?? cities[0];
  // 兵営
  const barracks = g.ownBuildings(s.id, 'barracks', true);
  if (barracks.length < Math.min(2, Math.ceil(cities.length / 3)) && r.wood >= 170) {
    if (aiBuild(g, s, 'barracks', cap, workers, 2.5, 5)) return;
  }
  // 倉庫: 集積所から遠い採集地
  if (r.wood >= 110 && g.ownBuildings(s.id, 'storehouse', true).length < cities.length * 2) {
    for (const w of workers) {
      const o = w.order;
      if (o.type !== 'gather') continue;
      const drop = g.nearestDrop(s.id, o.tx, o.ty);
      if (drop && Math.hypot(drop.x - o.tx, drop.y - o.ty) > 6) {
        if (aiBuild(g, s, 'storehouse', { x: o.tx, y: o.ty }, workers, 1, 2.5)) return;
      }
    }
  }
  // 望楼
  if (atWar && r.wood >= 120 && r.metal >= 200 && g.rng() < 0.4) {
    const c = cities[Math.floor(g.rng() * cities.length)];
    const towers = g.ownBuildings(s.id, 'tower', true).filter((t) => Math.hypot(t.x - c.x, t.y - c.y) < 8).length;
    if (towers < 1 && aiBuild(g, s, 'tower', c, workers, 2, 4)) return;
  }
  // 砦: 戦時、国境寄りの城の外側に
  if (atWar && r.wood >= 250 && r.metal >= 300 && r.food >= 100 && g.rng() < 0.25) {
    const c = cities[Math.floor(g.rng() * cities.length)];
    const forts = g.ownBuildings(s.id, 'fort', true).filter((t) => Math.hypot(t.x - c.x, t.y - c.y) < 12).length;
    if (forts < 1 && aiBuild(g, s, 'fort', c, workers, 5, 8)) return;
  }
  // 市
  if (!g.ownBuildings(s.id, 'market', true).length && r.wood >= 350 && cities.length >= 2) {
    if (aiBuild(g, s, 'market', cap, workers, 3, 6)) return;
  }
  // 築城
  const maxCities = s.bias >= 1.3 ? 10 : s.bias >= 0.95 ? 5 : 3;
  if (cities.length < maxCities && canAfford(r, { food: NEW_CITY_COST.food + 120, wood: NEW_CITY_COST.wood + 40, metal: 0 }) && g.rng() < 0.3 * s.bias) {
    tryExpand(g, s, cities, workers);
  }
}

function tryExpand(g: Game, s: State, cities: City[], workers: Unit[]): void {
  let best: [number, number] | null = null;
  let bestScore = -Infinity;
  for (let t = 0; t < 40; t++) {
    const c = cities[Math.floor(g.rng() * cities.length)];
    const a = g.rng() * Math.PI * 2;
    const rr = 8 + g.rng() * 10;
    const tx = Math.floor(c.x + Math.cos(a) * rr);
    const ty = Math.floor(c.y + Math.sin(a) * rr);
    if (g.canFoundCity(s.id, tx, ty)) continue;
    let score = 0;
    for (let dy = -3; dy <= 3; dy++) {
      for (let dx = -3; dx <= 3; dx++) {
        const x = tx + dx;
        const y = ty + dy;
        if (!g.map.inBounds(x, y)) continue;
        const i = y * MW + x;
        const ter = g.map.terrain[i];
        if (g.map.owner[i] === -1) score += 1;
        if (ter === T_PLAIN) score += g.map.fertile[i] ? 1.5 : 1;
        else if (ter === T_FOREST || ter === T_STEPPE) score += 0.6;
        else if (ter === T_HILL) score += 0.3;
        if (g.map.mine[i]) score += 2;
      }
    }
    score -= rr * 0.5;
    if (score > bestScore) {
      bestScore = score;
      best = [tx, ty];
    }
  }
  if (!best) return;
  g.foundCity(s.id, best[0], best[1], nearestWorkers(workers, best[0], best[1], 2));
}

function armyControl(g: Game, s: State, cities: City[], soldiers: Unit[], enemies: number[]): void {
  for (const id of [...s.army]) {
    const u = g.unitById.get(id);
    if (!u || u.dead) s.army.delete(id);
  }
  // 撤退
  for (const u of soldiers) {
    if (u.ability < u.maxAbility * 0.3 && g.map.ownerAt(u.x, u.y) !== s.id && u.order.type !== 'move') {
      const home = g.nearestOwnCity(s.id, u.x, u.y);
      if (home) {
        s.army.delete(u.id);
        g.moveUnits([u], home.x + 1, home.y + 1, false);
      }
    }
  }
  if (!enemies.length) {
    for (const id of s.army) {
      const u = g.unitById.get(id);
      if (u && u.order.type !== 'move' && g.map.ownerAt(u.x, u.y) !== s.id) {
        const home = g.nearestOwnCity(s.id, u.x, u.y);
        if (home) g.moveUnits([u], home.x + 1.5, home.y + 1.5, false);
      }
    }
    s.army.clear();
    return;
  }
  // 防衛
  const enemySet = new Set(enemies);
  let intruder: Unit | null = null;
  const cap = cities.find((c) => c.capital) ?? cities[0];
  let bd = Infinity;
  for (const u of g.units) {
    if (u.dead || !enemySet.has(u.owner) || u.kind === 'worker') continue;
    if (g.map.ownerAt(u.x, u.y) !== s.id) continue;
    const d = Math.hypot(u.x - cap.x, u.y - cap.y);
    if (d < bd) {
      bd = d;
      intruder = u;
    }
  }
  const home = soldiers.filter((u) => !s.army.has(u.id));
  if (intruder) {
    const defenders = home.filter((u) => u.order.type === 'idle' && u.kind !== 'ram' && Math.hypot(u.x - intruder!.x, u.y - intruder!.y) < 30);
    if (defenders.length) g.attackTarget(defenders, { unit: intruder });
  }

  // 攻撃目標国
  let target = -1;
  const ev = g.event;
  if (ev && ev.members.includes(s.id) && enemySet.has(ev.target)) target = ev.target;
  else if (s.warTarget >= 0 && enemySet.has(s.warTarget)) target = s.warTarget;
  else {
    let best = Infinity;
    for (const e of enemies) {
      const d = g.minCityDistance(s.id, e);
      if (d < best) {
        best = d;
        target = e;
      }
    }
  }
  if (target < 0) return;
  if (ev && ev.target === s.id && soldiers.length < 6) return;

  let tc: City | null = null;
  let td = Infinity;
  for (const c of g.cities) {
    if (c.destroyed || c.owner !== target) continue;
    for (const oc of cities) {
      const d = Math.hypot(c.x - oc.x, c.y - oc.y) + (c.built < 1 ? -5 : 0);
      if (d < td) {
        td = d;
        tc = c;
      }
    }
  }
  if (!tc || td > 90) return;

  const strengthNeeded = Math.max(3, Math.round(soldiers.length * 0.6));
  const avail = home.filter((u) => u.order.type === 'idle' && u.ability >= u.maxAbility * 0.65);
  const keep = ev && ev.target === s.id ? 2 : 1;
  const sendable = avail.slice(0, Math.max(0, avail.length - keep));
  if (s.army.size + sendable.length >= strengthNeeded && sendable.length) {
    for (const u of sendable) s.army.add(u.id);
  }
  const army: Unit[] = [];
  for (const id of s.army) {
    const u = g.unitById.get(id);
    if (u && !u.dead) army.push(u);
  }
  const idleArmy = army.filter((u) => u.order.type === 'idle');
  if (idleArmy.length) g.moveUnits(idleArmy, tc.x, tc.y, true);
  if (army.length && army.length < 2 && g.rng() < 0.3) {
    for (const u of army) {
      s.army.delete(u.id);
      const h = g.nearestOwnCity(s.id, u.x, u.y);
      if (h) g.moveUnits([u], h.x + 1, h.y + 1, false);
    }
  }
}

/** 年に一度の外交判断 */
export function aiYearly(g: Game, s: State): void {
  const my = g.milStrength(s.id) + 30;
  const enemies = g.enemiesOf(s.id);
  for (const e of enemies) {
    const w = g.getWar(s.id, e);
    if (!w || g.isForcedWar(s.id, e)) continue;
    const dur = g.year - w.since;
    const theirs = g.milStrength(e) + 30;
    const losing = my < theirs * 0.6;
    const stale = dur >= 10 && g.rng() < 0.2;
    if ((dur >= 4 && losing) || stale) {
      const other = g.states[e];
      if (other.isPlayer) {
        if (!s.peaceOffers.has(e)) {
          s.peaceOffers.add(e);
          g.msg(`${s.name} が講和を申し入れてきた（諸国一覧から受諾できる）`, 'info', true);
        }
      } else if (theirs < my * 2.0 || g.rng() < 0.3) g.makePeace(s.id, e);
    }
  }
  if (s.warTarget >= 0 && !g.atWar(s.id, s.warTarget)) s.warTarget = -1;

  const ev = g.event;
  if (ev && (ev.target === s.id || ev.members.includes(s.id))) return;
  const nonForced = enemies.filter((e) => !g.isForcedWar(s.id, e));
  if (nonForced.length >= 1) return;
  if (g.year < -755) return; // 序盤の猶予
  const caution = g.era === 'spring' ? Math.max(0, 1 - s.aggro / 90) : 1;
  if (g.rng() > s.aggr * 0.22 * caution) return;
  if (g.soldierCount(s.id) < 3) return;
  let best = -1;
  let bestScore = 0;
  for (const o of g.states) {
    if (!o.alive || o.id === s.id) continue;
    const d = g.minCityDistance(s.id, o.id);
    if (d > 35) continue;
    const theirs = g.milStrength(o.id) + 30;
    const ratio = my / theirs;
    const need = o.bias >= 1.3 ? (s.bias >= 1.3 ? 1.6 : 2.2) : 1.3;
    if (ratio < need) continue;
    const respect = o.def.key === 'zhou' && g.era === 'spring' ? 0.3 : 1;
    const score = (ratio / (d + 10)) * respect * (0.7 + g.rng() * 0.6);
    if (score > bestScore) {
      bestScore = score;
      best = o.id;
    }
  }
  if (best >= 0) {
    g.declareWar(s.id, best);
    s.warTarget = best;
  }
}

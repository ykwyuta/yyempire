import { NEW_CITY_COST, UNIT, abilityCap, canAfford, cityLevelCap, cityUpgradeCost, type ResKey, type UnitKind } from './config';
import type { City, Game, Order, State, Unit } from './game';
import { MW, T_FOREST, T_HILL, T_PLAIN, T_STEPPE } from './map';

/** 1.5 秒ごとの AI 思考 */
export function aiThink(g: Game, s: State): void {
  const cities = g.ownCities(s.id);
  if (!cities.length) return;
  const units = g.unitsOf(s.id);
  const workers = units.filter((u) => u.kind === 'worker');
  const soldiers = units.filter((u) => u.kind !== 'worker');
  const r = s.res;

  // ---- 経済 ----
  const capW = g.workerCap(s.id);
  const freeCities = cities.filter((c) => !c.queue);
  if (workers.length + g.queuedCount(s.id, false) < capW && r.food >= 60 && freeCities.length) {
    g.enqueue(freeCities[Math.floor(g.rng() * freeCities.length)], 'worker');
  }

  const upkeep = g.upkeepPerSec(s.id);
  const counts: Record<ResKey, number> = { food: 0, wood: 0, metal: 0 };
  for (const w of workers) if (w.order.type === 'gather') counts[w.order.res]++;
  const foodNeed = 0.45 + (r.food < 150 ? 0.2 : 0) + Math.min(0.25, upkeep * 0.4) - (r.food > 600 ? 0.2 : 0) - (r.food > 1500 ? 0.15 : 0);
  const woodNeed = r.wood > 1000 ? 0.1 : 0.3;
  const metalNeed = r.metal > 1000 ? 0.1 : 0.3;
  const total = foodNeed + woodNeed + metalNeed;
  const want: Record<ResKey, number> = {
    food: (foodNeed / total) * workers.length,
    wood: (woodNeed / total) * workers.length,
    metal: (metalNeed / total) * workers.length,
  };
  const pickRes = (): ResKey => {
    let best: ResKey = 'food';
    let bd = -Infinity;
    for (const k of ['food', 'wood', 'metal'] as ResKey[]) {
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
  // 採集配分の付け替え（1 回の思考で 1 人まで）
  {
    const keys: ResKey[] = ['food', 'wood', 'metal'];
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
      const w = workers.find((x) => x.order.type === 'gather' && x.order.res === over && x.order.phase !== 'toCity');
      if (w) g.autoGather(w, need);
    }
  }

  // ---- 築城 ----
  const building = g.ownCities(s.id, true).some((c) => c.built < 1);
  const maxCities = s.bias >= 1.3 ? 10 : s.bias >= 0.95 ? 5 : 3;
  if (!building && cities.length < maxCities && canAfford(r, { food: NEW_CITY_COST.food + 120, wood: NEW_CITY_COST.wood + 40, metal: 0 }) && workers.length >= 3 && g.rng() < 0.1 * s.bias) {
    tryExpand(g, s, cities, workers);
  }

  // ---- 城の強化 ----
  for (const c of freeCities) {
    if (c.queue || c.level >= cityLevelCap(g.era)) continue;
    const cost = cityUpgradeCost(c.level);
    if (r.food > cost.food + 250 && r.wood > cost.wood + 100 && r.metal > cost.metal + 100 && g.rng() < 0.2) {
      g.enqueue(c, 'upgrade');
      break;
    }
  }

  // ---- 軍備 ----
  const capS = g.soldierCap(s.id);
  const enemies = g.enemiesOf(s.id);
  const atWar = enemies.length > 0;
  let foodIncome = 0;
  for (const w of workers) if (w.order.type === 'gather' && w.order.res === 'food') foodIncome += 0.28 * (w.ability / 100) * s.bias;
  for (const c of cities) foodIncome += 0.2 * c.level * s.bias;
  const sustainable = foodIncome > upkeep * (atWar ? 1.0 : 1.4) + 0.15 || r.food > 600;
  const desired = atWar ? capS : Math.max(2, Math.round(capS * (0.45 + s.aggr * 0.35)));
  if (soldiers.length + g.queuedCount(s.id, true) < desired && sustainable && r.food > 90) {
    const free = cities.filter((c) => !c.queue);
    if (free.length) {
      const kind = chooseKind(g, s, soldiers);
      g.enqueue(free[Math.floor(g.rng() * free.length)], kind);
    }
  }

  // 訓練・補充
  for (const u of soldiers) {
    if (u.order.type !== 'idle') continue;
    if (u.ability < u.maxAbility * 0.7 && r.food > 200 && r.metal > 80) g.reinforce(u);
  }
  if (r.food > 350 && r.metal > 200 && g.rng() < 0.4) {
    const cand = soldiers.filter((u) => g.canTrain(u)).sort((a, b) => a.maxAbility - b.maxAbility)[0];
    if (cand) g.train(cand);
  }
  if (r.food > 300 && r.wood > 300 && g.rng() < 0.3) {
    const cand = workers.filter((u) => g.canTrain(u)).sort((a, b) => a.maxAbility - b.maxAbility)[0];
    if (cand) g.train(cand);
  }
  // 兵が多すぎて兵糧が尽きそうなら解散
  if (s.starving && soldiers.length > 2 && workers.length < capW) {
    const weakest = soldiers.slice().sort((a, b) => a.ability - b.ability)[0];
    const w = g.disband(weakest);
    if (w) g.autoGather(w, 'food');
  }

  // ---- 軍事行動 ----
  armyControl(g, s, cities, soldiers, enemies);
}

function chooseKind(g: Game, s: State, soldiers: Unit[]): UnitKind {
  const n: Record<string, number> = { infantry: 0, chariot: 0, archer: 0, cavalry: 0 };
  for (const u of soldiers) n[u.kind]++;
  const ratios: [UnitKind, number][] = g.era === 'spring'
    ? [['infantry', 0.4], ['chariot', 0.35], ['archer', 0.25]]
    : g.kindAvailable('cavalry')
      ? [['infantry', 0.35], ['archer', 0.3], ['cavalry', 0.25], ['chariot', 0.1]]
      : [['infantry', 0.45], ['archer', 0.35], ['chariot', 0.2]];
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
  const builders = workers
    .slice()
    .sort((a, b) => Math.hypot(a.x - best![0], a.y - best![1]) - Math.hypot(b.x - best![0], b.y - best![1]))
    .slice(0, 2);
  g.foundCity(s.id, best[0], best[1], builders);
}

function armyControl(g: Game, s: State, cities: City[], soldiers: Unit[], enemies: number[]): void {
  // 部隊の掃除
  for (const id of [...s.army]) {
    const u = g.unitById.get(id);
    if (!u || u.dead) s.army.delete(id);
  }
  // 撤退: 消耗した兵を帰還させる
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
    // 平時: 部隊を帰還
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
  // 防衛: 領内の敵
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
    const defenders = home.filter((u) => u.order.type === 'idle' && Math.hypot(u.x - intruder!.x, u.y - intruder!.y) < 30);
    if (defenders.length) g.attackTarget(defenders, { unit: intruder });
  }

  // 攻撃目標国の決定
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
  // 包囲されている国は防衛優先
  if (ev && ev.target === s.id && soldiers.length < 6) return;

  // 目標都市
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
  if (idleArmy.length) {
    g.moveUnits(idleArmy, tc.x, tc.y, true);
  }
  if (army.length && army.length < 2 && g.rng() < 0.3) {
    // 小勢は帰還
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
  // 講和
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
      } else {
        const otherMy = theirs;
        if (otherMy < my * 2.0 || g.rng() < 0.3) g.makePeace(s.id, e);
      }
    }
  }
  if (s.warTarget >= 0 && !g.atWar(s.id, s.warTarget)) s.warTarget = -1;

  // 宣戦
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
    // 大国は大国との正面衝突を避け、小国は大国を恐れる
    const need = o.bias >= 1.3 ? (s.bias >= 1.3 ? 1.6 : 2.2) : 1.3;
    if (ratio < need) continue;
    // 周王室は攻撃しにくい（春秋）
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

export function abilityHeadroom(g: Game, u: Unit): number {
  return abilityCap(g.era) - u.maxAbility;
}

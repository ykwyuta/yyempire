import { LAKES, RANGES, RIVERS, SEA_POLY, type LonLat } from './data/geo';
import { fbm, hash2 } from './rng';
import { FOREST_AMOUNT, MINE_AMOUNT } from './config';

export const LON0 = 102;
export const LAT1 = 42.5;
export const DEG = 0.1;
export const MW = 220;
export const MH = 210;

export const T_SEA = 0;
export const T_LAKE = 1;
export const T_PLAIN = 2;
export const T_STEPPE = 3;
export const T_FOREST = 4;
export const T_HILL = 5;
export const T_MOUNTAIN = 6;
export const T_DESERT = 7;

export const TERRAIN_NAMES = ['海', '湖', '平原', '草原', '森林', '丘陵', '山岳', '荒地'];
const BASE_COST = [Infinity, Infinity, 1, 1, 1.6, 2.2, 5, 1.5];

export const lonToX = (lon: number): number => (lon - LON0) / DEG;
export const latToY = (lat: number): number => (LAT1 - lat) / DEG;

function pointInPoly(x: number, y: number, poly: LonLat[]): boolean {
  let inside = false;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const [xi, yi] = poly[i];
    const [xj, yj] = poly[j];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}

function distToSeg(px: number, py: number, ax: number, ay: number, bx: number, by: number): number {
  const dx = bx - ax;
  const dy = by - ay;
  const l2 = dx * dx + dy * dy;
  let t = l2 > 0 ? ((px - ax) * dx + (py - ay) * dy) / l2 : 0;
  t = Math.max(0, Math.min(1, t));
  const cx = ax + t * dx - px;
  const cy = ay + t * dy - py;
  return Math.sqrt(cx * cx + cy * cy);
}

function distToPolyline(px: number, py: number, pts: LonLat[]): number {
  let d = Infinity;
  for (let i = 0; i + 1 < pts.length; i++) d = Math.min(d, distToSeg(px, py, pts[i][0], pts[i][1], pts[i + 1][0], pts[i + 1][1]));
  return d;
}

export type GatherType = 'food' | 'wood' | 'metal' | null;

export class GameMap {
  readonly w = MW;
  readonly h = MH;
  terrain = new Uint8Array(MW * MH);
  river = new Uint8Array(MW * MH);
  fertile = new Uint8Array(MW * MH);
  mine = new Uint8Array(MW * MH);
  cost = new Float32Array(MW * MH);
  /** 森林・鉱山の残量 */
  amount = new Float32Array(MW * MH);
  /** 地形が変化したタイル（描画側が消費する） */
  changed: number[] = [];
  /** 領有国 (-1 = 無主) */
  owner = new Int16Array(MW * MH).fill(-1);

  // A* 用の作業領域
  private gScore = new Float32Array(MW * MH);
  private came = new Int32Array(MW * MH);
  private stamp = new Uint32Array(MW * MH);
  private closed = new Uint32Array(MW * MH);
  private gen = 0;
  private heapN = new Int32Array(MW * MH * 2);
  private heapF = new Float32Array(MW * MH * 2);

  constructor(seed: number, cityTiles: [number, number][]) {
    this.generate(seed, cityTiles);
  }

  idx(x: number, y: number): number {
    return y * MW + x;
  }
  inBounds(x: number, y: number): boolean {
    return x >= 0 && y >= 0 && x < MW && y < MH;
  }
  terrainAt(x: number, y: number): number {
    const tx = Math.floor(x);
    const ty = Math.floor(y);
    if (!this.inBounds(tx, ty)) return T_SEA;
    return this.terrain[ty * MW + tx];
  }
  passable(x: number, y: number): boolean {
    const tx = Math.floor(x);
    const ty = Math.floor(y);
    if (!this.inBounds(tx, ty)) return false;
    return this.cost[ty * MW + tx] < Infinity;
  }
  costAt(x: number, y: number): number {
    const tx = Math.floor(x);
    const ty = Math.floor(y);
    if (!this.inBounds(tx, ty)) return Infinity;
    return this.cost[ty * MW + tx];
  }
  ownerAt(x: number, y: number): number {
    const tx = Math.floor(x);
    const ty = Math.floor(y);
    if (!this.inBounds(tx, ty)) return -1;
    return this.owner[ty * MW + tx];
  }

  gatherType(tx: number, ty: number): GatherType {
    if (!this.inBounds(tx, ty)) return null;
    const i = ty * MW + tx;
    if (this.mine[i]) return 'metal';
    const t = this.terrain[i];
    if (t === T_FOREST) return 'wood';
    if (t === T_PLAIN || t === T_STEPPE) return 'food';
    return null;
  }

  /** タイルの採集効率 */
  gatherYield(tx: number, ty: number): number {
    const i = ty * MW + tx;
    if (this.mine[i]) return 0.35;
    const t = this.terrain[i];
    if (t === T_FOREST) return 0.5;
    if (t === T_PLAIN) return this.fertile[i] ? 0.62 : 0.48;
    if (t === T_STEPPE) return 0.3;
    return 0;
  }

  private generate(seed: number, cityTiles: [number, number][]): void {
    for (let y = 0; y < MH; y++) {
      for (let x = 0; x < MW; x++) {
        const lon = LON0 + (x + 0.5) * DEG;
        const lat = LAT1 - (y + 0.5) * DEG;
        const i = y * MW + x;
        if (pointInPoly(lon, lat, SEA_POLY)) {
          this.terrain[i] = T_SEA;
          continue;
        }
        let lake = false;
        for (const [cx, cy, rx, ry] of LAKES) {
          const dx = (lon - cx) / rx;
          const dy = (lat - cy) / ry;
          if (dx * dx + dy * dy < 1) lake = true;
        }
        if (lake) {
          this.terrain[i] = T_LAKE;
          continue;
        }
        const n = fbm(x * 0.09, y * 0.09, seed);
        let mval = -1;
        for (const r of RANGES) {
          const d = distToPolyline(lon, lat, r.pts);
          mval = Math.max(mval, 1 - d / r.w);
        }
        mval += (n - 0.5) * 0.6;
        // 西方の高原
        if (lon < 103.3 && lat > 31.5) mval = Math.max(mval, 0.2 + (n - 0.5) * 0.8);
        if (mval > 0.5) {
          this.terrain[i] = T_MOUNTAIN;
          continue;
        }
        if (mval > 0) {
          this.terrain[i] = T_HILL;
          continue;
        }
        const n2 = fbm(x * 0.12 + 50, y * 0.12 + 50, seed + 7);
        const ordos = lon > 106.6 && lon < 110.8 && lat > 37.8 && lat < 40.6;
        if (ordos && n2 > 0.45) {
          this.terrain[i] = T_DESERT;
          continue;
        }
        if (ordos || lat > 41.0 || (lat > 40.4 && lon < 115.5) || (lon < 106 && lat > 36.3)) {
          this.terrain[i] = n2 < 0.3 ? T_FOREST : T_STEPPE;
          continue;
        }
        let fp: number;
        if (lat < 26) fp = 0.58;
        else if (lat < 30) fp = 0.5;
        else if (lat < 33) fp = 0.38;
        else if (lat < 38) fp = 0.3;
        else fp = 0.36;
        const sichuan = lon > 103.6 && lon < 107.2 && lat > 29.2 && lat < 31.8;
        if (sichuan) fp = 0.25;
        this.terrain[i] = n2 < fp ? T_FOREST : T_PLAIN;
      }
    }
    // 河川のラスタライズ（山は谷として丘陵に）
    for (const r of RIVERS) {
      for (let k = 0; k + 1 < r.pts.length; k++) {
        const ax = lonToX(r.pts[k][0]);
        const ay = latToY(r.pts[k][1]);
        const bx = lonToX(r.pts[k + 1][0]);
        const by = latToY(r.pts[k + 1][1]);
        const len = Math.hypot(bx - ax, by - ay);
        const steps = Math.ceil(len / 0.3);
        for (let s = 0; s <= steps; s++) {
          const px = Math.floor(ax + ((bx - ax) * s) / steps);
          const py = Math.floor(ay + ((by - ay) * s) / steps);
          if (!this.inBounds(px, py)) continue;
          const i = py * MW + px;
          const t = this.terrain[i];
          if (t === T_SEA || t === T_LAKE) continue;
          this.river[i] = 1;
          if (t === T_MOUNTAIN) this.terrain[i] = T_HILL;
        }
      }
    }
    // 肥沃度
    for (let y = 0; y < MH; y++) {
      for (let x = 0; x < MW; x++) {
        if (!this.river[y * MW + x]) continue;
        for (let dy = -3; dy <= 3; dy++) {
          for (let dx = -3; dx <= 3; dx++) {
            const nx = x + dx;
            const ny = y + dy;
            if (this.inBounds(nx, ny) && dx * dx + dy * dy <= 10) this.fertile[ny * MW + nx] = 1;
          }
        }
      }
    }
    // 鉱山
    for (let y = 0; y < MH; y++) {
      for (let x = 0; x < MW; x++) {
        const i = y * MW + x;
        if (this.terrain[i] === T_HILL && hash2(x, y, seed + 31) < 0.045) this.mine[i] = 1;
      }
    }
    // 都市周辺の保証
    for (const [cx, cy] of cityTiles) this.prepareCitySite(cx, cy, seed);
    this.recomputeCost();
    for (let i = 0; i < MW * MH; i++) {
      if (this.mine[i]) this.amount[i] = MINE_AMOUNT;
      else if (this.terrain[i] === T_FOREST) this.amount[i] = FOREST_AMOUNT;
    }
  }

  prepareCitySite(cx: number, cy: number, seed: number): void {
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        const x = cx + dx;
        const y = cy + dy;
        if (!this.inBounds(x, y)) continue;
        const i = y * MW + x;
        if (this.terrain[i] !== T_SEA && this.terrain[i] !== T_LAKE) {
          this.terrain[i] = T_PLAIN;
          this.mine[i] = 0;
        }
      }
    }
    const near = (pred: (i: number) => boolean, r: number): boolean => {
      for (let dy = -r; dy <= r; dy++) {
        for (let dx = -r; dx <= r; dx++) {
          const x = cx + dx;
          const y = cy + dy;
          if (this.inBounds(x, y) && pred(y * MW + x)) return true;
        }
      }
      return false;
    };
    const place = (fn: (i: number) => void, rmin: number, rmax: number, k: number): void => {
      for (let tries = 0; tries < 60; tries++) {
        const a = hash2(cx * 7 + tries, cy * 13 + k, seed) * Math.PI * 2;
        const r = rmin + hash2(cx + tries, cy - k, seed + 3) * (rmax - rmin);
        const x = Math.round(cx + Math.cos(a) * r);
        const y = Math.round(cy + Math.sin(a) * r);
        if (!this.inBounds(x, y)) continue;
        const t = this.terrain[y * MW + x];
        if (t === T_SEA || t === T_LAKE) continue;
        fn(y * MW + x);
        return;
      }
    };
    if (!near((i) => this.terrain[i] === T_FOREST, 5)) {
      place((i) => {
        this.terrain[i] = T_FOREST;
        this.mine[i] = 0;
        const x = i % MW;
        const y = (i / MW) | 0;
        for (const [dx, dy] of [[1, 0], [0, 1], [1, 1]]) {
          if (this.inBounds(x + dx, y + dy)) {
            const j = (y + dy) * MW + x + dx;
            if (this.terrain[j] !== T_SEA && this.terrain[j] !== T_LAKE) this.terrain[j] = T_FOREST;
          }
        }
      }, 3, 5, 1);
    }
    if (!near((i) => this.mine[i] === 1, 6)) {
      place((i) => {
        this.terrain[i] = T_HILL;
        this.mine[i] = 1;
      }, 3, 5, 2);
    }
    for (let dy = -6; dy <= 6; dy++) {
      for (let dx = -6; dx <= 6; dx++) {
        const x = cx + dx;
        const y = cy + dy;
        if (!this.inBounds(x, y)) continue;
        const i = y * MW + x;
        if (this.mine[i] && this.amount[i] <= 0) this.amount[i] = MINE_AMOUNT;
        else if (this.terrain[i] === T_FOREST && this.amount[i] <= 0) this.amount[i] = FOREST_AMOUNT;
        this.changed.push(i);
      }
    }
    if (!near((i) => this.terrain[i] === T_PLAIN && this.fertile[i] === 1, 3)) {
      // 平原を少し広げる
      for (let dy = -2; dy <= 2; dy++) {
        for (let dx = -2; dx <= 2; dx++) {
          const x = cx + dx;
          const y = cy + dy;
          if (!this.inBounds(x, y)) continue;
          const i = y * MW + x;
          if (this.terrain[i] === T_MOUNTAIN || this.terrain[i] === T_DESERT) this.terrain[i] = T_PLAIN;
        }
      }
    }
  }

  /** 資源を採る。枯渇すると地形が変わる（森林→平原、鉱山→廃坑） */
  harvest(tx: number, ty: number, amt: number): number {
    const i = ty * MW + tx;
    if (!this.mine[i] && this.terrain[i] !== T_FOREST) return amt; // 平原・草原は無尽蔵
    const got = Math.min(amt, this.amount[i]);
    this.amount[i] -= got;
    if (this.amount[i] <= 0) {
      if (this.mine[i]) this.mine[i] = 0;
      else {
        this.terrain[i] = T_PLAIN;
        this.cost[i] = 1 + (this.river[i] ? 2 : 0);
      }
      this.changed.push(i);
    }
    return got;
  }

  recomputeCost(): void {
    for (let i = 0; i < MW * MH; i++) {
      this.cost[i] = BASE_COST[this.terrain[i]] + (this.river[i] ? 2.0 : 0);
    }
  }

  nearestPassable(x: number, y: number, maxR = 12): [number, number] | null {
    const tx = Math.floor(x);
    const ty = Math.floor(y);
    if (this.passable(tx, ty)) return [tx, ty];
    for (let r = 1; r <= maxR; r++) {
      let best: [number, number] | null = null;
      let bd = Infinity;
      for (let dy = -r; dy <= r; dy++) {
        for (let dx = -r; dx <= r; dx++) {
          if (Math.max(Math.abs(dx), Math.abs(dy)) !== r) continue;
          if (this.passable(tx + dx, ty + dy)) {
            const d = dx * dx + dy * dy;
            if (d < bd) {
              bd = d;
              best = [tx + dx, ty + dy];
            }
          }
        }
      }
      if (best) return best;
    }
    return null;
  }

  /** A* 経路探索。戻り値は通過点（タイル中心, 最後は目的地そのもの） */
  findPath(sx: number, sy: number, gx: number, gy: number, maxNodes = 30000): { x: number; y: number }[] | null {
    const s0 = this.nearestPassable(sx, sy);
    const g0 = this.nearestPassable(gx, gy);
    if (!s0 || !g0) return null;
    const [stx, sty] = s0;
    const [gtx, gty] = g0;
    const start = sty * MW + stx;
    const goal = gty * MW + gtx;
    const endPoint = this.passable(gx, gy) ? { x: gx, y: gy } : { x: gtx + 0.5, y: gty + 0.5 };
    if (start === goal) return [endPoint];
    this.gen++;
    const gen = this.gen;
    const gS = this.gScore;
    const came = this.came;
    const stamp = this.stamp;
    const closed = this.closed;
    const hn = this.heapN;
    const hf = this.heapF;
    let size = 0;
    const push = (n: number, f: number): void => {
      let i = size++;
      while (i > 0) {
        const p = (i - 1) >> 1;
        if (hf[p] <= f) break;
        hn[i] = hn[p];
        hf[i] = hf[p];
        i = p;
      }
      hn[i] = n;
      hf[i] = f;
    };
    const pop = (): number => {
      const top = hn[0];
      size--;
      if (size > 0) {
        const n = hn[size];
        const f = hf[size];
        let i = 0;
        for (;;) {
          let c = 2 * i + 1;
          if (c >= size) break;
          if (c + 1 < size && hf[c + 1] < hf[c]) c++;
          if (hf[c] >= f) break;
          hn[i] = hn[c];
          hf[i] = hf[c];
          i = c;
        }
        hn[i] = n;
        hf[i] = f;
      }
      return top;
    };
    const h = (n: number): number => {
      const x = n % MW;
      const y = (n / MW) | 0;
      const dx = Math.abs(x - gtx);
      const dy = Math.abs(y - gty);
      return (dx + dy + (Math.SQRT2 - 2) * Math.min(dx, dy)) * 0.98;
    };
    stamp[start] = gen;
    gS[start] = 0;
    came[start] = -1;
    push(start, h(start));
    let expanded = 0;
    let found = false;
    let bestNode = start;
    let bestH = h(start);
    const cost = this.cost;
    while (size > 0) {
      const cur = pop();
      if (closed[cur] === gen) continue;
      closed[cur] = gen;
      if (cur === goal) {
        found = true;
        break;
      }
      const hc = h(cur);
      if (hc < bestH) {
        bestH = hc;
        bestNode = cur;
      }
      if (++expanded > maxNodes) break;
      const cx = cur % MW;
      const cy = (cur / MW) | 0;
      const cc = cost[cur];
      for (let dy = -1; dy <= 1; dy++) {
        const ny = cy + dy;
        if (ny < 0 || ny >= MH) continue;
        for (let dx = -1; dx <= 1; dx++) {
          if (dx === 0 && dy === 0) continue;
          const nx = cx + dx;
          if (nx < 0 || nx >= MW) continue;
          const ni = ny * MW + nx;
          const nc = cost[ni];
          if (nc === Infinity || closed[ni] === gen) continue;
          let step = (cc + nc) * 0.5;
          if (dx !== 0 && dy !== 0) {
            if (cost[cy * MW + nx] === Infinity || cost[ny * MW + cx] === Infinity) continue;
            step *= Math.SQRT2;
          }
          const ng = gS[cur] + step;
          if (stamp[ni] !== gen || ng < gS[ni]) {
            stamp[ni] = gen;
            gS[ni] = ng;
            came[ni] = cur;
            push(ni, ng + h(ni));
          }
        }
      }
    }
    const end = found ? goal : bestNode;
    const tiles: number[] = [];
    for (let n = end; n !== -1 && n !== start; n = came[n]) tiles.push(n);
    tiles.reverse();
    // 共線点の除去
    const pts: { x: number; y: number }[] = [];
    let pdx = 0;
    let pdy = 0;
    let prev = start;
    for (let k = 0; k < tiles.length; k++) {
      const n = tiles[k];
      const dx = (n % MW) - (prev % MW);
      const dy = ((n / MW) | 0) - ((prev / MW) | 0);
      if (k > 0 && (dx !== pdx || dy !== pdy)) pts.push({ x: (prev % MW) + 0.5, y: ((prev / MW) | 0) + 0.5 });
      pdx = dx;
      pdy = dy;
      prev = n;
    }
    if (found) pts.push(endPoint);
    else pts.push({ x: (end % MW) + 0.5, y: ((end / MW) | 0) + 0.5 });
    return pts;
  }
}

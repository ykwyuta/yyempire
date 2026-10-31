import { BUILDING, cityRadius, type BuildingKind } from './config';
import { RIVERS } from './data/geo';
import { structMaxHp, type Building, type City, type Game, type Struct, type Unit } from './game';
import { buildingSvg, iconImage, unitSvg } from './icons';
import { MH, MW, T_DESERT, T_FOREST, T_HILL, T_LAKE, T_MOUNTAIN, T_PLAIN, T_SEA, T_STEPPE, latToY, lonToX } from './map';
import { hash2 } from './rng';

const PX = 4; // 地形キャンバスの 1 タイルあたりピクセル

const TCOL: Record<number, [number, number, number]> = {
  [T_SEA]: [43, 86, 128],
  [T_LAKE]: [58, 108, 150],
  [T_PLAIN]: [168, 178, 112],
  [T_STEPPE]: [190, 180, 122],
  [T_FOREST]: [78, 116, 62],
  [T_HILL]: [150, 138, 98],
  [T_MOUNTAIN]: [120, 104, 86],
  [T_DESERT]: [214, 194, 142],
};

export interface Camera {
  x: number;
  y: number;
  zoom: number;
}

export interface DrawState {
  selected: Set<number>;
  selStruct: number;
  dragRect: { x0: number; y0: number; x1: number; y1: number } | null;
  buildPreview: { tx: number; ty: number; ok: boolean; kind: BuildingKind | 'city' } | null;
  mode: 'normal' | 'attackMove' | 'build' | 'rally';
  buildKind: BuildingKind | 'city';
  hover: { x: number; y: number } | null;
}

export class Renderer {
  terrain: HTMLCanvasElement;
  territory: HTMLCanvasElement;
  fog: HTMLCanvasElement;
  private terrVersion = -1;
  private fogVersion = -1;
  private terrainImg: ImageData | null = null;
  cam: Camera = { x: 80, y: 60, zoom: 10 };
  private ctx: CanvasRenderingContext2D;
  private mm: CanvasRenderingContext2D;
  private mmTimer = 0;
  private dpr = 1;

  constructor(private g: Game, private canvas: HTMLCanvasElement, minimap: HTMLCanvasElement) {
    this.ctx = canvas.getContext('2d')!;
    this.mm = minimap.getContext('2d')!;
    this.terrain = this.buildTerrain();
    this.territory = document.createElement('canvas');
    this.territory.width = MW;
    this.territory.height = MH;
    this.fog = document.createElement('canvas');
    this.fog.width = MW;
    this.fog.height = MH;
    minimap.width = MW;
    minimap.height = MH;
  }

  resize(): void {
    this.dpr = Math.min(2, window.devicePixelRatio || 1);
    this.canvas.width = Math.floor(this.canvas.clientWidth * this.dpr);
    this.canvas.height = Math.floor(this.canvas.clientHeight * this.dpr);
  }

  get viewW(): number {
    return this.canvas.clientWidth;
  }
  get viewH(): number {
    return this.canvas.clientHeight;
  }

  screenToWorld(sx: number, sy: number): { x: number; y: number } {
    return { x: sx / this.cam.zoom + this.cam.x, y: sy / this.cam.zoom + this.cam.y };
  }
  worldToScreen(x: number, y: number): { x: number; y: number } {
    return { x: (x - this.cam.x) * this.cam.zoom, y: (y - this.cam.y) * this.cam.zoom };
  }
  centerOn(x: number, y: number): void {
    this.cam.x = x - this.viewW / 2 / this.cam.zoom;
    this.cam.y = y - this.viewH / 2 / this.cam.zoom;
    this.clamp();
  }
  zoomAt(sx: number, sy: number, factor: number): void {
    const w = this.screenToWorld(sx, sy);
    this.cam.zoom = Math.max(2.5, Math.min(30, this.cam.zoom * factor));
    this.cam.x = w.x - sx / this.cam.zoom;
    this.cam.y = w.y - sy / this.cam.zoom;
    this.clamp();
  }
  clamp(): void {
    const vw = this.viewW / this.cam.zoom;
    const vh = this.viewH / this.cam.zoom;
    const mx = Math.min(vw * 0.15, 20);
    const my = Math.min(vh * 0.15, 20);
    this.cam.x = vw > MW ? (MW - vw) / 2 : Math.max(-mx, Math.min(MW - vw + mx, this.cam.x));
    this.cam.y = vh > MH ? (MH - vh) / 2 : Math.max(-my, Math.min(MH - vh + my, this.cam.y));
  }

  private paintTile(d: Uint8ClampedArray, cw: number, tx: number, ty: number): void {
    const m = this.g.map;
    const i = ty * MW + tx;
    const t = m.terrain[i];
    let [r, gg, b] = TCOL[t];
    if (t === T_PLAIN && m.fertile[i]) {
      r -= 18;
      gg += 4;
      b -= 10;
    }
    if (m.river[i] && t !== T_SEA && t !== T_LAKE) {
      r -= 6;
      gg += 2;
    }
    for (let py = 0; py < PX; py++) {
      for (let px = 0; px < PX; px++) {
        const X = tx * PX + px;
        const Y = ty * PX + py;
        const n = (hash2(X, Y, 5) - 0.5) * (t === T_SEA ? 4 : 8);
        let k = 0;
        if (t === T_MOUNTAIN) k = hash2(X >> 2, Y >> 1, 9) > 0.6 ? 22 : -4;
        if (t === T_FOREST) k = hash2(X >> 1, Y >> 1, 11) > 0.55 ? -12 : 4;
        if (t === T_HILL) k = hash2(X >> 2, Y >> 2, 13) > 0.5 ? 8 : -2;
        const o = (Y * cw + X) * 4;
        d[o] = r + n + k;
        d[o + 1] = gg + n + k;
        d[o + 2] = b + n + k;
        d[o + 3] = 255;
      }
    }
  }

  private drawTileOverlays(ctx: CanvasRenderingContext2D, tx: number, ty: number): void {
    const m = this.g.map;
    const i = ty * MW + tx;
    const x = tx * PX;
    const y = ty * PX;
    if (m.terrain[i] === T_FOREST) {
      // 木
      ctx.fillStyle = '#2f5a28';
      ctx.beginPath();
      ctx.moveTo(x + 2, y + 0.3);
      ctx.lineTo(x + 3.5, y + 3);
      ctx.lineTo(x + 0.5, y + 3);
      ctx.closePath();
      ctx.fill();
      ctx.fillStyle = '#5a3a1a';
      ctx.fillRect(x + 1.7, y + 3, 0.6, 0.8);
    }
    if (m.mine[i]) {
      ctx.fillStyle = '#3d3326';
      ctx.beginPath();
      ctx.moveTo(x + 2, y + 0.4);
      ctx.lineTo(x + 3.7, y + 3.6);
      ctx.lineTo(x + 0.3, y + 3.6);
      ctx.closePath();
      ctx.fill();
      ctx.fillStyle = '#e0c060';
      ctx.fillRect(x + 1.6, y + 2.2, 0.9, 0.9);
    }
  }

  private buildTerrain(): HTMLCanvasElement {
    const c = document.createElement('canvas');
    c.width = MW * PX;
    c.height = MH * PX;
    const ctx = c.getContext('2d')!;
    const img = ctx.createImageData(c.width, c.height);
    for (let ty = 0; ty < MH; ty++) for (let tx = 0; tx < MW; tx++) this.paintTile(img.data, c.width, tx, ty);
    this.terrainImg = img;
    ctx.putImageData(img, 0, 0);
    this.drawRivers(ctx);
    for (let ty = 0; ty < MH; ty++) for (let tx = 0; tx < MW; tx++) this.drawTileOverlays(ctx, tx, ty);
    return c;
  }

  private drawRivers(ctx: CanvasRenderingContext2D, clip?: { x: number; y: number; w: number; h: number }): void {
    ctx.save();
    if (clip) {
      ctx.beginPath();
      ctx.rect(clip.x, clip.y, clip.w, clip.h);
      ctx.clip();
    }
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    for (const r of RIVERS) {
      ctx.strokeStyle = 'rgba(64,124,178,0.95)';
      ctx.lineWidth = r.width * PX * 1.4;
      ctx.beginPath();
      r.pts.forEach(([lon, lat], k) => {
        const x = lonToX(lon) * PX;
        const y = latToY(lat) * PX;
        if (k === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      ctx.stroke();
    }
    ctx.restore();
  }

  /** 伐採・採掘で変化したタイルを描き直す */
  private patchTerrain(): void {
    const ch = this.g.map.changed;
    if (!ch.length || !this.terrainImg) return;
    const ctx = this.terrain.getContext('2d')!;
    const img = this.terrainImg;
    const tiles = [...new Set(ch.splice(0))];
    for (const i of tiles) {
      const tx = i % MW;
      const ty = (i / MW) | 0;
      this.paintTile(img.data, img.width, tx, ty);
      ctx.putImageData(img, 0, 0, tx * PX, ty * PX, PX, PX);
    }
    for (const i of tiles) {
      const tx = i % MW;
      const ty = (i / MW) | 0;
      const clip = { x: tx * PX, y: ty * PX, w: PX, h: PX };
      this.drawRivers(ctx, clip);
      this.drawTileOverlays(ctx, tx, ty);
    }
  }

  private updateFog(): void {
    const g = this.g;
    if (g.fogMode === 'none' || this.fogVersion === g.visVersion) return;
    this.fogVersion = g.visVersion;
    const ctx = this.fog.getContext('2d')!;
    const img = ctx.createImageData(MW, MH);
    const d = img.data;
    const full = g.fogMode === 'full';
    for (let i = 0; i < MW * MH; i++) {
      const o = i * 4;
      d[o] = 12;
      d[o + 1] = 9;
      d[o + 2] = 6;
      d[o + 3] = g.visible[i] ? 0 : full && !g.explored[i] ? 255 : 105;
    }
    ctx.putImageData(img, 0, 0);
  }

  private updateTerritory(): void {
    if (this.terrVersion === this.g.territoryVersion) return;
    this.terrVersion = this.g.territoryVersion;
    const ctx = this.territory.getContext('2d')!;
    const img = ctx.createImageData(MW, MH);
    const d = img.data;
    const own = this.g.map.owner;
    for (let y = 0; y < MH; y++) {
      for (let x = 0; x < MW; x++) {
        const i = y * MW + x;
        const o = own[i];
        if (o < 0) continue;
        const [r, g, b] = this.g.states[o].rgb;
        let border = false;
        if (x > 0 && own[i - 1] !== o) border = true;
        else if (x < MW - 1 && own[i + 1] !== o) border = true;
        else if (y > 0 && own[i - MW] !== o) border = true;
        else if (y < MH - 1 && own[i + MW] !== o) border = true;
        d[i * 4] = r;
        d[i * 4 + 1] = g;
        d[i * 4 + 2] = b;
        d[i * 4 + 3] = border ? 210 : 72;
      }
    }
    ctx.putImageData(img, 0, 0);
  }

  draw(ds: DrawState, realDt: number): void {
    const ctx = this.ctx;
    const g = this.g;
    const z = this.cam.zoom;
    this.updateTerritory();
    this.patchTerrain();
    this.updateFog();
    ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    ctx.fillStyle = '#1b1410';
    ctx.fillRect(0, 0, this.viewW, this.viewH);
    const ox = -this.cam.x * z;
    const oy = -this.cam.y * z;
    ctx.imageSmoothingEnabled = z < PX * 2.5;
    ctx.drawImage(this.terrain, ox, oy, MW * z, MH * z);
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(this.territory, ox, oy, MW * z, MH * z);
    if (g.fogMode !== 'none') {
      ctx.imageSmoothingEnabled = true;
      ctx.drawImage(this.fog, ox, oy, MW * z, MH * z);
    }

    const vx0 = this.cam.x - 2;
    const vy0 = this.cam.y - 2;
    const vx1 = this.cam.x + this.viewW / z + 2;
    const vy1 = this.cam.y + this.viewH / z + 2;
    const inView = (x: number, y: number): boolean => x >= vx0 && x <= vx1 && y >= vy0 && y <= vy1;

    // イベント対象の強調
    const ev = g.event;
    const pulse = 0.5 + 0.5 * Math.sin(performance.now() / 300);

    // 建物
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (const b of g.buildings) {
      if (b.destroyed || !inView(b.x, b.y) || !g.isExplored(b.x, b.y)) continue;
      if (b.owner !== g.playerId && g.fogMode === 'full' && !g.isVisible(b.x, b.y)) continue;
      this.drawBuilding(b, ds);
    }
    // 都市
    for (const c of g.cities) {
      if (c.destroyed || !inView(c.x, c.y) || !g.isExplored(c.x, c.y)) continue;
      this.drawCity(c, ds, ev?.target === c.owner ? pulse : -1);
    }
    // 集結地点
    const sel = g.structById.get(ds.selStruct);
    if (sel && sel.owner === g.playerId && sel.rally) {
      const a = this.worldToScreen(sel.x, sel.y);
      const p = this.worldToScreen(sel.rally.x, sel.rally.y);
      ctx.strokeStyle = 'rgba(255,255,255,0.6)';
      ctx.setLineDash([5, 4]);
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(p.x, p.y);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.strokeStyle = '#111';
      ctx.beginPath();
      ctx.moveTo(p.x, p.y);
      ctx.lineTo(p.x, p.y - 16);
      ctx.stroke();
      ctx.fillStyle = g.states[sel.owner].color;
      ctx.beginPath();
      ctx.moveTo(p.x, p.y - 16);
      ctx.lineTo(p.x + 11, p.y - 12.5);
      ctx.lineTo(p.x, p.y - 9);
      ctx.closePath();
      ctx.fill();
      ctx.stroke();
    }

    // ユニット（小さいものから描く）
    const usize = Math.max(9, Math.min(40, z * 1.7));
    for (const u of g.units) {
      if (u.dead || u.kind !== 'worker' || !inView(u.x, u.y)) continue;
      if (u.owner !== g.playerId && !g.isVisible(u.x, u.y)) continue;
      this.drawUnit(u, ds.selected.has(u.id), usize * 0.85);
    }
    for (const u of g.units) {
      if (u.dead || u.kind === 'worker' || !inView(u.x, u.y)) continue;
      if (u.owner !== g.playerId && !g.isVisible(u.x, u.y)) continue;
      this.drawUnit(u, ds.selected.has(u.id), usize);
    }

    // 国名ラベル（縮小時）
    if (z < 6) {
      ctx.font = `bold ${Math.round(12 + z * 1.5)}px "Noto Serif JP", serif`;
      for (const s of g.states) {
        if (!s.alive) continue;
        const cap = g.cities.find((c) => c.owner === s.id && c.capital && !c.destroyed);
        if (!cap || !inView(cap.x, cap.y) || !g.isExplored(cap.x, cap.y)) continue;
        const p = this.worldToScreen(cap.x, cap.y - 2.2);
        ctx.lineWidth = 3;
        ctx.strokeStyle = 'rgba(0,0,0,0.75)';
        ctx.strokeText(s.name, p.x, p.y);
        ctx.fillStyle = '#fff';
        ctx.fillText(s.name, p.x, p.y);
      }
    }

    // 建設プレビュー
    if (ds.buildPreview) {
      const bp = ds.buildPreview;
      const p = this.worldToScreen(bp.tx + 0.5, bp.ty + 0.5);
      const size = Math.max(16, z * (bp.kind === 'city' ? 2.2 : 1.6));
      const img = iconImage(`b:${bp.kind}:${g.player?.color}`, () => buildingSvg(bp.kind, g.player?.color ?? '#888', 64));
      ctx.globalAlpha = 0.65;
      if (img) ctx.drawImage(img, p.x - size / 2, p.y - size / 2, size, size);
      ctx.globalAlpha = 1;
      ctx.strokeStyle = bp.ok ? '#7CFC00' : '#ff4040';
      ctx.lineWidth = 2;
      ctx.strokeRect(p.x - size / 2 - 2, p.y - size / 2 - 2, size + 4, size + 4);
      if (bp.kind === 'city' || bp.kind === 'tower') {
        ctx.beginPath();
        ctx.arc(p.x, p.y, (bp.kind === 'city' ? cityRadius(1) : 5) * z, 0, Math.PI * 2);
        ctx.setLineDash([4, 4]);
        ctx.stroke();
        ctx.setLineDash([]);
      }
    }
    if (ds.dragRect) {
      const r = ds.dragRect;
      ctx.strokeStyle = '#fff';
      ctx.lineWidth = 1;
      ctx.fillStyle = 'rgba(255,255,255,0.08)';
      ctx.fillRect(Math.min(r.x0, r.x1), Math.min(r.y0, r.y1), Math.abs(r.x1 - r.x0), Math.abs(r.y1 - r.y0));
      ctx.strokeRect(Math.min(r.x0, r.x1), Math.min(r.y0, r.y1), Math.abs(r.x1 - r.x0), Math.abs(r.y1 - r.y0));
    }

    this.mmTimer -= realDt;
    if (this.mmTimer <= 0) {
      this.mmTimer = 0.4;
      this.drawMinimap();
    }
  }

  private drawHpBar(x: number, y: number, w: number, frac: number): void {
    const ctx = this.ctx;
    ctx.fillStyle = 'rgba(40,0,0,0.85)';
    ctx.fillRect(x - w / 2, y, w, 3.5);
    ctx.fillStyle = frac > 0.5 ? '#4caf50' : frac > 0.25 ? '#ffc107' : '#ff5722';
    ctx.fillRect(x - w / 2, y, w * Math.max(0, Math.min(1, frac)), 3.5);
  }

  private drawConstruction(st: Struct, p: { x: number; y: number }, size: number, img: HTMLImageElement | null): void {
    const ctx = this.ctx;
    ctx.globalAlpha = 0.45;
    if (img) ctx.drawImage(img, p.x - size / 2, p.y - size / 2, size, size);
    ctx.globalAlpha = 1;
    ctx.save();
    ctx.beginPath();
    ctx.rect(p.x - size / 2, p.y + size / 2 - size * st.built, size, size * st.built);
    ctx.clip();
    if (img) ctx.drawImage(img, p.x - size / 2, p.y - size / 2, size, size);
    ctx.restore();
    ctx.setLineDash([3, 3]);
    ctx.strokeStyle = '#f5e6c0';
    ctx.lineWidth = 1.5;
    ctx.strokeRect(p.x - size / 2, p.y - size / 2, size, size);
    ctx.setLineDash([]);
  }

  private drawCity(c: City, ds: DrawState, pulse: number): void {
    const ctx = this.ctx;
    const z = this.cam.zoom;
    const s = this.g.states[c.owner];
    const p = this.worldToScreen(c.x, c.y);
    const size = Math.max(14, z * (1.7 + c.level * 0.25));
    if (pulse >= 0) {
      ctx.strokeStyle = `rgba(255,60,40,${0.4 + pulse * 0.5})`;
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.arc(p.x, p.y, size * (0.85 + pulse * 0.2), 0, Math.PI * 2);
      ctx.stroke();
    }
    const img = iconImage(`b:city:${s.color}`, () => buildingSvg('city', s.color, 64));
    if (c.built < 1) this.drawConstruction(c, p, size, img);
    else if (img) ctx.drawImage(img, p.x - size / 2, p.y - size / 2, size, size);
    if (c.capital && c.built >= 1) {
      ctx.fillStyle = '#ffd700';
      ctx.strokeStyle = '#000';
      ctx.lineWidth = 2;
      ctx.font = `${Math.max(10, size * 0.42)}px sans-serif`;
      ctx.strokeText('★', p.x + size / 2, p.y - size / 2);
      ctx.fillText('★', p.x + size / 2, p.y - size / 2);
    }
    if (ds.selStruct === c.id) {
      ctx.strokeStyle = '#fff';
      ctx.lineWidth = 2;
      ctx.strokeRect(p.x - size / 2 - 3, p.y - size / 2 - 3, size + 6, size + 6);
    }
    const maxHp = structMaxHp(c);
    if (c.built >= 1 && c.hp < maxHp - 1) this.drawHpBar(p.x, p.y + size / 2 + 3, size * 1.2, c.hp / maxHp);
    if (z >= 4.5) {
      ctx.font = `${Math.round(Math.min(15, 9 + z * 0.3))}px "Noto Serif JP", serif`;
      const label = z >= 9 ? `${s.name}・${c.name}${c.built >= 1 ? ' Lv' + c.level : ''}` : c.name;
      ctx.lineWidth = 3;
      ctx.strokeStyle = 'rgba(0,0,0,0.8)';
      const ly = p.y - size / 2 - 9;
      ctx.strokeText(label, p.x, ly);
      ctx.fillStyle = '#fff';
      ctx.fillText(label, p.x, ly);
    }
  }

  private drawBuilding(b: Building, ds: DrawState): void {
    const ctx = this.ctx;
    const z = this.cam.zoom;
    const s = this.g.states[b.owner];
    const p = this.worldToScreen(b.x, b.y);
    const size = Math.max(11, z * 1.5);
    const img = iconImage(`b:${b.kind}:${s.color}`, () => buildingSvg(b.kind, s.color, 64));
    if (b.built < 1) this.drawConstruction(b, p, size, img);
    else if (img) ctx.drawImage(img, p.x - size / 2, p.y - size / 2, size, size);
    if (ds.selStruct === b.id) {
      ctx.strokeStyle = '#fff';
      ctx.lineWidth = 2;
      ctx.strokeRect(p.x - size / 2 - 3, p.y - size / 2 - 3, size + 6, size + 6);
    }
    const maxHp = BUILDING[b.kind].hp;
    if (b.built >= 1 && b.hp < maxHp - 1) this.drawHpBar(p.x, p.y + size / 2 + 2, size * 1.2, b.hp / maxHp);
    if (b.queue.length && z >= 6) {
      const q = b.queue[0];
      ctx.fillStyle = 'rgba(0,0,0,0.6)';
      ctx.fillRect(p.x - size / 2, p.y - size / 2 - 5, size, 3);
      ctx.fillStyle = '#d4a246';
      ctx.fillRect(p.x - size / 2, p.y - size / 2 - 5, (size * q.progress) / q.time, 3);
    }
  }

  private drawUnit(u: Unit, sel: boolean, size: number): void {
    const ctx = this.ctx;
    const z = this.cam.zoom;
    const s = this.g.states[u.owner];
    const p = this.worldToScreen(u.x, u.y);
    const warring = this.g.era === 'warring';
    const r = size / 2;
    if (sel) {
      ctx.strokeStyle = '#7CFC00';
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.ellipse(p.x, p.y + r * 0.75, r * 1.05, r * 0.45, 0, 0, Math.PI * 2);
      ctx.stroke();
    }
    const img = size >= 10 ? iconImage(`u:${u.kind}:${s.color}:${warring && u.kind === 'archer' ? 1 : 0}`, () => unitSvg(u.kind, s.color, warring, 64)) : null;
    if (img) {
      ctx.drawImage(img, p.x - r, p.y - r, size, size);
    } else {
      ctx.fillStyle = s.color;
      ctx.strokeStyle = u.kind === 'worker' ? '#f5f5f5' : '#111';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.arc(p.x, p.y, Math.max(2.5, r * 0.6), 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }
    if (u.kind !== 'worker' && z >= 5) {
      const frac = u.ability / u.maxAbility;
      this.drawHpBar(p.x, p.y - r - 5, size * 0.9, frac);
    }
    if (u.kind === 'worker' && u.carry > 0.5 && z >= 7) {
      ctx.fillStyle = u.carryType === 'food' ? '#f0d060' : u.carryType === 'wood' ? '#8d5a2b' : '#c0c0c0';
      ctx.strokeStyle = '#000';
      ctx.lineWidth = 1;
      ctx.fillRect(p.x + r * 0.45, p.y - r * 1.05, r * 0.6, r * 0.6);
      ctx.strokeRect(p.x + r * 0.45, p.y - r * 1.05, r * 0.6, r * 0.6);
    }
  }

  drawMinimap(): void {
    const m = this.mm;
    m.imageSmoothingEnabled = true;
    m.drawImage(this.terrain, 0, 0, MW, MH);
    m.drawImage(this.territory, 0, 0);
    if (this.g.fogMode !== 'none') m.drawImage(this.fog, 0, 0);
    for (const c of this.g.cities) {
      if (c.destroyed || !this.g.isExplored(c.x, c.y)) continue;
      m.fillStyle = this.g.states[c.owner].color;
      m.fillRect(c.x - 1.5, c.y - 1.5, 3, 3);
      m.strokeStyle = '#000';
      m.lineWidth = 0.5;
      m.strokeRect(c.x - 1.5, c.y - 1.5, 3, 3);
    }
    const pid = this.g.playerId;
    m.fillStyle = '#fff';
    for (const u of this.g.units) if (u.owner === pid && !u.dead) m.fillRect(u.x - 0.5, u.y - 0.5, 1.2, 1.2);
    m.fillStyle = '#ff5040';
    for (const u of this.g.units) {
      if (u.dead || u.owner === pid || u.kind === 'worker' || !this.g.atWar(pid, u.owner) || !this.g.isVisible(u.x, u.y)) continue;
      m.fillRect(u.x - 0.6, u.y - 0.6, 1.4, 1.4);
    }
    m.strokeStyle = '#fff';
    m.lineWidth = 1;
    m.strokeRect(this.cam.x, this.cam.y, this.viewW / this.cam.zoom, this.viewH / this.cam.zoom);
  }
}

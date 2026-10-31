import { UNIT } from './config';
import { RIVERS } from './data/geo';
import type { City, Game, Unit } from './game';
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
  selCity: number;
  dragRect: { x0: number; y0: number; x1: number; y1: number } | null;
  buildPreview: { tx: number; ty: number; ok: boolean } | null;
  mode: 'normal' | 'attackMove' | 'build';
  hover: { x: number; y: number } | null;
}

export class Renderer {
  terrain: HTMLCanvasElement;
  territory: HTMLCanvasElement;
  private terrVersion = -1;
  cam: Camera = { x: 80, y: 60, zoom: 7 };
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

  private buildTerrain(): HTMLCanvasElement {
    const c = document.createElement('canvas');
    c.width = MW * PX;
    c.height = MH * PX;
    const ctx = c.getContext('2d')!;
    const img = ctx.createImageData(c.width, c.height);
    const d = img.data;
    const m = this.g.map;
    for (let ty = 0; ty < MH; ty++) {
      for (let tx = 0; tx < MW; tx++) {
        const i = ty * MW + tx;
        const t = m.terrain[i];
        let [r, gg, b] = TCOL[t];
        if (t === T_PLAIN && m.fertile[i]) {
          r -= 18;
          gg += 4;
          b -= 10;
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
            const o = (Y * c.width + X) * 4;
            d[o] = r + n + k;
            d[o + 1] = gg + n + k;
            d[o + 2] = b + n + k;
            d[o + 3] = 255;
          }
        }
      }
    }
    ctx.putImageData(img, 0, 0);
    // 河川
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
    // 鉱山
    for (let ty = 0; ty < MH; ty++) {
      for (let tx = 0; tx < MW; tx++) {
        if (!m.mine[ty * MW + tx]) continue;
        const x = tx * PX;
        const y = ty * PX;
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
    return c;
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
    ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    ctx.fillStyle = '#1b1410';
    ctx.fillRect(0, 0, this.viewW, this.viewH);
    const ox = -this.cam.x * z;
    const oy = -this.cam.y * z;
    ctx.imageSmoothingEnabled = z < PX * 2.5;
    ctx.drawImage(this.terrain, ox, oy, MW * z, MH * z);
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(this.territory, ox, oy, MW * z, MH * z);

    const vx0 = this.cam.x - 2;
    const vy0 = this.cam.y - 2;
    const vx1 = this.cam.x + this.viewW / z + 2;
    const vy1 = this.cam.y + this.viewH / z + 2;
    const inView = (x: number, y: number): boolean => x >= vx0 && x <= vx1 && y >= vy0 && y <= vy1;

    // イベント対象の強調
    const ev = g.event;
    const pulse = 0.5 + 0.5 * Math.sin(performance.now() / 300);

    // 都市
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (const c of g.cities) {
      if (c.destroyed || !inView(c.x, c.y)) continue;
      this.drawCity(c, ds, ev?.target === c.owner ? pulse : -1);
    }

    // ユニット
    for (const u of g.units) {
      if (u.dead || !inView(u.x, u.y)) continue;
      this.drawUnit(u, ds.selected.has(u.id));
    }

    // 国名ラベル（縮小時）
    if (z < 6) {
      ctx.font = `bold ${Math.round(12 + z * 1.5)}px "Noto Serif JP", serif`;
      for (const s of g.states) {
        if (!s.alive) continue;
        const cap = g.cities.find((c) => c.owner === s.id && c.capital && !c.destroyed);
        if (!cap || !inView(cap.x, cap.y)) continue;
        const p = this.worldToScreen(cap.x, cap.y - 2.2);
        ctx.lineWidth = 3;
        ctx.strokeStyle = 'rgba(0,0,0,0.75)';
        ctx.strokeText(s.name, p.x, p.y);
        ctx.fillStyle = '#fff';
        ctx.fillText(s.name, p.x, p.y);
      }
    }

    // 築城プレビュー
    if (ds.buildPreview) {
      const p = this.worldToScreen(ds.buildPreview.tx, ds.buildPreview.ty);
      ctx.strokeStyle = ds.buildPreview.ok ? '#7CFC00' : '#ff4040';
      ctx.lineWidth = 2;
      ctx.strokeRect(p.x - z * 0.5, p.y - z * 0.5, z * 2, z * 2);
      ctx.beginPath();
      ctx.arc(p.x + z * 0.5, p.y + z * 0.5, 7 * z, 0, Math.PI * 2);
      ctx.setLineDash([4, 4]);
      ctx.stroke();
      ctx.setLineDash([]);
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

  private drawCity(c: City, ds: DrawState, pulse: number): void {
    const ctx = this.ctx;
    const z = this.cam.zoom;
    const s = this.g.states[c.owner];
    const p = this.worldToScreen(c.x, c.y);
    const size = Math.max(7, z * (1.1 + c.level * 0.2));
    if (pulse >= 0) {
      ctx.strokeStyle = `rgba(255,60,40,${0.4 + pulse * 0.5})`;
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.arc(p.x, p.y, size * (1.1 + pulse * 0.25), 0, Math.PI * 2);
      ctx.stroke();
    }
    if (c.built < 1) {
      ctx.setLineDash([3, 3]);
      ctx.strokeStyle = s.color;
      ctx.lineWidth = 2;
      ctx.strokeRect(p.x - size / 2, p.y - size / 2, size, size);
      ctx.setLineDash([]);
      ctx.fillStyle = s.color;
      ctx.fillRect(p.x - size / 2, p.y + size / 2 - size * c.built, size, size * c.built);
    } else {
      ctx.fillStyle = s.color;
      ctx.strokeStyle = '#111';
      ctx.lineWidth = 1.5;
      ctx.fillRect(p.x - size / 2, p.y - size / 2, size, size);
      ctx.strokeRect(p.x - size / 2, p.y - size / 2, size, size);
      // 城壁の凸凹
      ctx.fillStyle = '#111';
      const b = Math.max(1.5, size / 7);
      for (let k = 0; k < 3; k++) ctx.fillRect(p.x - size / 2 + (k * (size - b)) / 2, p.y - size / 2 - b, b, b);
      if (c.capital) {
        ctx.fillStyle = '#ffd700';
        ctx.font = `${Math.max(9, size * 0.7)}px sans-serif`;
        ctx.fillText('★', p.x, p.y + 0.5);
      }
    }
    if (ds.selCity === c.id) {
      ctx.strokeStyle = '#fff';
      ctx.lineWidth = 2;
      ctx.strokeRect(p.x - size / 2 - 3, p.y - size / 2 - 3, size + 6, size + 6);
    }
    // HP
    const maxHp = 600 + 400 * c.level;
    if (c.built >= 1 && c.hp < maxHp - 1) {
      const w = size * 1.4;
      ctx.fillStyle = '#300';
      ctx.fillRect(p.x - w / 2, p.y + size / 2 + 3, w, 3);
      ctx.fillStyle = c.hp / maxHp > 0.4 ? '#4caf50' : '#ff5722';
      ctx.fillRect(p.x - w / 2, p.y + size / 2 + 3, (w * Math.max(0, c.hp)) / maxHp, 3);
    }
    if (z >= 5) {
      ctx.font = `${Math.round(Math.min(15, 9 + z * 0.3))}px "Noto Serif JP", serif`;
      const label = z >= 9 ? `${s.name}・${c.name}${c.built >= 1 ? ' Lv' + c.level : ''}` : c.name;
      ctx.lineWidth = 3;
      ctx.strokeStyle = 'rgba(0,0,0,0.8)';
      const ly = p.y - size / 2 - Math.max(8, z * 0.6) - 2;
      ctx.strokeText(label, p.x, ly);
      ctx.fillStyle = '#fff';
      ctx.fillText(label, p.x, ly);
    }
  }

  private drawUnit(u: Unit, sel: boolean): void {
    const ctx = this.ctx;
    const z = this.cam.zoom;
    const s = this.g.states[u.owner];
    const p = this.worldToScreen(u.x, u.y);
    const r = Math.max(2.5, z * (u.kind === 'worker' ? 0.32 : 0.42));
    if (sel) {
      ctx.strokeStyle = '#7CFC00';
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(p.x, p.y, r + 3, 0, Math.PI * 2);
      ctx.stroke();
    }
    ctx.fillStyle = s.color;
    ctx.strokeStyle = u.kind === 'worker' ? '#f5f5f5' : '#111';
    ctx.lineWidth = u.kind === 'worker' ? 1 : 1.5;
    ctx.beginPath();
    if (u.kind === 'worker') ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
    else if (u.kind === 'chariot' || u.kind === 'cavalry') {
      ctx.moveTo(p.x, p.y - r * 1.2);
      ctx.lineTo(p.x + r * 1.1, p.y + r * 0.9);
      ctx.lineTo(p.x - r * 1.1, p.y + r * 0.9);
      ctx.closePath();
    } else ctx.rect(p.x - r, p.y - r, r * 2, r * 2);
    ctx.fill();
    ctx.stroke();
    if (z >= 11) {
      ctx.fillStyle = '#fff';
      ctx.font = `bold ${Math.round(r * 1.1)}px sans-serif`;
      ctx.fillText(u.kind === 'archer' && this.g.era === 'warring' ? '弩' : UNIT[u.kind].short, p.x, p.y + 0.5);
    }
    if (u.kind !== 'worker' && z >= 6) {
      const w = r * 2.4;
      const frac = u.ability / u.maxAbility;
      ctx.fillStyle = '#300';
      ctx.fillRect(p.x - w / 2, p.y - r - 5, w, 2.5);
      ctx.fillStyle = frac > 0.5 ? '#8bc34a' : frac > 0.25 ? '#ffc107' : '#f44336';
      ctx.fillRect(p.x - w / 2, p.y - r - 5, w * Math.max(0, Math.min(1, frac)), 2.5);
    }
    if (u.kind === 'worker' && u.carry > 0.5 && z >= 8) {
      ctx.fillStyle = u.carryType === 'food' ? '#f0d060' : u.carryType === 'wood' ? '#8d5a2b' : '#c0c0c0';
      ctx.fillRect(p.x + r * 0.4, p.y - r * 1.3, r * 0.9, r * 0.9);
    }
  }

  drawMinimap(): void {
    const m = this.mm;
    m.imageSmoothingEnabled = true;
    m.drawImage(this.terrain, 0, 0, MW, MH);
    m.drawImage(this.territory, 0, 0);
    for (const c of this.g.cities) {
      if (c.destroyed) continue;
      m.fillStyle = this.g.states[c.owner].color;
      m.fillRect(c.x - 1.5, c.y - 1.5, 3, 3);
      m.strokeStyle = '#000';
      m.lineWidth = 0.5;
      m.strokeRect(c.x - 1.5, c.y - 1.5, 3, 3);
    }
    const pid = this.g.playerId;
    m.fillStyle = '#fff';
    for (const u of this.g.units) if (u.owner === pid && !u.dead) m.fillRect(u.x - 0.5, u.y - 0.5, 1.2, 1.2);
    m.strokeStyle = '#fff';
    m.lineWidth = 1;
    m.strokeRect(this.cam.x, this.cam.y, this.viewW / this.cam.zoom, this.viewH / this.cam.zoom);
  }
}

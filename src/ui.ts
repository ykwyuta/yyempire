import {
  BUILDING, BUILDING_KINDS, MAX_QUEUE, NEW_CITY_COST, RES_NAMES, UNIT, abilityCap, canAfford, cityLevelCap, cityUpgradeCost,
  resStr, yearLabel, type BuildingKind, type Res, type ResKey, type UnitKind, MILITARY_KINDS,
} from './config';
import { structMaxHp, structName, type Game, type Msg, type Struct, type Unit } from './game';
import { plainBuildingSvg, plainUnitSvg, resSvg, unitSvg, buildingSvg, upgradeSvg } from './icons';
import { MH, MW, TERRAIN_NAMES } from './map';
import type { DrawState, Renderer } from './render';

const $ = <T extends HTMLElement>(id: string): T => document.getElementById(id) as T;

function esc(s: string): string {
  return s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]!);
}

function costHtml(r: Res): string {
  const parts: string[] = [];
  if (r.food) parts.push(`${resSvg('food', 11)}${Math.ceil(r.food)}`);
  if (r.wood) parts.push(`${resSvg('wood', 11)}${Math.ceil(r.wood)}`);
  if (r.metal) parts.push(`${resSvg('metal', 11)}${Math.ceil(r.metal)}`);
  return parts.join(' ') || '—';
}

export class UI {
  ds: DrawState = {
    selected: new Set(), selStruct: -1, dragRect: null, buildPreview: null, mode: 'normal', buildKind: 'city', hover: null,
  };
  speed = 1;
  prevSpeed = 1;
  modalOpen = false;
  cmdMode = false; // タッチ用
  private buildMenu = false;
  private selOther = -1;
  private panelTimer = 0;
  private topTimer = 0;
  private keys = new Set<string>();
  private dragStart: { x: number; y: number; button: number; moved: boolean } | null = null;
  private pointers = new Map<number, { x: number; y: number }>();
  private pinchDist = 0;
  private lastPanelHtml = '';
  private bannerKey = '';
  private panelHeld = false;
  private groups = new Map<number, number[]>();
  private lastGroupKey = { n: -1, t: 0 };
  private idleCycle = 0;

  constructor(private g: Game, private r: Renderer, private canvas: HTMLCanvasElement) {
    this.bindInput();
    this.bindButtons();
    g.onMessage = (m) => this.addLog(m);
    g.onEvent = (t, b) => this.showModal(t, b);
    this.setSpeed(1);
  }

  // ---------- 入力 ----------
  private bindInput(): void {
    const cv = this.canvas;
    cv.addEventListener('contextmenu', (e) => e.preventDefault());
    cv.addEventListener('wheel', (e) => {
      e.preventDefault();
      this.r.zoomAt(e.offsetX, e.offsetY, e.deltaY < 0 ? 1.15 : 1 / 1.15);
    }, { passive: false });
    cv.addEventListener('pointerdown', (e) => this.onDown(e));
    cv.addEventListener('pointermove', (e) => this.onMove(e));
    cv.addEventListener('pointerup', (e) => this.onUp(e));
    cv.addEventListener('pointercancel', (e) => {
      this.pointers.delete(e.pointerId);
      this.dragStart = null;
      this.ds.dragRect = null;
    });
    window.addEventListener('keydown', (e) => this.onKey(e));
    window.addEventListener('keyup', (e) => this.keys.delete(e.key));
    const mm = $<HTMLCanvasElement>('minimap');
    const mmMove = (e: PointerEvent): void => {
      const rect = mm.getBoundingClientRect();
      this.r.centerOn(((e.clientX - rect.left) / rect.width) * MW, ((e.clientY - rect.top) / rect.height) * MH);
    };
    mm.addEventListener('pointerdown', (e) => {
      if (e.button === 2) {
        const rect = mm.getBoundingClientRect();
        this.command(((e.clientX - rect.left) / rect.width) * MW, ((e.clientY - rect.top) / rect.height) * MH, false);
        return;
      }
      mmMove(e);
      mm.setPointerCapture(e.pointerId);
    });
    mm.addEventListener('contextmenu', (e) => e.preventDefault());
    mm.addEventListener('pointermove', (e) => {
      if (e.buttons === 1) mmMove(e);
    });
  }

  private onDown(e: PointerEvent): void {
    if (this.modalOpen) return;
    this.canvas.setPointerCapture(e.pointerId);
    this.pointers.set(e.pointerId, { x: e.offsetX, y: e.offsetY });
    if (this.pointers.size === 2) {
      const [a, b] = [...this.pointers.values()];
      this.pinchDist = Math.hypot(a.x - b.x, a.y - b.y);
      this.dragStart = null;
      this.ds.dragRect = null;
      return;
    }
    this.dragStart = { x: e.offsetX, y: e.offsetY, button: e.pointerType === 'touch' ? 9 : e.button, moved: false };
  }

  private onMove(e: PointerEvent): void {
    const prev = this.pointers.get(e.pointerId);
    if (prev) this.pointers.set(e.pointerId, { x: e.offsetX, y: e.offsetY });
    if (this.pointers.size === 2) {
      const [a, b] = [...this.pointers.values()];
      const d = Math.hypot(a.x - b.x, a.y - b.y);
      if (this.pinchDist > 0) this.r.zoomAt((a.x + b.x) / 2, (a.y + b.y) / 2, d / this.pinchDist);
      this.pinchDist = d;
      return;
    }
    const w = this.r.screenToWorld(e.offsetX, e.offsetY);
    this.ds.hover = w;
    if (this.ds.mode === 'build') this.updatePreview(w.x, w.y);
    const d = this.dragStart;
    if (!d) return;
    if (Math.hypot(e.offsetX - d.x, e.offsetY - d.y) > 5) d.moved = true;
    if (!d.moved) return;
    if (d.button === 1 || d.button === 9 || (d.button === 0 && this.keys.has(' '))) {
      if (prev) {
        this.r.cam.x -= (e.offsetX - prev.x) / this.r.cam.zoom;
        this.r.cam.y -= (e.offsetY - prev.y) / this.r.cam.zoom;
        this.r.clamp();
      }
    } else if (d.button === 0) {
      this.ds.dragRect = { x0: d.x, y0: d.y, x1: e.offsetX, y1: e.offsetY };
    }
  }

  private updatePreview(x: number, y: number): void {
    const tx = Math.floor(x);
    const ty = Math.floor(y);
    const pid = this.g.playerId;
    const k = this.ds.buildKind;
    const ok = pid >= 0 && !(k === 'city' ? this.g.canFoundCity(pid, tx, ty) : this.g.canPlaceBuilding(pid, k, tx, ty));
    this.ds.buildPreview = { tx, ty, ok, kind: k };
  }

  private onUp(e: PointerEvent): void {
    this.pointers.delete(e.pointerId);
    if (this.pointers.size < 2) this.pinchDist = 0;
    const d = this.dragStart;
    this.dragStart = null;
    if (!d) return;
    const w = this.r.screenToWorld(e.offsetX, e.offsetY);
    if (d.button === 0 || d.button === 9) {
      if (d.moved && d.button === 0 && this.ds.dragRect) {
        this.boxSelect(this.ds.dragRect, e.shiftKey);
        this.ds.dragRect = null;
        return;
      }
      if (d.moved) return;
      if (this.ds.mode === 'build') {
        this.tryBuild(Math.floor(w.x), Math.floor(w.y), e.shiftKey);
        return;
      }
      if (this.ds.mode === 'attackMove') {
        this.command(w.x, w.y, true);
        this.setMode('normal');
        return;
      }
      if (this.ds.mode === 'rally') {
        const st = this.g.structById.get(this.ds.selStruct);
        if (st) this.g.setRally(st, w.x, w.y);
        this.setMode('normal');
        return;
      }
      if (d.button === 9 && this.cmdMode && (this.ds.selected.size || this.ds.selStruct >= 0)) {
        this.command(w.x, w.y, false);
        return;
      }
      this.clickSelect(w.x, w.y, e.shiftKey);
    } else if (d.button === 2 && !d.moved) {
      if (this.ds.mode !== 'normal') {
        this.setMode('normal');
        return;
      }
      this.command(w.x, w.y, false);
    }
  }

  private onKey(e: KeyboardEvent): void {
    if (this.modalOpen) {
      if (e.key === 'Enter' || e.key === 'Escape') this.closeModal();
      return;
    }
    if ((e.target as HTMLElement)?.tagName === 'INPUT') return;
    this.keys.add(e.key);
    const k = e.key.toLowerCase();
    // 部隊編成
    if (/^[0-9]$/.test(e.key)) {
      const n = Number(e.key);
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        this.groups.set(n, [...this.ds.selected]);
        this.toast(`部隊 ${n} を編成（${this.ds.selected.size}）`);
      } else {
        const ids = (this.groups.get(n) ?? []).filter((id) => this.g.unitById.has(id));
        if (!ids.length) return;
        this.ds.selected = new Set(ids);
        this.ds.selStruct = -1;
        this.selOther = -1;
        const now = performance.now();
        if (this.lastGroupKey.n === n && now - this.lastGroupKey.t < 400) {
          const u = this.g.unitById.get(ids[0])!;
          this.r.centerOn(u.x, u.y);
        }
        this.lastGroupKey = { n, t: now };
        this.refreshPanel(true);
      }
      return;
    }
    if (k === ' ') {
      e.preventDefault();
      this.setSpeed(this.speed === 0 ? this.prevSpeed || 1 : 0);
    } else if (k === 'escape') {
      if (this.ds.mode !== 'normal' || this.buildMenu) {
        this.buildMenu = false;
        this.setMode('normal');
        return;
      }
      this.ds.selected.clear();
      this.ds.selStruct = -1;
      this.selOther = -1;
      this.refreshPanel(true);
    } else if (k === 'a') {
      if (this.selectedUnits().some((u) => u.kind !== 'worker')) this.setMode('attackMove');
    } else if (k === 'b') {
      if (this.selectedUnits().some((u) => u.kind === 'worker')) {
        this.buildMenu = !this.buildMenu;
        this.refreshPanel(true);
      }
    } else if (k === 'q') {
      this.selectWhere((u) => u.kind !== 'worker');
    } else if (k === 'e' || k === '.') {
      this.nextIdleWorker();
    } else if (k === 'h') {
      const cap = this.g.cities.find((c) => c.owner === this.g.playerId && c.capital && !c.destroyed);
      if (cap) this.r.centerOn(cap.x, cap.y);
    } else if (k === '+' || k === '=') this.r.zoomAt(this.r.viewW / 2, this.r.viewH / 2, 1.2);
    else if (k === '-') this.r.zoomAt(this.r.viewW / 2, this.r.viewH / 2, 1 / 1.2);
  }

  tick(realDt: number): void {
    const sp = (600 * realDt) / this.r.cam.zoom;
    if (this.keys.has('ArrowLeft')) this.r.cam.x -= sp;
    if (this.keys.has('ArrowRight')) this.r.cam.x += sp;
    if (this.keys.has('ArrowUp')) this.r.cam.y -= sp;
    if (this.keys.has('ArrowDown')) this.r.cam.y += sp;
    if (this.keys.size) this.r.clamp();
    for (const id of this.ds.selected) if (!this.g.unitById.has(id) || this.g.unitById.get(id)!.owner !== this.g.playerId) this.ds.selected.delete(id);
    const st = this.g.structById.get(this.ds.selStruct);
    if (this.ds.selStruct >= 0 && (!st || st.destroyed)) this.ds.selStruct = -1;
    this.panelTimer -= realDt;
    if (this.panelTimer <= 0) {
      this.panelTimer = 0.25;
      this.refreshPanel(false);
    }
    this.topTimer -= realDt;
    if (this.topTimer <= 0) {
      this.topTimer = 0.25;
      this.refreshTop();
    }
  }

  setMode(m: DrawState['mode']): void {
    this.ds.mode = m;
    if (m !== 'build') this.ds.buildPreview = null;
    this.canvas.classList.toggle('cmd-attack', m === 'attackMove' || m === 'rally');
    this.canvas.classList.toggle('cmd-build', m === 'build');
    this.refreshPanel(true);
  }

  // ---------- 選択 ----------
  selectedUnits(): Unit[] {
    const out: Unit[] = [];
    for (const id of this.ds.selected) {
      const u = this.g.unitById.get(id);
      if (u && !u.dead) out.push(u);
    }
    return out;
  }

  private selectWhere(pred: (u: Unit) => boolean): void {
    this.ds.selected.clear();
    this.ds.selStruct = -1;
    this.selOther = -1;
    this.buildMenu = false;
    for (const u of this.g.units) if (!u.dead && u.owner === this.g.playerId && pred(u)) this.ds.selected.add(u.id);
    this.refreshPanel(true);
  }

  private nextIdleWorker(): void {
    const idle = this.g.idleWorkers(this.g.playerId);
    if (!idle.length) {
      this.toast('待機中の工人はいない');
      return;
    }
    const u = idle[this.idleCycle++ % idle.length];
    this.ds.selected = new Set([u.id]);
    this.ds.selStruct = -1;
    this.selOther = -1;
    this.r.centerOn(u.x, u.y);
    this.refreshPanel(true);
  }

  private pickUnit(x: number, y: number, pred: (u: Unit) => boolean): Unit | null {
    const rad = Math.max(0.6, 11 / this.r.cam.zoom);
    let best: Unit | null = null;
    let bd = rad;
    for (const u of this.g.units) {
      if (u.dead || !pred(u)) continue;
      if (u.owner !== this.g.playerId && !this.g.isVisible(u.x, u.y)) continue;
      const d = Math.hypot(u.x - x, u.y - y);
      if (d < bd) {
        bd = d;
        best = u;
      }
    }
    return best;
  }

  private pickStruct(x: number, y: number): Struct | null {
    let best: Struct | null = null;
    let bd = Infinity;
    for (const c of this.g.allStructs()) {
      if (c.destroyed || !this.g.isExplored(c.x, c.y)) continue;
      const rad = Math.max(c.isCity ? 1.2 : 0.8, (c.isCity ? 12 : 9) / this.r.cam.zoom);
      const d = Math.hypot(c.x - x, c.y - y);
      if (d < rad && d < bd) {
        bd = d;
        best = c;
      }
    }
    return best;
  }

  private clickSelect(x: number, y: number, add: boolean): void {
    const pid = this.g.playerId;
    const st = this.pickStruct(x, y);
    const onStruct = st && Math.hypot(st.x - x, st.y - y) < Math.max(0.6, 7 / this.r.cam.zoom);
    const own = onStruct ? null : this.pickUnit(x, y, (u) => u.owner === pid);
    this.buildMenu = false;
    if (!add) {
      this.ds.selected.clear();
      this.ds.selStruct = -1;
      this.selOther = -1;
    }
    if (own) {
      if (add && this.ds.selected.has(own.id)) this.ds.selected.delete(own.id);
      else this.ds.selected.add(own.id);
    } else if (st) {
      this.ds.selected.clear();
      this.ds.selStruct = st.id;
    } else {
      const other = this.pickUnit(x, y, () => true);
      if (other) this.selOther = other.id;
    }
    this.refreshPanel(true);
  }

  private boxSelect(r: { x0: number; y0: number; x1: number; y1: number }, add: boolean): void {
    const a = this.r.screenToWorld(Math.min(r.x0, r.x1), Math.min(r.y0, r.y1));
    const b = this.r.screenToWorld(Math.max(r.x0, r.x1), Math.max(r.y0, r.y1));
    if (!add) this.ds.selected.clear();
    this.ds.selStruct = -1;
    this.selOther = -1;
    this.buildMenu = false;
    const pid = this.g.playerId;
    const inBox = this.g.units.filter((u) => !u.dead && u.owner === pid && u.x >= a.x && u.x <= b.x && u.y >= a.y && u.y <= b.y);
    const hasMil = inBox.some((u) => u.kind !== 'worker');
    for (const u of inBox) if (!hasMil || u.kind !== 'worker' || add) this.ds.selected.add(u.id);
    this.refreshPanel(true);
  }

  // ---------- 命令 ----------
  private command(x: number, y: number, attackMove: boolean): void {
    const g = this.g;
    const pid = g.playerId;
    // 建物選択中の右クリック = 集結地点
    const selSt = g.structById.get(this.ds.selStruct);
    if (selSt && selSt.owner === pid && !this.ds.selected.size) {
      if (g.producible(selSt).length) {
        g.setRally(selSt, x, y);
        this.refreshPanel(true);
      }
      return;
    }
    const units = this.selectedUnits();
    if (!units.length) return;
    const soldiers = units.filter((u) => u.kind !== 'worker');
    const workers = units.filter((u) => u.kind === 'worker');
    const enemy = this.pickUnit(x, y, (u) => u.owner !== pid);
    if (enemy && soldiers.length) {
      if (g.atWar(pid, enemy.owner)) {
        g.attackTarget(soldiers, { unit: enemy });
        if (workers.length) g.moveUnits(workers, x, y, false);
        return;
      }
      this.toast(`${g.states[enemy.owner].name} とは交戦していない（諸国一覧から宣戦できる）`);
    }
    const st = this.pickStruct(x, y);
    if (st) {
      if (st.owner !== pid && soldiers.length) {
        if (g.atWar(pid, st.owner)) {
          g.attackTarget(soldiers, { struct: st });
          if (workers.length) g.moveUnits(workers, x, y, false);
          return;
        }
        this.toast(`${g.states[st.owner].name} とは交戦していない`);
      }
      if (st.owner === pid && workers.length && (st.built < 1 || st.hp < structMaxHp(st))) {
        for (const w of workers) g.orderBuild(w, st);
        if (soldiers.length) g.moveUnits(soldiers, x, y, attackMove);
        return;
      }
    }
    const tx = Math.floor(x);
    const ty = Math.floor(y);
    if (workers.length && g.map.inBounds(tx, ty)) {
      const res = g.map.gatherType(tx, ty);
      const own = g.map.owner[ty * MW + tx];
      if (res && (own === pid || own === -1)) {
        for (const w of workers) g.gather(w, tx, ty);
      } else {
        g.moveUnits(workers, x, y, false);
        if (res && own !== pid) this.toast('他国の領土では採集できない');
      }
    }
    if (soldiers.length) g.moveUnits(soldiers, x, y, attackMove);
  }

  private tryBuild(tx: number, ty: number, keep: boolean): void {
    const g = this.g;
    const workers = this.selectedUnits().filter((u) => u.kind === 'worker');
    if (!workers.length) {
      this.setMode('normal');
      return;
    }
    const k = this.ds.buildKind;
    const err = k === 'city' ? g.canFoundCity(g.playerId, tx, ty) : g.canPlaceBuilding(g.playerId, k, tx, ty);
    if (err) {
      this.toast(err);
      return;
    }
    const st = k === 'city' ? g.foundCity(g.playerId, tx, ty, workers) : g.placeBuilding(g.playerId, k, tx, ty, workers);
    if (st) this.toast(`${structName(st)} の建設を開始した`);
    if (!keep) {
      this.buildMenu = false;
      this.setMode('normal');
    }
  }

  // ---------- パネル ----------
  private bindButtons(): void {
    document.querySelectorAll<HTMLButtonElement>('#controls [data-speed]').forEach((b) =>
      b.addEventListener('click', () => this.setSpeed(Number(b.dataset.speed))),
    );
    $('btn-diplo').addEventListener('click', () => this.openDiplo());
    $('diplo-close').addEventListener('click', () => this.closeOverlay('diplo'));
    $('btn-help').addEventListener('click', () => this.openOverlay('help'));
    $('help-close').addEventListener('click', () => this.closeOverlay('help'));
    $('modal-ok').addEventListener('click', () => this.closeModal());
    $('btn-idle').addEventListener('click', () => this.nextIdleWorker());
    const panel = $('panel');
    panel.addEventListener('pointerdown', () => (this.panelHeld = true));
    window.addEventListener('pointerup', () => setTimeout(() => (this.panelHeld = false), 0));
    panel.addEventListener('click', (e) => {
      const b = (e.target as HTMLElement).closest('[data-act]') as HTMLElement | null;
      if (!b || (b as HTMLButtonElement).disabled) return;
      this.panelAction(b.dataset.act!, b.dataset.arg ?? '');
    });
    $('diplo-body').addEventListener('click', (e) => {
      const b = (e.target as HTMLElement).closest('[data-act]') as HTMLElement | null;
      if (!b) return;
      e.preventDefault();
      const sid = Number(b.dataset.arg);
      if (b.dataset.act === 'war') this.g.declareWar(this.g.playerId, sid);
      else if (b.dataset.act === 'peace') this.g.requestPeace(this.g.playerId, sid);
      else if (b.dataset.act === 'accept') this.g.makePeace(this.g.playerId, sid);
      else if (b.dataset.act === 'goto') {
        const cap = this.g.cities.find((c) => c.owner === sid && !c.destroyed && c.capital) ?? this.g.cities.find((c) => c.owner === sid && !c.destroyed);
        if (cap) {
          this.r.centerOn(cap.x, cap.y);
          this.closeOverlay('diplo');
        }
        return;
      }
      this.openDiplo();
    });
  }

  setSpeed(s: number): void {
    if (s > 0) this.prevSpeed = s;
    this.speed = s;
    document.querySelectorAll<HTMLButtonElement>('#controls [data-speed]').forEach((b) => b.classList.toggle('on', Number(b.dataset.speed) === s));
  }

  private openOverlay(id: string): void {
    $(id).classList.remove('hidden');
    this.modalOpen = true;
  }
  private closeOverlay(id: string): void {
    $(id).classList.add('hidden');
    this.modalOpen = false;
  }
  showModal(title: string, body: string): void {
    $('modal-title').textContent = title;
    $('modal-body').textContent = body;
    this.openOverlay('modal');
  }
  private closeModal(): void {
    this.closeOverlay('modal');
    this.closeOverlay('help');
  }
  toast(text: string): void {
    this.addLog({ text, year: this.g.year, kind: 'info', player: true });
  }

  private addLog(m: Msg): void {
    if (!m.player && m.kind !== 'event') return;
    const log = $('log');
    const d = document.createElement('div');
    d.className = m.kind;
    d.textContent = `${yearLabel(m.year)} ${m.text}`;
    log.appendChild(d);
    while (log.children.length > 9) log.firstChild!.remove();
    setTimeout(() => d.remove(), 25000);
  }

  private refreshTop(): void {
    const g = this.g;
    const p = g.player;
    $('date').innerHTML = `${yearLabel(g.year)}<span class="era">${g.era === 'spring' ? '春秋時代' : '戦国時代'}</span>`;
    if (p) {
      const up = g.upkeepPerSec(p.id);
      const r = p.res;
      $('resources').innerHTML =
        `<span><span class="chip" style="background:${p.color}"></span> ${esc(p.name)}</span>` +
        `<span class="${p.starving ? 'neg' : ''}" title="糧（兵の維持費 ${(up * 6).toFixed(1)}/年）">${resSvg('food')} ${Math.floor(r.food)} <small>-${(up * 6).toFixed(1)}/年</small></span>` +
        `<span title="木">${resSvg('wood')} ${Math.floor(r.wood)}</span><span title="金（青銅・鉄）">${resSvg('metal')} ${Math.floor(r.metal)}</span>` +
        `<span title="工人 / 上限">${unitSvg('worker', p.color, false, 16)} ${g.workerCount(p.id)}/${g.workerCap(p.id)}</span>` +
        `<span title="兵 / 上限">${unitSvg('infantry', p.color, false, 16)} ${g.soldierCount(p.id)}/${g.soldierCap(p.id)}</span>` +
        (p.aggro > 20 && g.era === 'spring' ? `<span class="neg" title="周辺国への攻撃で高まる。70 を超えると覇者の呼びかけが起こる">諸侯の警戒 ${Math.round(p.aggro)}</span>` : '');
      const idle = g.idleWorkers(p.id).length;
      const ib = $('btn-idle');
      ib.innerHTML = `${unitSvg('worker', p.color, false, 18)}<b>${idle}</b>`;
      ib.classList.toggle('alert', idle > 0);
    }
    const ev = g.event;
    const banner = $('banner');
    if (ev) {
      const key = `${ev.title}|${g.year}`;
      if (key !== this.bannerKey) {
        this.bannerKey = key;
        const role = ev.target === g.playerId ? '（あなたが標的！）' : ev.members.includes(g.playerId) ? '（あなたも参加国）' : '';
        banner.textContent = `${ev.title} ${role} 残り${ev.endYear - g.year}年`;
      }
      banner.classList.remove('hidden');
    } else banner.classList.add('hidden');
  }

  refreshPanel(force: boolean): void {
    if (this.panelHeld && !force) return;
    const html = this.panelHtml();
    if (!force && html === this.lastPanelHtml) return;
    this.lastPanelHtml = html;
    $('panel').innerHTML = html;
  }

  /** コマンドカードのボタン */
  private card(act: string, icon: string, label: string, enabled: boolean, arg = '', tip = '', hotkey = ''): string {
    return `<button class="card" data-act="${act}" data-arg="${arg}" ${enabled ? '' : 'disabled'} title="${esc(tip)}">${icon}<span class="lbl">${label}</span>${hotkey ? `<span class="hk">${hotkey}</span>` : ''}</button>`;
  }
  private btn(act: string, label: string, enabled: boolean, arg = '', title = ''): string {
    return `<button data-act="${act}" data-arg="${arg}" ${enabled ? '' : 'disabled'} title="${esc(title)}">${label}</button>`;
  }

  private panelHtml(): string {
    const g = this.g;
    const pid = g.playerId;
    if (pid < 0) return '';
    const p = g.states[pid];
    const units = this.selectedUnits();
    if (units.length) return this.unitPanel(units);
    const st = g.structById.get(this.ds.selStruct);
    if (st && !st.destroyed) return this.structPanel(st);
    const other = g.unitById.get(this.selOther);
    if (other && !other.dead && g.isVisible(other.x, other.y)) {
      const s = g.states[other.owner];
      return `<div class="sel-head">${unitSvg(other.kind, s.color, g.era === 'warring', 44)}<div><h3>${esc(s.name)} の ${g.unitName(other.kind)}</h3><div class="info">能力 ${Math.round(other.ability)}/${Math.round(other.maxAbility)}</div></div></div>${this.relationHtml(other.owner)}`;
    }
    const hov = this.ds.hover;
    let html = `<h3><span class="chip" style="background:${p.color}"></span>${esc(p.name)}</h3>`;
    html += `<div class="info">城 ${g.ownCities(pid).length} ・ 建物 ${g.ownBuildings(pid).length} ・ 国力 ${Math.round(g.power(pid))}</div>`;
    const en = g.enemiesOf(pid).map((e) => g.states[e].name);
    html += `<div class="info">交戦中: ${en.length ? esc(en.join('・')) : 'なし'}</div>`;
    if (hov) {
      const tx = Math.floor(hov.x);
      const ty = Math.floor(hov.y);
      if (g.map.inBounds(tx, ty)) {
        const i = ty * MW + tx;
        const own = g.map.owner[i];
        const res = g.map.gatherType(tx, ty);
        const amt = res === 'wood' || res === 'metal' ? ` 残${Math.ceil(g.map.amount[i])}` : '';
        html += `<div class="info">地形: ${TERRAIN_NAMES[g.map.terrain[i]]}${g.map.river[i] ? '（河川）' : ''}${g.map.mine[i] ? '・鉱山' : ''}${g.map.fertile[i] && res === 'food' ? '・肥沃' : ''} / ${own >= 0 ? esc(g.states[own].name) + '領' : '無主地'}${res ? ` / ${RES_NAMES[res]}${amt}` : ''}</div>`;
      }
    }
    html += `<div class="info">ユニットや建物をクリックして選択、右クリックで命令。Ctrl+数字で部隊編成。</div>`;
    html += `<div class="row">${this.btn('selmil', '全軍選択 (Q)', true)}${this.btn('selidle', '待機工人 (E)', true)}${this.btn('home', '都へ (H)', true)}${this.btn('cmdmode', this.cmdMode ? '命令モード: ON' : '命令モード: OFF', true, '', 'タッチ操作用: ON の間はタップで命令')}</div>`;
    return html;
  }

  private queueHtml(st: Struct): string {
    const g = this.g;
    if (!st.queue.length) return '';
    const color = g.states[st.owner].color;
    let html = `<div class="queue">`;
    st.queue.forEach((q, i) => {
      const icon = q.kind === 'upgrade' ? upgradeSvg(30) : unitSvg(q.kind, color, g.era === 'warring', 30);
      const pct = i === 0 ? Math.floor((q.progress / q.time) * 100) : 0;
      html += `<div class="qi" data-act="cancel" data-arg="${i}" title="クリックで取消（費用返還）">${icon}${i === 0 ? `<i style="width:${pct}%"></i>` : ''}</div>`;
    });
    html += `</div>`;
    return html;
  }

  private structPanel(st: Struct): string {
    const g = this.g;
    const pid = g.playerId;
    const p = g.states[pid];
    const owner = g.states[st.owner];
    const icon = st.isCity ? buildingSvg('city', owner.color, 48) : buildingSvg(st.kind, owner.color, 48);
    const title = st.isCity ? `${esc(owner.name)}・${esc(st.name)} ${st.capital ? '★' : ''} <small>Lv${st.level}</small>` : `${esc(owner.name)} の${BUILDING[st.kind].name}`;
    const maxHp = structMaxHp(st);
    let html = `<div class="sel-head">${icon}<div><h3>${title}</h3>`;
    if (st.built < 1) {
      html += `<div class="info">建設中 ${Math.floor(st.built * 100)}%</div><div class="bar"><i style="width:${st.built * 100}%"></i></div></div></div>`;
      if (st.owner === pid) html += `<div class="info">工人を選んでこの建物を右クリックすると建設が進む</div>`;
      return html;
    }
    html += `<div class="info">耐久 ${Math.ceil(st.hp)}/${maxHp}</div><div class="bar"><i style="width:${(st.hp / maxHp) * 100}%"></i></div></div></div>`;
    if (!st.isCity) html += `<div class="info">${BUILDING[st.kind].desc}</div>`;
    if (st.owner !== pid) return html + this.relationHtml(st.owner);

    html += this.queueHtml(st);
    const kinds = g.producible(st);
    if (kinds.length || st.isCity) {
      html += `<div class="cards">`;
      const wfull = g.workerCount(pid) + g.queuedCount(pid, false) >= g.workerCap(pid);
      const sfull = g.soldierCount(pid) + g.queuedCount(pid, true) >= g.soldierCap(pid);
      const qfull = st.queue.length >= MAX_QUEUE;
      for (const k of kinds) {
        const stt = UNIT[k];
        const avail = g.kindAvailable(k);
        const full = k === 'worker' ? wfull : sfull;
        const ok = !qfull && avail && !full && canAfford(p.res, stt.cost);
        const tip = `${g.unitName(k)} ${resStr(stt.cost)}\n` + (!avail ? '前307年 胡服騎射以降' : full ? '上限に達している（城を増やす・強化すると上限が上がる）' : unitTip(k));
        html += this.card('produce', plainUnitSvg(k, g.era === 'warring', 30), costHtml(stt.cost), ok, k, tip);
      }
      if (st.isCity) {
        const lvCap = cityLevelCap(g.era);
        if (st.level < lvCap) {
          const cost = cityUpgradeCost(st.level);
          html += this.card('upgrade', upgradeSvg(30), costHtml(cost), !qfull && !st.queue.some((q) => q.kind === 'upgrade') && canAfford(p.res, cost), '', `城を Lv${st.level + 1} に強化 ${resStr(cost)}\n城壁・領域・兵の上限が増える（この時代の上限 Lv${lvCap}）`);
        }
      }
      html += `</div>`;
      html += `<div class="row">${this.btn('rally', this.ds.mode === 'rally' ? '集結地点: 地点をクリック…' : st.rally ? '集結地点を変更' : '集結地点を設定', true, '', '右クリックでも設定できる。工人の集結地点を資源にすると自動で採集')}${st.rally ? this.btn('clearrally', '集結地点を解除', true) : ''}</div>`;
    }
    if (!st.isCity && st.kind === 'market') {
      html += `<div class="label">交易（相場は天下共通・手数料込み）</div><div class="trade">`;
      const keys: ResKey[] = ['food', 'wood', 'metal'];
      for (const a of keys) {
        for (const b of keys) {
          if (a === b) continue;
          const rate = g.tradeRate(a, b);
          html += this.btn('trade', `${resSvg(a, 13)}100 → ${resSvg(b, 13)}${Math.floor(100 * rate)}`, p.res[a] >= 100, `${a}:${b}`);
        }
      }
      html += `</div>`;
    }
    if (!st.isCity && st.kind === 'tower') html += `<div class="info">射程 5・敵兵を自動で射撃する</div>`;
    if (!st.isCity && st.kind === 'storehouse') html += `<div class="info">工人はここに資源を運び込める</div>`;
    return html;
  }

  private relationHtml(sid: number): string {
    const g = this.g;
    const pid = g.playerId;
    const s = g.states[sid];
    const war = g.atWar(pid, sid);
    let html = `<div class="info">国力 ${Math.round(g.power(sid))}（自国 ${Math.round(g.power(pid))}）・城 ${g.ownCities(sid).length}・生産力補正 ×${s.bias.toFixed(2)}</div>`;
    html += `<div class="info">関係: <b class="${war ? 'rel-war' : 'rel-peace'}">${war ? '交戦中' : '平和'}</b></div><div class="row">`;
    if (war) {
      const forced = g.isForcedWar(pid, sid);
      html += this.btn('peace', s.peaceOffers.has(pid) ? '講和を受諾' : '講和を申し入れる', !forced, String(sid), forced ? '盟約による戦争中は講和できない' : '');
    } else html += this.btn('war', '宣戦布告', true, String(sid), g.era === 'spring' ? '攻撃を続けると覇者の呼びかけを招く' : '');
    html += '</div>';
    return html;
  }

  private unitPanel(units: Unit[]): string {
    const g = this.g;
    const p = g.states[g.playerId];
    const warring = g.era === 'warring';
    const soldiers = units.filter((u) => u.kind !== 'worker');
    const workers = units.filter((u) => u.kind === 'worker');
    let html = '';
    if (units.length === 1) {
      const u = units[0];
      const st = u.kind === 'worker' ? this.workerState(u) : this.soldierState(u);
      html += `<div class="sel-head">${unitSvg(u.kind, p.color, warring, 48)}<div><h3>${g.unitName(u.kind)}</h3><div class="info">能力 ${Math.round(u.ability)}/${Math.round(u.maxAbility)}（上限 ${abilityCap(g.era)}）・${st}</div>` +
        `<div class="bar"><i style="width:${(u.ability / u.maxAbility) * 100}%"></i></div><div class="info">${unitTip(u.kind)}</div></div></div>`;
    } else {
      html += `<div class="units">`;
      for (const u of units.slice(0, 30)) {
        const frac = u.ability / u.maxAbility;
        html += `<div class="ut" data-act="focus" data-arg="${u.id}" title="${g.unitName(u.kind)} ${Math.round(u.ability)}/${Math.round(u.maxAbility)}">${unitSvg(u.kind, p.color, warring, 30)}<i style="width:${frac * 100}%;background:${frac > 0.5 ? '#7cb342' : frac > 0.25 ? '#ffc107' : '#f44336'}"></i><small>${Math.round(u.maxAbility)}</small></div>`;
      }
      html += `</div>`;
    }

    // 建設メニュー
    if (this.buildMenu && workers.length) {
      html += `<div class="label">建設（工人 ${workers.length}）— 地点をクリック・Shift で連続配置</div><div class="cards">`;
      html += this.card('place', plainBuildingSvg('city', 30), costHtml(NEW_CITY_COST), canAfford(p.res, NEW_CITY_COST), 'city', `城 ${resStr(NEW_CITY_COST)}\n領土を広げ、工人と歩兵を生産する。他の城から7以上離れた自領か無主地に建てる`);
      for (const k of BUILDING_KINDS) {
        const b = BUILDING[k];
        html += this.card('place', plainBuildingSvg(k, 30), costHtml(b.cost), canAfford(p.res, b.cost), k, `${b.name} ${resStr(b.cost)}\n${b.desc}`);
      }
      html += `</div><div class="row">${this.btn('closebuild', '戻る', true)}</div>`;
      return html;
    }

    const cap = abilityCap(g.era);
    const trainable = units.filter((u) => g.canTrain(u));
    const tcost: Res = { food: 0, wood: 0, metal: 0 };
    for (const u of trainable) {
      const c = g.trainCost(u);
      tcost.food += c.food;
      tcost.wood += c.wood;
      tcost.metal += c.metal;
    }
    html += `<div class="cards">`;
    html += this.card('train', upgradeSvg(30), trainable.length ? costHtml(tcost) : '領外', trainable.length > 0 && canAfford(p.res, tcost), '',
      `訓練: 能力 +25（上限 ${cap}・全国共通）\n自国領内にいるユニットのみ`);
    if (soldiers.length) {
      const rc: Res = { food: 0, wood: 0, metal: 0 };
      const need = soldiers.filter((u) => u.ability < u.maxAbility - 0.5 && g.map.ownerAt(u.x, u.y) === u.owner);
      for (const u of need) {
        const c = g.reinforceCost(u);
        rc.food += c.food;
        rc.metal += c.metal;
      }
      html += this.card('reinforce', `<svg viewBox="0 0 32 32" width="30" height="30"><path d="M13 4h6v9h9v6h-9v9h-6v-9H4v-6h9z" fill="#c0392b" stroke="#2a1a0e" stroke-width="1.4"/></svg>`,
        need.length ? costHtml(rc) : '—', need.length > 0 && canAfford(p.res, rc), '', '補充: 自国領内で兵力を最大まで回復');
      html += this.card('amove', `<svg viewBox="0 0 32 32" width="30" height="30"><path d="M5 27 L22 10 M17 6 L26 6 L26 15" fill="none" stroke="#c9a24a" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"/><path d="M5 27 L22 10" stroke="#2a1a0e" stroke-width="1"/></svg>`,
        '攻撃移動', true, '', '途中の敵と交戦しながら進軍する', 'A');
      const room = g.workerCap(p.id) - g.workerCount(p.id);
      html += this.card('disband', plainUnitSvg('worker', false, 30), `解散 ${Math.min(room, soldiers.length)}`, room > 0, '', '兵を解散して工人にする（能力は 2/3）');
    }
    if (workers.length) {
      html += this.card('openbuild', `<svg viewBox="0 0 32 32" width="30" height="30"><path d="M6 26 L18 14" stroke="#8a5a2b" stroke-width="3" stroke-linecap="round"/><path d="M15 7 L25 7 L27 13 L21 15 L17 11 Z" fill="#9aa3a8" stroke="#2a1a0e" stroke-width="1.2" stroke-linejoin="round"/></svg>`,
        '建設', true, '', '城・倉庫・兵営・望楼・市を建てる', 'B');
      const room = g.soldierCap(p.id) - g.soldierCount(p.id);
      for (const k of MILITARY_KINDS) {
        if (!g.kindAvailable(k) || k === 'ram') continue;
        const cc = g.conscriptCost(k);
        html += this.card('conscript', plainUnitSvg(k, warring, 30), costHtml(cc), room > 0 && canAfford(p.res, cc), k, `徴兵: ${g.unitName(k)} にする（能力は 2/3、装備費が必要）`);
      }
    }
    html += this.card('stop', `<svg viewBox="0 0 32 32" width="30" height="30"><rect x="7" y="7" width="18" height="18" rx="2" fill="#7a2a1a" stroke="#2a1a0e" stroke-width="1.4"/></svg>`, '停止', true);
    html += `</div>`;
    html += `<div class="row">${this.btn('cmdmode', this.cmdMode ? '命令モード: ON' : '命令モード: OFF', true, '', 'タッチ操作用: ON の間はタップで命令')}</div>`;
    return html;
  }

  private workerState(u: Unit): string {
    const o = u.order;
    if (o.type === 'gather') return `${RES_NAMES[o.res]}を採集`;
    if (o.type === 'build') return '建設';
    if (o.type === 'move') return '移動';
    return '待機';
  }
  private soldierState(u: Unit): string {
    const o = u.order;
    if (o.type === 'attack') return '攻撃';
    if (o.type === 'move') return o.attackMove ? '進軍' : '移動';
    return u.engageId ? '交戦' : '待機';
  }

  private panelAction(act: string, arg: string): void {
    const g = this.g;
    const pid = g.playerId;
    const units = this.selectedUnits();
    const st = g.structById.get(this.ds.selStruct);
    switch (act) {
      case 'produce':
        if (st) g.enqueue(st, arg as UnitKind);
        break;
      case 'upgrade':
        if (st) g.enqueue(st, 'upgrade');
        break;
      case 'cancel':
        if (st) g.cancelQueue(st, Number(arg));
        break;
      case 'rally':
        this.setMode('rally');
        break;
      case 'clearrally':
        if (st) st.rally = null;
        break;
      case 'trade': {
        const [a, b] = arg.split(':') as [ResKey, ResKey];
        g.trade(pid, a, b, 100);
        break;
      }
      case 'war':
        g.declareWar(pid, Number(arg));
        break;
      case 'peace':
        g.requestPeace(pid, Number(arg));
        break;
      case 'train':
        for (const u of units.filter((x) => g.canTrain(x))) g.train(u);
        break;
      case 'reinforce':
        for (const u of units) if (u.kind !== 'worker') g.reinforce(u);
        break;
      case 'disband':
        for (const u of units) {
          if (u.kind === 'worker') continue;
          const w = g.disband(u);
          if (w) {
            this.ds.selected.delete(u.id);
            this.ds.selected.add(w.id);
          }
        }
        break;
      case 'conscript':
        for (const u of units) {
          if (u.kind !== 'worker') continue;
          const s = g.conscript(u, arg as UnitKind);
          if (s) {
            this.ds.selected.delete(u.id);
            this.ds.selected.add(s.id);
          }
        }
        break;
      case 'amove':
        this.setMode('attackMove');
        break;
      case 'openbuild':
        this.buildMenu = true;
        break;
      case 'closebuild':
        this.buildMenu = false;
        this.setMode('normal');
        break;
      case 'place':
        this.ds.buildKind = arg as BuildingKind | 'city';
        this.setMode('build');
        if (this.ds.hover) this.updatePreview(this.ds.hover.x, this.ds.hover.y);
        break;
      case 'stop':
        for (const u of units) {
          u.order = { type: 'idle' };
          u.path = [];
          u.homeX = u.x;
          u.homeY = u.y;
          u.engageId = 0;
        }
        break;
      case 'focus': {
        const u = g.unitById.get(Number(arg));
        if (u) {
          this.r.centerOn(u.x, u.y);
          this.ds.selected.clear();
          this.ds.selected.add(u.id);
        }
        break;
      }
      case 'selmil':
        this.selectWhere((u) => u.kind !== 'worker');
        break;
      case 'selidle':
        this.nextIdleWorker();
        break;
      case 'home': {
        const cap = g.cities.find((c) => c.owner === pid && c.capital && !c.destroyed);
        if (cap) this.r.centerOn(cap.x, cap.y);
        break;
      }
      case 'cmdmode':
        this.cmdMode = !this.cmdMode;
        break;
    }
    this.refreshPanel(true);
    this.refreshTop();
  }

  // ---------- 諸国一覧 ----------
  openDiplo(): void {
    const g = this.g;
    const pid = g.playerId;
    const rows = g.states
      .filter((s) => s.alive)
      .map((s) => ({ s, p: g.power(s.id) }))
      .sort((a, b) => b.p - a.p);
    let html = `<table class="dip"><tr><th>#</th><th>国</th><th>城</th><th>工/兵</th><th>国力</th><th>生産補正</th><th>関係</th><th></th></tr>`;
    rows.forEach(({ s, p }, k) => {
      const me = s.id === pid;
      const war = g.atWar(pid, s.id);
      const forced = g.isForcedWar(pid, s.id);
      let act = '';
      if (!me) {
        if (war) {
          if (s.peaceOffers.has(pid)) act = `<button data-act="accept" data-arg="${s.id}">講和受諾</button>`;
          else act = `<button data-act="peace" data-arg="${s.id}" ${forced ? 'disabled' : ''}>講和申入</button>`;
        } else act = `<button data-act="war" data-arg="${s.id}">宣戦</button>`;
      }
      const enemies = g.enemiesOf(s.id).map((e) => g.states[e].name).join('・');
      html += `<tr class="${me ? 'me' : ''}"><td>${k + 1}</td><td><span class="chip" style="background:${s.color}"></span> <a href="#" data-act="goto" data-arg="${s.id}" style="color:inherit">${esc(s.name)}</a>${g.isSevenHero(s) ? ' <small style="color:#d4a246">七雄</small>' : ''}</td>` +
        `<td class="num">${g.ownCities(s.id).length}</td><td class="num">${g.workerCount(s.id)}/${g.soldierCount(s.id)}</td><td class="num">${Math.round(p)}</td><td class="num">×${s.bias.toFixed(2)}</td>` +
        `<td>${me ? '自国' : war ? `<span class="rel-war">交戦中${forced ? '(盟約)' : ''}</span>` : '<span class="rel-peace">平和</span>'}${enemies ? `<br><small style="color:#998">交戦: ${esc(enemies)}</small>` : ''}</td><td>${act}</td></tr>`;
    });
    html += `</table>`;
    const ev = g.event;
    if (ev) html = `<p style="color:#f1c40f">${esc(ev.title)}（〜${yearLabel(ev.endYear)}）</p>` + html;
    $('diplo-body').innerHTML = html;
    this.openOverlay('diplo');
  }
}

function unitTip(k: UnitKind): string {
  switch (k) {
    case 'worker': return '資源の採集・建設・修理を行う';
    case 'infantry': return '戈を持つ歩兵。戦車・騎兵・衝車に強い。城でも兵営でも生産できる';
    case 'chariot': return '戦車。歩兵・弓兵に強いが山林では弱い';
    case 'archer': return '弓兵（戦国期は弩兵）。射程が長く歩兵に強い。騎兵に弱い';
    case 'cavalry': return '騎兵。足が速く弓兵に強い（前307年以降）';
    case 'ram': return '衝車。城や建物に大打撃を与えるが兵とは戦えない。弓矢はほぼ効かない';
  }
}

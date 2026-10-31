import {
  MILITARY_KINDS, NEW_CITY_COST, RES_NAMES, UNIT, abilityCap, canAfford, cityLevelCap, cityMaxHp, cityUpgradeCost,
  resStr, yearLabel, type Res, type UnitKind,
} from './config';
import type { City, Game, Msg, Unit } from './game';
import { MW, TERRAIN_NAMES } from './map';
import type { DrawState, Renderer } from './render';

const $ = <T extends HTMLElement>(id: string): T => document.getElementById(id) as T;

function esc(s: string): string {
  return s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]!);
}

export class UI {
  ds: DrawState = { selected: new Set(), selCity: -1, dragRect: null, buildPreview: null, mode: 'normal', hover: null };
  speed = 1;
  prevSpeed = 1;
  modalOpen = false;
  cmdMode = false; // タッチ用
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
      const x = ((e.clientX - rect.left) / rect.width) * MW;
      const y = ((e.clientY - rect.top) / rect.height) * 210;
      this.r.centerOn(x, y);
    };
    mm.addEventListener('pointerdown', (e) => {
      mmMove(e);
      mm.setPointerCapture(e.pointerId);
    });
    mm.addEventListener('pointermove', (e) => {
      if (e.buttons) mmMove(e);
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
    if (this.ds.mode === 'build') {
      const tx = Math.floor(w.x);
      const ty = Math.floor(w.y);
      this.ds.buildPreview = { tx, ty, ok: this.g.playerId >= 0 && !this.g.canFoundCity(this.g.playerId, tx, ty) };
    }
    const d = this.dragStart;
    if (!d) return;
    if (Math.hypot(e.offsetX - d.x, e.offsetY - d.y) > 5) d.moved = true;
    if (!d.moved) return;
    if (d.button === 1 || d.button === 9 || (d.button === 0 && this.keys.has(' '))) {
      // パン
      if (prev) {
        this.r.cam.x -= (e.offsetX - prev.x) / this.r.cam.zoom;
        this.r.cam.y -= (e.offsetY - prev.y) / this.r.cam.zoom;
        this.r.clamp();
      }
    } else if (d.button === 0) {
      this.ds.dragRect = { x0: d.x, y0: d.y, x1: e.offsetX, y1: e.offsetY };
    }
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
        this.tryBuild(Math.floor(w.x), Math.floor(w.y));
        return;
      }
      if (this.ds.mode === 'attackMove') {
        this.command(w.x, w.y, true);
        this.setMode('normal');
        return;
      }
      if (d.button === 9 && this.cmdMode && this.ds.selected.size) {
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
    if (k === ' ') {
      e.preventDefault();
      this.setSpeed(this.speed === 0 ? this.prevSpeed || 1 : 0);
    } else if (k === 'escape') {
      this.setMode('normal');
      this.ds.selected.clear();
      this.ds.selCity = -1;
      this.selOther = -1;
      this.refreshPanel(true);
    } else if (k === 'a') {
      if (this.selectedUnits().some((u) => u.kind !== 'worker')) this.setMode('attackMove');
    } else if (k === 'b') {
      if (this.selectedUnits().some((u) => u.kind === 'worker')) this.setMode('build');
    } else if (k === 'q') {
      this.selectWhere((u) => u.kind !== 'worker');
    } else if (k === 'e') {
      this.selectWhere((u) => u.kind === 'worker' && u.order.type === 'idle');
    } else if (k === 'h') {
      const cap = this.g.cities.find((c) => c.owner === this.g.playerId && c.capital && !c.destroyed);
      if (cap) this.r.centerOn(cap.x, cap.y);
    } else if (k === '+' || k === '=') this.r.zoomAt(this.r.viewW / 2, this.r.viewH / 2, 1.2);
    else if (k === '-') this.r.zoomAt(this.r.viewW / 2, this.r.viewH / 2, 1 / 1.2);
  }

  /** 毎フレームのキー入力によるスクロール */
  tick(realDt: number): void {
    const sp = (600 * realDt) / this.r.cam.zoom;
    if (this.keys.has('ArrowLeft')) this.r.cam.x -= sp;
    if (this.keys.has('ArrowRight')) this.r.cam.x += sp;
    if (this.keys.has('ArrowUp')) this.r.cam.y -= sp;
    if (this.keys.has('ArrowDown')) this.r.cam.y += sp;
    if (this.keys.size) this.r.clamp();
    // 死んだユニットを選択から外す
    for (const id of this.ds.selected) if (!this.g.unitById.has(id) || this.g.unitById.get(id)!.owner !== this.g.playerId) this.ds.selected.delete(id);
    const c = this.g.cityById.get(this.ds.selCity);
    if (this.ds.selCity >= 0 && (!c || c.destroyed)) this.ds.selCity = -1;
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
    this.canvas.classList.toggle('cmd-attack', m === 'attackMove');
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
    this.ds.selCity = -1;
    this.selOther = -1;
    for (const u of this.g.units) if (!u.dead && u.owner === this.g.playerId && pred(u)) this.ds.selected.add(u.id);
    this.refreshPanel(true);
  }

  private pickUnit(x: number, y: number, pred: (u: Unit) => boolean): Unit | null {
    const rad = Math.max(0.6, 9 / this.r.cam.zoom);
    let best: Unit | null = null;
    let bd = rad;
    for (const u of this.g.units) {
      if (u.dead || !pred(u)) continue;
      const d = Math.hypot(u.x - x, u.y - y);
      if (d < bd) {
        bd = d;
        best = u;
      }
    }
    return best;
  }

  private pickCity(x: number, y: number): City | null {
    const rad = Math.max(1.2, 10 / this.r.cam.zoom);
    let best: City | null = null;
    let bd = rad;
    for (const c of this.g.cities) {
      if (c.destroyed) continue;
      const d = Math.hypot(c.x - x, c.y - y);
      if (d < bd) {
        bd = d;
        best = c;
      }
    }
    return best;
  }

  private clickSelect(x: number, y: number, add: boolean): void {
    const pid = this.g.playerId;
    const cityHit = this.pickCity(x, y);
    const onCity = cityHit && Math.hypot(cityHit.x - x, cityHit.y - y) < Math.max(0.7, 6 / this.r.cam.zoom);
    const own = onCity ? null : this.pickUnit(x, y, (u) => u.owner === pid);
    if (!add) {
      this.ds.selected.clear();
      this.ds.selCity = -1;
      this.selOther = -1;
    }
    if (own) {
      if (add && this.ds.selected.has(own.id)) this.ds.selected.delete(own.id);
      else this.ds.selected.add(own.id);
    } else {
      const c = this.pickCity(x, y);
      if (c) {
        this.ds.selected.clear();
        this.ds.selCity = c.id;
      } else {
        const other = this.pickUnit(x, y, () => true);
        if (other) this.selOther = other.id;
      }
    }
    this.refreshPanel(true);
  }

  private boxSelect(r: { x0: number; y0: number; x1: number; y1: number }, add: boolean): void {
    const a = this.r.screenToWorld(Math.min(r.x0, r.x1), Math.min(r.y0, r.y1));
    const b = this.r.screenToWorld(Math.max(r.x0, r.x1), Math.max(r.y0, r.y1));
    if (!add) this.ds.selected.clear();
    this.ds.selCity = -1;
    this.selOther = -1;
    const pid = this.g.playerId;
    const inBox = this.g.units.filter((u) => !u.dead && u.owner === pid && u.x >= a.x && u.x <= b.x && u.y >= a.y && u.y <= b.y);
    // 兵が含まれていれば兵のみ選択
    const hasMil = inBox.some((u) => u.kind !== 'worker');
    for (const u of inBox) if (!hasMil || u.kind !== 'worker' || add) this.ds.selected.add(u.id);
    this.refreshPanel(true);
  }

  // ---------- 命令 ----------
  private command(x: number, y: number, attackMove: boolean): void {
    const g = this.g;
    const pid = g.playerId;
    const units = this.selectedUnits();
    if (!units.length) return;
    const soldiers = units.filter((u) => u.kind !== 'worker');
    const workers = units.filter((u) => u.kind === 'worker');
    // 敵ユニット
    const enemy = this.pickUnit(x, y, (u) => u.owner !== pid);
    if (enemy && soldiers.length) {
      if (g.atWar(pid, enemy.owner)) {
        g.attackTarget(soldiers, { unit: enemy });
        if (workers.length) g.moveUnits(workers, x, y, false);
        return;
      }
      this.toast(`${g.states[enemy.owner].name} とは交戦していない（諸国一覧から宣戦できる）`);
    }
    const city = this.pickCity(x, y);
    if (city) {
      if (city.owner !== pid && soldiers.length) {
        if (g.atWar(pid, city.owner)) {
          g.attackTarget(soldiers, { city });
          if (workers.length) g.moveUnits(workers, x, y, false);
          return;
        }
        this.toast(`${g.states[city.owner].name} とは交戦していない`);
      }
      if (city.owner === pid && city.built < 1 && workers.length) {
        for (const w of workers) g.orderBuild(w, city);
        if (soldiers.length) g.moveUnits(soldiers, x, y, attackMove);
        return;
      }
    }
    // 採集
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

  private tryBuild(tx: number, ty: number): void {
    const g = this.g;
    const workers = this.selectedUnits().filter((u) => u.kind === 'worker');
    if (!workers.length) {
      this.setMode('normal');
      return;
    }
    const err = g.canFoundCity(g.playerId, tx, ty);
    if (err) {
      this.toast(err);
      return;
    }
    const c = g.foundCity(g.playerId, tx, ty, workers);
    if (c) this.toast(`${c.name} の築城を開始した`);
    this.setMode('normal');
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
      else if (b.dataset.act === 'accept') {
        this.g.makePeace(this.g.playerId, sid);
      } else if (b.dataset.act === 'goto') {
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
        `<span><span class="chip" style="display:inline-block;width:11px;height:11px;background:${p.color};border:1px solid #000"></span> ${esc(p.name)}</span>` +
        `<span class="${p.starving ? 'neg' : ''}">${RES_NAMES.food} ${Math.floor(r.food)} <small>(維持 -${(up * 6).toFixed(1)}/年)</small></span>` +
        `<span>${RES_NAMES.wood} ${Math.floor(r.wood)}</span><span>${RES_NAMES.metal} ${Math.floor(r.metal)}</span>` +
        `<span>工人 ${g.workerCount(p.id)}/${g.workerCap(p.id)}</span><span>兵 ${g.soldierCount(p.id)}/${g.soldierCap(p.id)}</span>` +
        (p.aggro > 20 && g.era === 'spring' ? `<span class="neg" title="周辺国への攻撃で高まる。70 を超えると覇者の呼びかけが起こる">諸侯の警戒 ${Math.round(p.aggro)}</span>` : '');
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
    const c = g.cityById.get(this.ds.selCity);
    if (c && !c.destroyed) {
      const owner = g.states[c.owner];
      const head = `<h3><span class="chip" style="background:${owner.color}"></span>${esc(owner.name)}・${esc(c.name)} ${c.capital ? '★' : ''} <small>Lv${c.level}</small></h3>`;
      const maxHp = cityMaxHp(c.level);
      let html = head;
      if (c.built < 1) {
        html += `<div class="info">築城中 ${Math.floor(c.built * 100)}%（工人を右クリックで送ると建設が進む）</div><div class="bar"><i style="width:${c.built * 100}%"></i></div>`;
        return html;
      }
      html += `<div class="info">城壁 ${Math.ceil(c.hp)}/${maxHp}</div><div class="bar"><i style="width:${(c.hp / maxHp) * 100}%"></i></div>`;
      if (c.owner === pid) {
        if (c.queue) {
          const q = c.queue;
          const nm = q.kind === 'upgrade' ? `城の強化 → Lv${c.level + 1}` : g.unitName(q.kind);
          html += `<div class="info">生産中: ${nm} ${Math.floor((q.progress / q.time) * 100)}%</div><div class="bar"><i style="width:${(q.progress / q.time) * 100}%;background:#d4a246"></i></div>`;
        }
        const wfull = g.workerCount(pid) + g.queuedCount(pid, false) >= g.workerCap(pid);
        const sfull = g.soldierCount(pid) + g.queuedCount(pid, true) >= g.soldierCap(pid);
        html += `<div class="label">生産</div><div class="row">`;
        const kinds: UnitKind[] = ['worker', ...MILITARY_KINDS];
        for (const k of kinds) {
          const st = UNIT[k];
          const avail = g.kindAvailable(k);
          const full = k === 'worker' ? wfull : sfull;
          const ok = !c.queue && avail && !full && canAfford(p.res, st.cost);
          const tip = !avail ? '前307年 胡服騎射以降' : full ? '上限に達している（城を増やすと上限が上がる）' : resStr(st.cost);
          html += this.btn('produce', `${g.unitName(k)}<br><small>${resStr(st.cost)}</small>`, ok, k, tip);
        }
        html += `</div>`;
        const lvCap = cityLevelCap(g.era);
        if (c.level < lvCap) {
          const cost = cityUpgradeCost(c.level);
          html += `<div class="row">${this.btn('upgrade', `城を強化 Lv${c.level + 1}<br><small>${resStr(cost)}</small>`, !c.queue && canAfford(p.res, cost), '', '城壁・領域・兵の上限が増える')}</div>`;
        } else html += `<div class="info">この時代の最大レベル（Lv${lvCap}）</div>`;
      } else {
        html += this.relationHtml(c.owner);
      }
      return html;
    }
    const other = g.unitById.get(this.selOther);
    if (other && !other.dead) {
      const s = g.states[other.owner];
      return `<h3><span class="chip" style="background:${s.color}"></span>${esc(s.name)} の ${g.unitName(other.kind)}</h3><div class="info">能力 ${Math.round(other.ability)}/${Math.round(other.maxAbility)}</div>${this.relationHtml(other.owner)}`;
    }
    // 何も選択していない
    const hov = this.ds.hover;
    let html = `<h3><span class="chip" style="background:${p.color}"></span>${esc(p.name)}</h3>`;
    html += `<div class="info">城 ${g.ownCities(pid).length} ・ 工人 ${g.workerCount(pid)}/${g.workerCap(pid)} ・ 兵 ${g.soldierCount(pid)}/${g.soldierCap(pid)} ・ 国力 ${Math.round(g.power(pid))}</div>`;
    const en = g.enemiesOf(pid).map((e) => g.states[e].name);
    html += `<div class="info">交戦中: ${en.length ? esc(en.join('・')) : 'なし'}</div>`;
    if (hov) {
      const tx = Math.floor(hov.x);
      const ty = Math.floor(hov.y);
      if (g.map.inBounds(tx, ty)) {
        const i = ty * MW + tx;
        const own = g.map.owner[i];
        const res = g.map.gatherType(tx, ty);
        html += `<div class="info">地形: ${TERRAIN_NAMES[g.map.terrain[i]]}${g.map.river[i] ? '（河川）' : ''}${g.map.mine[i] ? '・鉱山' : ''}${g.map.fertile[i] && res === 'food' ? '・肥沃' : ''} / ${own >= 0 ? esc(g.states[own].name) + '領' : '無主地'}${res ? ` / 採集: ${RES_NAMES[res]}` : ''}</div>`;
      }
    }
    html += `<div class="info">ユニットや城をクリックして選択。右クリックで命令。</div>`;
    html += `<div class="row">${this.btn('selmil', '全軍選択 (Q)', true)}${this.btn('selidle', '待機工人 (E)', true)}${this.btn('home', '都へ (H)', true)}${this.btn('cmdmode', this.cmdMode ? '命令モード: ON' : '命令モード: OFF', true, '', 'タッチ操作用: ON の間はタップで命令')}</div>`;
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
    const soldiers = units.filter((u) => u.kind !== 'worker');
    const workers = units.filter((u) => u.kind === 'worker');
    const counts = new Map<string, number>();
    for (const u of units) counts.set(g.unitName(u.kind), (counts.get(g.unitName(u.kind)) ?? 0) + 1);
    let html = `<h3>${[...counts.entries()].map(([k, n]) => `${k}×${n}`).join(' ')}</h3>`;
    html += `<div class="units">`;
    for (const u of units.slice(0, 24)) {
      const st = u.kind === 'worker' ? this.workerState(u) : this.soldierState(u);
      html += `<div data-act="focus" data-arg="${u.id}" title="${esc(st)}">${g.unitName(u.kind)} ${Math.round(u.ability)}/${Math.round(u.maxAbility)}<br><small>${st}</small></div>`;
    }
    html += `</div>`;
    const cap = abilityCap(g.era);
    // 訓練
    const trainable = units.filter((u) => g.canTrain(u));
    const tcost: Res = { food: 0, wood: 0, metal: 0 };
    for (const u of trainable) {
      const c = g.trainCost(u);
      tcost.food += c.food;
      tcost.wood += c.wood;
      tcost.metal += c.metal;
    }
    html += `<div class="label">強化（能力上限 ${cap}・全国共通）</div><div class="row">`;
    html += this.btn('train', `訓練 +25<br><small>${trainable.length ? resStr(tcost) : '自領内でのみ'}</small>`, trainable.length > 0 && canAfford(p.res, tcost), '', '自国領内にいるユニットの能力を高める');
    if (soldiers.length) {
      const rc: Res = { food: 0, wood: 0, metal: 0 };
      const need = soldiers.filter((u) => u.ability < u.maxAbility - 0.5 && g.map.ownerAt(u.x, u.y) === u.owner);
      for (const u of need) {
        const c = g.reinforceCost(u);
        rc.food += c.food;
        rc.metal += c.metal;
      }
      html += this.btn('reinforce', `補充<br><small>${need.length ? resStr(rc) : '不要/領外'}</small>`, need.length > 0 && canAfford(p.res, rc), '', '自国領内で兵力を最大まで回復');
    }
    html += `</div><div class="label">転換（能力は 2/3 に）</div><div class="row">`;
    if (soldiers.length) {
      const room = g.workerCap(p.id) - g.workerCount(p.id);
      html += this.btn('disband', `解散して工人に (${Math.min(room, soldiers.length)})`, room > 0, '', '兵を解散し工人にする。能力は 2/3');
    }
    if (workers.length) {
      const room = g.soldierCap(p.id) - g.soldierCount(p.id);
      for (const k of MILITARY_KINDS) {
        if (!g.kindAvailable(k)) continue;
        const cc = g.conscriptCost(k);
        html += this.btn('conscript', `徴兵:${g.unitName(k)}<br><small>${resStr(cc)}</small>`, room > 0 && canAfford(p.res, cc), k, '工人を兵にする。能力は 2/3（装備費が必要）');
      }
    }
    html += `</div><div class="row">`;
    if (soldiers.length) html += this.btn('amove', this.ds.mode === 'attackMove' ? '攻撃移動: 地点を指定…' : '攻撃移動 (A)', true);
    if (workers.length) html += this.btn('build', this.ds.mode === 'build' ? '築城: 場所をクリック…' : `築城 (B)<br><small>${resStr(NEW_CITY_COST)}</small>`, canAfford(p.res, NEW_CITY_COST), '', '工人で新たな城を築く（他の城から7以上離れた自領/無主地）');
    html += this.btn('stop', '停止', true);
    html += this.btn('cmdmode', this.cmdMode ? '命令モード: ON' : '命令モード: OFF', true, '', 'タッチ操作用: ON の間はタップで命令');
    html += `</div>`;
    return html;
  }

  private workerState(u: Unit): string {
    const o = u.order;
    if (o.type === 'gather') return `${RES_NAMES[o.res]}採集`;
    if (o.type === 'build') return '築城';
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
    switch (act) {
      case 'produce': {
        const c = g.cityById.get(this.ds.selCity);
        if (c) g.enqueue(c, arg as UnitKind);
        break;
      }
      case 'upgrade': {
        const c = g.cityById.get(this.ds.selCity);
        if (c) g.enqueue(c, 'upgrade');
        break;
      }
      case 'war':
        g.declareWar(pid, Number(arg));
        break;
      case 'peace':
        g.requestPeace(pid, Number(arg));
        break;
      case 'train': {
        const list = units.filter((u) => g.canTrain(u));
        for (const u of list) g.train(u);
        break;
      }
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
      case 'build':
        this.setMode('build');
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
        this.selectWhere((u) => u.kind === 'worker' && u.order.type === 'idle');
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
      html += `<tr class="${me ? 'me' : ''}"><td>${k + 1}</td><td><span class="chip" style="display:inline-block;width:11px;height:11px;background:${s.color};border:1px solid #000"></span> <a href="#" data-act="goto" data-arg="${s.id}" style="color:inherit">${esc(s.name)}</a>${g.isSevenHero(s) ? ' <small style="color:#d4a246">七雄</small>' : ''}</td>` +
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

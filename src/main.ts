import './style.css';
import { STATE_DEFS, SEVEN_HEROES } from './data/states';
import { Game, type FogMode } from './game';
import { Renderer } from './render';
import { UI } from './ui';

const $ = <T extends HTMLElement>(id: string): T => document.getElementById(id) as T;

function esc(s: string): string {
  return s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]!);
}

function setupStart(): void {
  const list = $('state-list');
  let chosen: string | null = null;
  const defs = STATE_DEFS.filter((d) => d.cities.length > 0);
  list.innerHTML = defs
    .map((d) => {
      const major = SEVEN_HEROES.includes(d.key) || d.key === 'jin';
      const stars = '★'.repeat(Math.max(1, Math.round((d.bias - 0.55) * 5)));
      return `<div class="state-card" data-key="${d.key}"><div class="nm"><span class="chip" style="background:${d.color}"></span>${esc(d.name)}${major ? '<span class="tag">七雄の系譜</span>' : ''}</div>` +
        `<div class="ds">${esc(d.desc)}</div><div class="ds">城 ${d.cities.length} ・ 生産力 <span style="color:#d4a246">${stars}</span></div></div>`;
    })
    .join('');
  list.addEventListener('click', (e) => {
    const card = (e.target as HTMLElement).closest('.state-card') as HTMLElement | null;
    if (!card) return;
    chosen = card.dataset.key!;
    list.querySelectorAll('.state-card').forEach((c) => c.classList.toggle('sel', c === card));
    const d = STATE_DEFS.find((x) => x.key === chosen)!;
    $('start-sel').textContent = `${d.name} を率いる`;
    ($('btn-start') as HTMLButtonElement).disabled = false;
  });
  list.addEventListener('dblclick', () => chosen && start(chosen));
  $('btn-start').addEventListener('click', () => chosen && start(chosen));
  $('btn-help2').addEventListener('click', () => $('help').classList.remove('hidden'));
  $('help-close').addEventListener('click', () => $('help').classList.add('hidden'));
}

function start(key: string): void {
  $('start').classList.add('hidden');
  const seed = Math.floor(Math.random() * 1e9);
  const fog = ($('fog-mode') as HTMLSelectElement).value as FogMode;
  const game = new Game(key, seed, fog);
  const canvas = $<HTMLCanvasElement>('view');
  const renderer = new Renderer(game, canvas, $<HTMLCanvasElement>('minimap'));
  renderer.resize();
  window.addEventListener('resize', () => renderer.resize());
  const ui = new UI(game, renderer, canvas);
  (window as unknown as { __yy: unknown }).__yy = { game, ui, renderer }; // デバッグ用
  const cap = game.cities.find((c) => c.owner === game.playerId && c.capital)!;
  renderer.centerOn(cap.x, cap.y);
  const p = game.player!;
  ui.showModal(`${p.name} の君主として`,
    `前770年、周は東の洛邑に遷った。諸侯が覇を競う春秋の世の始まりである。\n\n` +
    `工人に資源を集めさせ、倉庫・兵営・望楼・市を建て、国を強くせよ。\n` +
    `城を落とすには衝車が要る。\n` +
    `ユニットの数には限りがある。訓練で能力を高めよ。\n` +
    `兵は存在するだけで糧を食う。\n\n` +
    `周辺国を攻め続ければ、覇者が諸侯を率いて討伐に来る。\n` +
    `戦国の世で最強となれば、合従軍が組まれるだろう。`);

  const STEP = 0.1;
  let acc = 0;
  let last = performance.now();
  let ended = false;
  const frame = (now: number): void => {
    const realDt = Math.min(0.25, (now - last) / 1000);
    last = now;
    if (!ui.modalOpen && ui.speed > 0 && !game.over) {
      acc += realDt * ui.speed;
      let n = 0;
      while (acc >= STEP && n < 60) {
        game.update(STEP);
        acc -= STEP;
        n++;
        if (ui.modalOpen) break;
      }
      if (n >= 60) acc = 0;
    }
    ui.tick(realDt);
    renderer.draw(ui.ds, realDt);
    if (game.over && !ended) {
      ended = true;
      showEnd(game);
    }
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

function showEnd(game: Game): void {
  const o = game.over!;
  $('end-title').textContent = o.victory ? '天下に覇を唱う' : '時代の終わり';
  $('end-reason').textContent = o.reason;
  $('end-rank').innerHTML = o.ranking
    .slice(0, 15)
    .map((r, i) => `<div>${i + 1}. <span style="display:inline-block;width:11px;height:11px;background:${r.color};border:1px solid #000"></span> ${esc(r.name)} — 国力 ${r.power}・城 ${r.cities}${r.alive ? '' : '（滅亡）'}</div>`)
    .join('');
  $('end').classList.remove('hidden');
}

$('btn-restart').addEventListener('click', () => location.reload());
setupStart();

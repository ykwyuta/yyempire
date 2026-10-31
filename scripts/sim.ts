/** ヘッドレスで全 AI 対戦を走らせ、バランスを確認するスクリプト */
import { Game } from '../src/game';
import { yearLabel } from '../src/config';

const seed = Number(process.argv[2] ?? 20240);
const player = process.argv[3] ?? null;
const g = new Game(player, seed);
g.onEvent = (t) => console.log(`  [EVENT ${yearLabel(g.year)}] ${t}`);
g.onMessage = (m) => {
  if (m.kind === 'event' || /滅ぼされ/.test(m.text)) console.log(`  ${yearLabel(m.year)} ${m.text}`);
};
const t0 = Date.now();
let lastReport = g.year;
let maxStep = 0;
while (!g.over) {
  const s0 = performance.now();
  g.update(0.1);
  maxStep = Math.max(maxStep, performance.now() - s0);
  if (g.year - lastReport >= 50) {
    lastReport = g.year;
    const alive = g.states.filter((s) => s.alive).map((s) => `${s.name}${g.ownCities(s.id).length}(${g.unitsOf(s.id).length})`);
    console.log(`${yearLabel(g.year)} units=${g.units.length} wars=${g.wars.size} alive=${alive.length}: ${alive.join(' ')}`);
  }
}
console.log(g.over.reason);
console.log(g.over.ranking.slice(0, 12).map((r) => `${r.name}:${r.power}/${r.cities}${r.alive ? '' : '✝'}`).join(' '));
console.log(`elapsed ${(Date.now() - t0) / 1000}s, max step ${maxStep.toFixed(1)}ms`);

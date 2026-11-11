// Run with node tools/test-browser-recovery.cjs. No browser packages required.
const { test } = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const recovery = fs.readFileSync(path.join(root, 'web/browser-recovery.js'), 'utf8');
const main = fs.readFileSync(path.join(root, 'web/main.js'), 'utf8');

function setup({ confirm = true, offline = false, cacheDenied = false, withCaches = true } = {}) {
  const listeners = {};
  const button = { style: {}, addEventListener(type, fn) { (listeners[type] ??= []).push(fn); } };
  const calls = { removed: [], fetch: [], navigations: [], alerts: [] };
  const entries = ['/game/main.js?v=old', '/game/game.wasm', '/other/main.js', '/game/save.yysav', '/game/index.html', 'https://other.test/game/main.js'];
  const caches = {
    async keys() { if (cacheDenied) throw Error('Denied'); return ['shared']; },
    async open() { return {
      async keys() { return entries.map(url => ({ url: new URL(url, 'https://example.test').href })); },
      async delete(request) { calls.removed.push(request.url); return true; },
    }; },
  };
  const window = {
    confirm: () => confirm,
    alert: message => calls.alerts.push(message),
    addEventListener() {},
    ...(withCaches ? { caches } : {}),
  };
  const context = vm.createContext({
    window, caches, URL, console: { warn() {} },
    location: { href: 'https://example.test/game/?lang=en#keep', replace: url => calls.navigations.push(url) },
    document: {
      currentScript: { src: 'https://example.test/game/browser-recovery.js' },
      documentElement: { lang: 'en' },
      createElement: () => button,
      body: { dataset: { gameWasm: 'game.wasm' }, appendChild() {} },
    },
    MutationObserver: class { observe() {} },
    fetch: async (url, options) => { calls.fetch.push([String(url), options]); if (offline) throw Error('Offline'); return { ok: true }; },
    // Any attempt to clear the save/settings stores fails the test.
    indexedDB: new Proxy({}, { get() { throw Error('Must preserve IndexedDB'); } }),
    localStorage: new Proxy({}, { get() { throw Error('Must preserve localStorage'); } }),
  });
  vm.runInContext(recovery, context);
  return { context, calls, button, api: window.yyBrowserRecovery,
    click: () => listeners.click.at(-1)() };
}

test('refresh removes only game assets and preserves query/hash and save stores', async () => {
  const { calls, click, button } = setup();
  assert.match(button.textContent, /Refresh cache/);
  await click();
  assert.deepEqual(calls.removed.sort(), [
    'https://example.test/game/main.js?v=old', 'https://example.test/game/game.wasm', 'https://example.test/game/index.html',
  ].sort());
  assert.equal(calls.navigations.length, 1);
  const destination = new URL(calls.navigations[0]);
  assert.equal(destination.searchParams.get('lang'), 'en');
  assert.equal(destination.hash, '#keep');
  assert.ok(destination.searchParams.get('_yyrefresh'));
  assert.ok(calls.fetch.every(([, options]) => options.cache === 'reload'));
});

test('cancel does not touch caches or navigate', async () => {
  const { calls, click } = setup({ confirm: false });
  await click();
  assert.deepEqual(calls, { removed: [], fetch: [], navigations: [], alerts: [] });
});

test('Help can refresh with the floating button hidden; the error fallback can also refresh', async () => {
  const help = setup();
  assert.equal(help.button.hidden, true);
  await help.api.refresh();
  assert.equal(help.calls.navigations.length, 1);
  assert.equal(help.button.hidden, true);

  const fallback = setup();
  fallback.api.showFallback();
  assert.equal(fallback.button.hidden, false);
  await fallback.click();
  assert.equal(fallback.calls.navigations.length, 1);
});

test('offline keeps the game open and makes the button usable again', async () => {
  const { calls, click, api, button } = setup({ offline: true });
  await click();
  assert.equal(calls.navigations.length, 0);
  assert.equal(calls.removed.length, 0);
  assert.equal(api.reloading, false);
  assert.equal(button.disabled, false);
  assert.match(calls.alerts[0], /Offline/);
});

test('restricted or missing CacheStorage still refreshes via network', async () => {
  for (const options of [{ cacheDenied: true }, { withCaches: false }]) {
    const { calls, click } = setup(options);
    await click();
    assert.equal(calls.navigations.length, 1);
  }
});

function installSaveCode(context) {
  // Exercise the real storage functions without starting WebGL/Wasm.
  const source = main.slice(main.indexOf('async function writeSave('), main.indexOf('async function loadSlotInfo('));
  vm.runInContext(`
    let db = null;
    const memStore = new Map();
    const T = ja => ja;
    let completeWrite;
    let rejectWrite;
    function dbReq() { return new Promise((resolve, reject) => { completeWrite = resolve; rejectWrite = reject; }); }
    ${source}
    window.testSaves = { storePut, persistent() { db = {}; }, complete() { completeWrite(); }, fail() { rejectWrite(Error('Quota')); } };
  `, context);
  return context.window.testSaves;
}

test('memory-only saves prevent reload before cache changes', async () => {
  const { context, calls, click } = setup();
  const saves = installSaveCode(context);
  await saves.storePut(1, { data: new Uint8Array([1, 2, 3]) });
  await click();
  assert.equal(calls.navigations.length, 0);
  assert.equal(calls.fetch.length, 0);
  assert.equal(calls.alerts.length, 1);
});

test('reload waits until pending saves commit', async () => {
  const { context, calls, click } = setup();
  const saves = installSaveCode(context);
  saves.persistent();
  const write = saves.storePut(1, { data: [1, 2, 3] });
  const refresh = click();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(calls.fetch.length, 0);
  saves.complete();
  await write;
  await refresh;
  assert.equal(calls.navigations.length, 1);
});

test('a failed save blocks reload; a successful retry allows it', async () => {
  const { context, calls, click } = setup();
  const saves = installSaveCode(context);
  saves.persistent();
  const write = saves.storePut(1, { data: [1, 2, 3] });
  saves.fail();
  await assert.rejects(write, /Quota/);
  await click();
  assert.equal(calls.navigations.length, 0);
  assert.equal(calls.fetch.length, 0);
  const retry = saves.storePut(1, { data: [1, 2, 3] });
  saves.complete();
  await retry;
  await click();
  assert.equal(calls.navigations.length, 1);
});

test('HTML, Wasm and build packaging use the recovery path', () => {
  const html = fs.readFileSync(path.join(root, 'web/index.html'), 'utf8');
  const build = fs.readFileSync(path.join(root, 'build.sh'), 'utf8');
  const filename = main.match(/new URL\('\.\/([^']+\.wasm)'/)[1];
  assert.ok(html.includes(`data-game-wasm="${filename}"`));
  assert.ok(html.includes('src="./browser-recovery.js"'));
  assert.ok(html.includes("url.searchParams.set('_yyrefresh', refresh)"));
  assert.ok(main.includes("url.searchParams.set('_yyrefresh', refresh)"));
  assert.ok(main.includes("cache: refresh ? 'reload' : 'no-cache'"));
  assert.ok(build.includes('web/browser-recovery.js'));
  assert.ok(build.includes('/browser-recovery.js\n  Cache-Control: no-cache'));
});

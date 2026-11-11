// Runs independently of WebGL/Wasm so a failed game can still be refreshed.
(() => {
  const base = new URL('./', document.currentScript.src);
  const button = document.createElement('button');
  button.id = 'browser-recovery';
  button.type = 'button';
  button.hidden = true;
  button.style.cssText = 'position:fixed;right:max(10px,env(safe-area-inset-right));bottom:calc(var(--sb, 0px) + 4px);z-index:100;min-height:36px;padding:6px 10px;border:1px solid #a99b78;border-radius:6px;background:#202521;color:#fff;font:13px system-ui,sans-serif;touch-action:manipulation;cursor:pointer';
  const text = (ja, en, zh) => {
    const lang = document.documentElement.lang;
    return lang.startsWith('zh') ? zh : lang.startsWith('en') ? en : ja;
  };
  const label = () => {
    button.textContent = text('↻ キャッシュ更新', '↻ Refresh cache', '↻ 刷新缓存');
    button.title = text('セーブを残してキャッシュを更新し、再読み込み', 'Refresh cached game files and reload, keeping saved games', '保留存档，刷新缓存并重新加载');
  };
  label();
  new MutationObserver(label).observe(document.documentElement, { attributes: true, attributeFilter: ['lang'] });
  let saveGuard = async () => {};
  const api = window.yyBrowserRecovery = {
    reloading: false,
    setSaveGuard(guard) { saveGuard = guard; },
    showFallback() { button.hidden = false; },
    refresh,
  };
  // Do not send the button's input (including Enter/Space) into the game.
  for (const type of ['pointerdown', 'pointerup', 'click', 'keydown', 'keyup']) {
    button.addEventListener(type, (event) => event.stopPropagation());
  }
  for (const type of ['pointerdown', 'pointerup', 'pointermove', 'keydown', 'keyup', 'wheel']) {
    window.addEventListener(type, (event) => {
      if (api.reloading) {
        event.preventDefault();
        event.stopImmediatePropagation();
      }
    }, { capture: true, passive: false });
  }
  async function refresh() {
    if (api.reloading) return;
    if (!window.confirm(text(
      'セーブデータと設定を残し、ゲームのキャッシュを更新して再読み込みします。未保存の進行は失われます。必要なら先にF5の記録画面で保存してください。続けますか？',
      'Refresh cached game files and reload? Saved games and settings will be kept. Unsaved progress will be lost. Save first in the F5 save screen if needed.',
      '保留存档和设置，刷新游戏缓存并重新加载。未保存的进度将丢失。请根据需要先在F5存档画面保存。继续吗？'
    ))) return;
    api.reloading = true;
    button.disabled = true;
    button.textContent = text('更新中…', 'Refreshing…', '正在刷新…');
    try {
      await saveGuard();
      const next = new URL(location.href);
      next.searchParams.set('_yyrefresh', Date.now().toString(36));
      // Refresh the HTTP cache and check connectivity before leaving the game.
      const response = await fetch(next, { cache: 'reload' });
      if (!response.ok) throw new Error(text('ゲームを取得できません。通信状態を確認してください。', 'Could not fetch the game. Check your connection.', '无法获取游戏，请检查网络连接。'));
      // CacheStorage is separate from IndexedDB/localStorage. In a shared
      // origin, remove only this game's known files, keeping other games.
      const assets = new Set([base.href, next.origin + next.pathname,
        ...['index.html', 'main.js', 'browser-recovery.js', 'manifest.webmanifest', 'favicon.ico', 'icon-192.png', 'icon-512.png', 'apple-touch-icon.png', 'key-art.jpg']
          .map((file) => new URL(file, base).href)]);
      // The Wasm filename is supplied in the HTML (directory names may differ).
      if (document.body.dataset.gameWasm) assets.add(new URL(document.body.dataset.gameWasm, base).href);
      try {
        if ('caches' in window) {
          for (const name of await caches.keys()) {
            const cache = await caches.open(name);
            for (const request of await cache.keys()) {
              const url = new URL(request.url);
              url.search = '';
              url.hash = '';
              if (assets.has(url.href)) await cache.delete(request);
            }
          }
        }
      } catch (error) {
        // Restricted storage must not prevent the network refresh below.
        console.warn('Game cache unavailable; refreshing via network', error);
      }
      // Refresh the independent recovery script too; main.js and Wasm receive
      // the new token on the next load, bypassing their previous HTTP entries.
      const recovery = await fetch(new URL('browser-recovery.js', base), { cache: 'reload' });
      if (!recovery.ok) throw new Error(text('更新ファイルを取得できません。もう一度お試しください。', 'Could not fetch the update. Please try again.', '无法获取更新文件，请重试。'));
      location.replace(next.href);
    } catch (error) {
      api.reloading = false;
      button.disabled = false;
      label();
      window.alert(text('再読み込みを中止しました。', 'Reload cancelled.', '已取消重新加载。') + '\n' + (error.message || error));
    }
  }
  button.addEventListener('click', refresh);
  document.body.appendChild(button);
})();

// 春秋戦国 — ブラウザ側のつなぎ目
// Rust（WebAssembly）の描画命令を WebGL2 と Canvas2D に、ブラウザの入力を Rust に渡す。

const glCanvas = document.getElementById('gl');
const uiCanvas = document.getElementById('ui');
const loading = document.getElementById('loading');
const loadingMsg = document.getElementById('loading-msg');

// ------------------------------------------------------------------ 表示言語（0=日本語 1=English 2=中文、lang.rs の Lang と同じ）
// 書体はデスクトップ版に合わせ、日本語は游ゴシック、中国語は微軟雅黒、英語は Segoe UI。無ければ Noto Sans で代える
const FAMILIES = [
  '"Yu Gothic UI", "Yu Gothic", "Hiragino Kaku Gothic ProN", "Hiragino Sans", Meiryo, "Noto Sans JP", sans-serif',
  '"Segoe UI", system-ui, -apple-system, Roboto, "Helvetica Neue", Arial, "Noto Sans JP", sans-serif',
  '"Microsoft YaHei UI", "Microsoft YaHei", "PingFang SC", "Hiragino Sans GB", "Noto Sans SC", "Noto Sans JP", sans-serif',
];
const LANG_CODES = ['ja', 'en', 'zh'];
const TITLES = ['春秋戦国 — 中華統一への道', 'Warring States — The Road to Unification', '春秋战国 — 中华统一之路'];
const LANG_KEY = 'yyempire.lang';
// 起動時の言語：URL の ?lang=、保存した設定、ブラウザの言語の順
function detectLang() {
  const q = new URLSearchParams(location.search).get('lang');
  const pick = (c) => {
    c = (c || '').toLowerCase();
    if (c.startsWith('ja')) return 0;
    if (c.startsWith('zh')) return 2;
    if (c.startsWith('en')) return 1;
    return -1;
  };
  if (pick(q) >= 0) return pick(q);
  try {
    const saved = localStorage.getItem(LANG_KEY);
    if (saved !== null && pick(saved) >= 0) return pick(saved);
  } catch {}
  for (const l of navigator.languages || [navigator.language]) {
    const k = pick(l);
    if (k >= 0) return k;
  }
  return 1;
}
let curLang = detectLang();
// JS 側で出す文
const T = (ja, en, zh) => [ja, en, zh][curLang] ?? ja;
const rotateMsg = document.getElementById('rotate-msg');
if (rotateMsg) rotateMsg.textContent = T('画面を横向きにしてお遊びください', 'Please rotate your device to landscape', '请将设备横向放置');
let FAMILY = FAMILIES[curLang];
// gfx::Font と同じ並び：[大きさ, 中央揃え, 太字]
const FONTS = [
  [12, false, false], // Small
  [14, false, false], // Normal
  [14, true, false], // NormalCenter
  [18, false, true], // Big
  [20, true, true], // BigCenter
  [40, true, true], // Title
  [30, true, true], // TitleMid（題字が扁額に収まらないとき）
];
const LINE = 1.33;
// この割合まではみ出す行は横に詰めて1行に収める
const SQUEEZE = 1.18;
let fontCss = [];
function setLang(l) {
  curLang = l;
  FAMILY = FAMILIES[l];
  fontCss = FONTS.map(([sz, , bold]) => `${bold ? '700' : '400'} ${sz}px ${FAMILY}`);
  document.documentElement.lang = LANG_CODES[l];
  document.title = TITLES[l];
}
setLang(curLang);

let wasm = null;
let memBuf = null;
let u8 = null;
let f32 = null;
const dec = new TextDecoder('utf-8');

function views() {
  if (memBuf !== wasm.memory.buffer) {
    memBuf = wasm.memory.buffer;
    u8 = new Uint8Array(memBuf);
    f32 = new Float32Array(memBuf);
  }
}
function str(ptr, len) {
  views();
  return dec.decode(u8.subarray(ptr, ptr + len));
}

// ------------------------------------------------------------------ WebGL2（Direct3D11 相当）
const gl = glCanvas.getContext('webgl2', { alpha: false, antialias: true, premultipliedAlpha: false, preserveDrawingBuffer: false });
if (!gl) throw new Error(T('WebGL2 に対応したブラウザが必要です', 'A browser with WebGL2 is required', '需要支持 WebGL2 的浏览器'));
const VS = `#version 300 es
in vec2 pos; in vec2 uv; in vec4 col;
uniform vec4 xf;
out vec2 vuv; out vec4 vcol;
void main() { gl_Position = vec4(pos * xf.xy + xf.zw, 0.0, 1.0); vuv = uv; vcol = col; }`;
const FS = `#version 300 es
precision mediump float;
in vec2 vuv; in vec4 vcol;
uniform sampler2D tex; uniform int useTex;
out vec4 o;
void main() { o = useTex == 1 ? texture(tex, vuv) * vcol : vcol; }`;
function compile(type, src) {
  const s = gl.createShader(type);
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
  return s;
}
const prog = gl.createProgram();
gl.attachShader(prog, compile(gl.VERTEX_SHADER, VS));
gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, FS));
gl.bindAttribLocation(prog, 0, 'pos');
gl.bindAttribLocation(prog, 1, 'uv');
gl.bindAttribLocation(prog, 2, 'col');
gl.linkProgram(prog);
if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog));
gl.useProgram(prog);
const uXf = gl.getUniformLocation(prog, 'xf');
const uUseTex = gl.getUniformLocation(prog, 'useTex');
gl.uniform1i(gl.getUniformLocation(prog, 'tex'), 0);
const vbo = gl.createBuffer();
gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
gl.enableVertexAttribArray(0);
gl.enableVertexAttribArray(1);
gl.enableVertexAttribArray(2);
gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 32, 0);
gl.vertexAttribPointer(1, 2, gl.FLOAT, false, 32, 8);
gl.vertexAttribPointer(2, 4, gl.FLOAT, false, 32, 16);
gl.enable(gl.BLEND);
gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
gl.disable(gl.CULL_FACE);
gl.disable(gl.DEPTH_TEST);
gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
const textures = [null];

// ------------------------------------------------------------------ Canvas2D（Direct2D / DirectWrite 相当）
const ctx = uiCanvas.getContext('2d');
const bitmaps = [];
const colorCache = new Map();
function css(r, g, b, a) {
  const k = ((r * 255) | 0) * 16777216 + ((g * 255) | 0) * 65536 + ((b * 255) | 0) * 256 + ((Math.min(1, Math.max(0, a)) * 255) | 0);
  let s = colorCache.get(k);
  if (!s) {
    s = `rgba(${Math.round(r * 255)},${Math.round(g * 255)},${Math.round(b * 255)},${Math.min(1, Math.max(0, a))})`;
    if (colorCache.size > 4000) colorCache.clear();
    colorCache.set(k, s);
  }
  return s;
}

// 行頭に来てはいけない文字（禁則）
const NO_START = new Set('、。，．・：；？！ー－）」』】〕｝〉》’”ぁぃぅぇぉっゃゅょゎァィゥェォッャュョヮヵヶ％‰…‥,.:;!?)]}%'.split(''));
const measureCache = new Map();
const wrapCache = new Map();
let curFont = -1;
let curUi = 1;
function setFont(f) {
  if (curFont !== f) {
    ctx.font = fontCss[f];
    curFont = f;
  }
}
function width(f, s) {
  const k = f + '\u0001' + s;
  let w = measureCache.get(k);
  if (w === undefined) {
    setFont(f);
    w = ctx.measureText(s).width;
    if (measureCache.size > 20000) measureCache.clear();
    measureCache.set(k, w);
  }
  return w;
}
// 英数字の並びは一語、それ以外は一字ずつ折り返せる
const TOKEN = /[A-Za-z0-9_'’\-+.,%/#()[\]$&=]+[ \t]*|[ \t]+|[\s\S]/gu;
function wrap(f, s, maxW) {
  const k = f + '\u0001' + maxW + '\u0001' + s;
  let out = wrapCache.get(k);
  if (out) return out;
  out = [];
  for (const para of s.split('\n')) {
    // 少しだけはみ出す行は折り返さず、描画時に横方向に詰める（Windows の Yu Gothic UI より幅の広い書体への対策）
    if (!(maxW > 0) || width(f, para) <= maxW * SQUEEZE + 0.5) {
      out.push(para);
      continue;
    }
    let line = '';
    for (const tok of para.match(TOKEN) || []) {
      const cand = line + tok;
      if (line !== '' && width(f, cand.trimEnd()) > maxW + 0.5 && !NO_START.has(tok[0])) {
        out.push(line.trimEnd());
        line = tok.trimStart();
        // 一語が幅を超えるなら字で割る
        while (line.length > 1 && width(f, line) > maxW + 0.5) {
          let n = line.length - 1;
          while (n > 1 && width(f, line.slice(0, n)) > maxW + 0.5) n--;
          out.push(line.slice(0, n));
          line = line.slice(n);
        }
      } else line = cand;
    }
    out.push(line.trimEnd());
  }
  if (wrapCache.size > 4000) wrapCache.clear();
  wrapCache.set(k, out);
  return out;
}
function clearTextCaches() {
  measureCache.clear();
  wrapCache.clear();
  curFont = -1;
}

// ------------------------------------------------------------------ 記録（IndexedDB）
// 枠 0 は自動記録、1〜3 は手動。{ data: ArrayBuffer, meta: 見出し, time: 保存時刻 } を保存する
const DB_NAME = 'yyempire';
const STORE = 'slots';
let db = null;
const memStore = new Map(); // IndexedDB が使えないときの代わり（この画面を閉じるまで）
const slotInfo = new Map(); // 枠 → { meta, time }
const enc = new TextEncoder();

function openDb() {
  return new Promise((resolve) => {
    try {
      const req = indexedDB.open(DB_NAME, 1);
      req.onupgradeneeded = () => req.result.createObjectStore(STORE);
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => resolve(null);
      req.onblocked = () => resolve(null);
    } catch {
      resolve(null);
    }
  });
}
function dbReq(mode, f) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, mode);
    const r = f(tx.objectStore(STORE));
    tx.oncomplete = () => resolve(r && r.result);
    tx.onerror = () => reject(tx.error);
    tx.onabort = () => reject(tx.error);
  });
}
async function storeGet(slot) {
  if (!db) return memStore.get(slot) ?? null;
  return (await dbReq('readonly', (st) => st.get(slot))) ?? null;
}
async function writeSave(slot, rec) {
  if (!db) {
    memStore.set(slot, rec);
    return;
  }
  await dbReq('readwrite', (st) => st.put(rec, slot));
}
// Keep saved games intact: wait for writes and refuse to discard memory-only saves.
const pendingSaves = new Set();
const failedSaveSlots = new Set();
function storePut(slot, rec) {
  const write = writeSave(slot, rec);
  pendingSaves.add(write);
  write.then(() => {
    pendingSaves.delete(write);
    failedSaveSlots.delete(slot);
  }, () => {
    pendingSaves.delete(write);
    failedSaveSlots.add(slot);
  });
  return write;
}
window.yyBrowserRecovery?.setSaveGuard(async () => {
  await Promise.allSettled([...pendingSaves]);
  if (failedSaveSlots.size || (!db && memStore.size)) {
    throw new Error(T('記録を安全に保存できていないため、再読み込みできません。F5の記録画面で保存し直すか、ファイルに書き出して退避してください。', 'Reload blocked: some saves are not stored safely. Save again or export them to files from the F5 save screen.', '无法重新加载：有存档尚未安全保存。请在F5存档画面重新保存，或导出文件备份。'));
  }
});

async function loadSlotInfo() {
  db = await openDb();
  for (let s = 0; s < 4; s++) {
    try {
      const rec = await storeGet(s);
      if (rec) slotInfo.set(s, { meta: rec.meta, time: rec.time });
    } catch {}
  }
}
function fmtDate(t) {
  const d = new Date(t);
  const p = (n) => String(n).padStart(2, '0');
  return `${d.getFullYear()}/${p(d.getMonth() + 1)}/${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}
// Rust の領域に書き込んで渡す
function passBytes(bytes, fn) {
  const p = wasm.yy_alloc(bytes.length);
  views();
  u8.set(bytes, p);
  fn(p, bytes.length);
}
function notify(text) {
  passBytes(enc.encode(text), (p, l) => wasm.yy_notice(p, l));
}
function deliver(buf) {
  passBytes(new Uint8Array(buf), (p, l) => wasm.yy_loaded(p, l));
}
function download(rec, slot) {
  // 端末によっては日本語のファイル名が使えないので英数字にする
  const bc = /紀元前(\d+)年/.exec(rec.meta || '');
  const d = new Date(rec.time || Date.now());
  const p2 = (n) => String(n).padStart(2, '0');
  const stamp = `${d.getFullYear()}${p2(d.getMonth() + 1)}${p2(d.getDate())}-${p2(d.getHours())}${p2(d.getMinutes())}`;
  const name = `yyempire-${slot === 0 ? 'auto' : 'slot' + slot}${bc ? '-BC' + bc[1] : ''}-${stamp}.yysav`;
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([rec.data], { type: 'application/octet-stream' }));
  a.download = name;
  document.body.appendChild(a);
  a.click();
  setTimeout(() => {
    URL.revokeObjectURL(a.href);
    a.remove();
  }, 1000);
}
function pickFile() {
  const inp = document.createElement('input');
  inp.type = 'file';
  inp.accept = '.yysav,application/octet-stream';
  inp.onchange = async () => {
    const f = inp.files && inp.files[0];
    if (!f) return;
    try {
      deliver(await f.arrayBuffer());
    } catch (e) {
      notify(T('ファイルを読めなかった：', 'Could not read the file: ', '无法读取文件：') + e);
    }
  };
  inp.click();
}

// ------------------------------------------------------------------ Rust から呼ばれる関数
const env = {
  js_save: (slot, p, l, mp, ml) => {
    views();
    const data = u8.slice(p, p + l).buffer;
    const meta = str(mp, ml);
    const rec = { data, meta, time: Date.now() };
    const prev = slotInfo.get(slot);
    slotInfo.set(slot, { meta, time: rec.time });
    storePut(slot, rec).catch((e) => {
      if (prev) slotInfo.set(slot, prev);
      else slotInfo.delete(slot);
      notify(T('記録できなかった：', 'Could not save: ', '无法保存：') + (e && e.name === 'QuotaExceededError' ? T('保存領域が足りない', 'not enough storage', '存储空间不足') : e));
    });
    if (!db && slot !== 0 && !window.__yyMemWarned) {
      window.__yyMemWarned = true;
      notify(T('このブラウザでは記録を保存できない。画面を閉じる前にファイルへ書き出してください', 'This browser cannot keep saves. Export a file before closing the page.', '此浏览器无法保存存档。关闭页面前请导出文件。'));
    }
  },
  js_load: (slot) => {
    storeGet(slot)
      .then((rec) => (rec ? deliver(rec.data) : notify(T('この枠には記録がない', 'This slot is empty', '此栏位没有存档'))))
      .catch((e) => notify(T('読み込めなかった：', 'Could not load: ', '无法读取：') + e));
  },
  js_slot_info: (slot, out, cap) => {
    const info = slotInfo.get(slot);
    if (!info) return 0;
    const b = enc.encode(`${info.meta}\t${fmtDate(info.time)}`);
    if (b.length > cap) return 0;
    views();
    u8.set(b, out);
    return b.length;
  },
  js_export: (slot) => {
    storeGet(slot)
      .then((rec) => (rec ? download(rec, slot) : notify(T('この枠には記録がない', 'This slot is empty', '此栏位没有存档'))))
      .catch((e) => notify(T('書き出せなかった：', 'Could not export: ', '无法导出：') + e));
  },
  js_import: () => pickFile(),

  js_now: () => performance.now(),
  js_random: () => (Math.random() * 0xffffffff) >>> 0,
  js_log: (p, l, level) => {
    const s = str(p, l);
    if (level >= 2) {
      console.error(s);
      showError(s);
    } else console.log(s);
  },

  js_tex_create: (w, h, p) => {
    views();
    const t = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, t);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, u8.subarray(p, p + w * h * 4));
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    textures.push(t);
    return textures.length - 1;
  },
  js_tex_update: (id, x, y, w, h, p) => {
    views();
    gl.bindTexture(gl.TEXTURE_2D, textures[id]);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, x, y, w, h, gl.RGBA, gl.UNSIGNED_BYTE, u8.subarray(p, p + w * h * 4));
  },
  js_gl_begin: (r, g, b, a) => {
    gl.viewport(0, 0, glCanvas.width, glCanvas.height);
    gl.clearColor(r, g, b, a);
    gl.clear(gl.COLOR_BUFFER_BIT);
  },
  js_gl_xform: (sx, sy, ox, oy) => gl.uniform4f(uXf, sx, sy, ox, oy),
  js_gl_draw: (p, n, tex, linear) => {
    views();
    gl.bufferData(gl.ARRAY_BUFFER, f32.subarray(p >> 2, (p >> 2) + n * 8), gl.STREAM_DRAW);
    if (tex) {
      gl.bindTexture(gl.TEXTURE_2D, textures[tex]);
      const f = linear ? gl.LINEAR : gl.NEAREST;
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, f);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, f);
      gl.uniform1i(uUseTex, 1);
    } else gl.uniform1i(uUseTex, 0);
    gl.drawArrays(gl.TRIANGLES, 0, n);
  },

  js_2d_begin: (ui) => {
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, uiCanvas.width, uiCanvas.height);
    ctx.setTransform(ui, 0, 0, ui, 0, 0);
    curUi = ui;
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';
    ctx.globalAlpha = 1;
  },
  js_2d_end: () => {},
  // 縦に収まらない一覧を、ずらして描く（実ピクセル）
  js_2d_offset: (dy) => ctx.setTransform(curUi, 0, 0, curUi, 0, dy),
  js_fill_rect: (x0, y0, x1, y1, r, g, b, a) => {
    ctx.fillStyle = css(r, g, b, a);
    ctx.fillRect(x0, y0, x1 - x0, y1 - y0);
  },
  js_stroke_rect: (x0, y0, x1, y1, w, r, g, b, a) => {
    ctx.strokeStyle = css(r, g, b, a);
    ctx.lineWidth = w;
    ctx.strokeRect(x0, y0, x1 - x0, y1 - y0);
  },
  js_fill_round: (x0, y0, x1, y1, rad, r, g, b, a) => {
    ctx.fillStyle = css(r, g, b, a);
    ctx.beginPath();
    ctx.roundRect(x0, y0, x1 - x0, y1 - y0, Math.max(0, rad));
    ctx.fill();
  },
  js_stroke_round: (x0, y0, x1, y1, rad, w, r, g, b, a) => {
    ctx.strokeStyle = css(r, g, b, a);
    ctx.lineWidth = w;
    ctx.beginPath();
    ctx.roundRect(x0, y0, x1 - x0, y1 - y0, Math.max(0, rad));
    ctx.stroke();
  },
  js_fill_circle: (x, y, rad, r, g, b, a) => {
    ctx.fillStyle = css(r, g, b, a);
    ctx.beginPath();
    ctx.arc(x, y, Math.max(0, rad), 0, Math.PI * 2);
    ctx.fill();
  },
  js_stroke_circle: (x, y, rad, w, r, g, b, a) => {
    ctx.strokeStyle = css(r, g, b, a);
    ctx.lineWidth = w;
    ctx.beginPath();
    ctx.arc(x, y, Math.max(0, rad), 0, Math.PI * 2);
    ctx.stroke();
  },
  js_text: (p, l, x, y, w, h, f, r, g, b, a) => {
    const s = str(p, l);
    if (!s) return;
    const [size, center] = FONTS[f];
    const lh = size * LINE;
    const lines = wrap(f, s, w);
    setFont(f);
    ctx.fillStyle = css(r, g, b, a);
    ctx.textBaseline = 'middle';
    let ty = y + lh / 2;
    if (center) ty += (h - lines.length * lh) / 2;
    for (const line of lines) {
      const lw = width(f, line);
      if (w > 0 && lw > w + 0.5) {
        // 横に詰めて枠に収める
        ctx.save();
        ctx.translate(x, ty);
        ctx.scale(w / lw, 1);
        ctx.fillText(line, 0, 0);
        ctx.restore();
      } else {
        const lx = center ? x + (w - lw) / 2 : x;
        ctx.fillText(line, lx, ty);
      }
      ty += lh;
    }
  },
  js_measure: (p, l, f) => {
    const s = str(p, l);
    let m = 0;
    for (const line of s.split('\n')) m = Math.max(m, width(f, line));
    return m;
  },
  js_measure_h: (p, l, f, w) => wrap(f, str(p, l), w).length * FONTS[f][0] * LINE,
  js_bmp_create: (w, h, p) => {
    const c = document.createElement('canvas');
    c.width = w;
    c.height = h;
    bitmaps.push(c);
    putBgra(c, w, h, p);
    return bitmaps.length - 1;
  },
  js_bmp_update: (id, w, p) => {
    const c = bitmaps[id];
    if (c) putBgra(c, w, c.height, p);
  },
  js_bmp_draw: (id, x0, y0, x1, y1, alpha) => {
    const c = bitmaps[id];
    if (!c || alpha <= 0) return;
    ctx.globalAlpha = Math.min(1, alpha);
    ctx.drawImage(c, x0, y0, x1 - x0, y1 - y0);
    ctx.globalAlpha = 1;
  },

  js_lang_get: () => curLang,
  js_lang_save: (l) => {
    try {
      localStorage.setItem(LANG_KEY, LANG_CODES[l] || 'ja');
    } catch {}
  },
  js_font_lang: (l) => {
    setLang(l);
    clearTextCaches();
    // その言語の書体が届いたら測り直す
    document.fonts.load(`400 14px ${FAMILY}`, '春秋战国 Warring').then(clearTextCaches, () => {});
  },

  js_fullscreen_toggle: () => {
    if (document.fullscreenElement) document.exitFullscreen?.();
    else {
      const el = document.documentElement;
      const req = el.requestFullscreen || el.webkitRequestFullscreen;
      // スマートフォンでは全画面にしたうえで横向きに固定する
      Promise.resolve(req?.call(el, { navigationUI: 'hide' }))
        .then(() => screen.orientation?.lock?.('landscape'))
        .catch(() => {});
    }
  },
  js_is_fullscreen: () => (document.fullscreenElement ? 1 : 0),
};

// BGRA（乗算済みアルファ）→ RGBA（非乗算）
function putBgra(c, w, h, p) {
  views();
  const src = u8.subarray(p, p + w * h * 4);
  const img = new ImageData(w, h);
  const d = img.data;
  for (let i = 0; i < src.length; i += 4) {
    const a = src[i + 3];
    if (a === 0) continue;
    const k = a === 255 ? 1 : 255 / a;
    d[i] = src[i + 2] * k;
    d[i + 1] = src[i + 1] * k;
    d[i + 2] = src[i] * k;
    d[i + 3] = a;
  }
  c.getContext('2d').putImageData(img, 0, 0);
}

function showError(s) {
  loading.classList.remove('hidden');
  loadingMsg.innerHTML = '';
  const e = document.createElement('div');
  e.className = 'err';
  e.textContent = T('エラーが発生しました：', 'An error occurred:', '发生错误：') + '\n' + s;
  loadingMsg.appendChild(e);
}

// ------------------------------------------------------------------ 画面寸法
let pw = 0;
let ph = 0;
// 指で操作する端末か（?touch=1 / ?touch=0 で強制）
const QS = new URLSearchParams(location.search);
const coarse = () => {
  if (QS.has('touch')) return QS.get('touch') === '1';
  if (matchMedia('(pointer: coarse)').matches) return true;
  // iPad は「デスクトップ用サイト」表示で Mac を名乗ることがある（キーボード・トラックパッドを付けたときなど）
  return /Macintosh/.test(navigator.userAgent) && navigator.maxTouchPoints > 1;
};
// 指で操作する端末なら、縁の余白などのための目印を付ける
document.documentElement.classList.toggle('touch', coarse());
// スマートフォンの小さい画面（?compact=1 / ?compact=0 で強制）
function isCompact() {
  if (QS.has('compact')) return QS.get('compact') === '1';
  // スマートフォンも iPad などのタブレットも、指で操作する端末はタッチ向けの画面にする
  return coarse();
}
let compact = false;
function fit() {
  // スマートフォンでは描く画素数を抑えて電池と発熱を減らす
  const dpr = Math.min(window.devicePixelRatio || 1, coarse() ? 2 : 3);
  // 縁の余白を除いた、実際に描く領域の大きさ
  const w = Math.max(1, Math.round((uiCanvas.clientWidth || window.innerWidth) * dpr));
  const h = Math.max(1, Math.round((uiCanvas.clientHeight || window.innerHeight) * dpr));
  if (w === pw && h === ph) return false;
  pw = w;
  ph = h;
  glCanvas.width = uiCanvas.width = w;
  glCanvas.height = uiCanvas.height = h;
  curFont = -1;
  return true;
}

// ------------------------------------------------------------------ 入力（Win32 の仮想キーコードに変換）
const VK = {
  Backspace: 0x08, Tab: 0x09, Enter: 0x0d, NumpadEnter: 0x0d, ShiftLeft: 0x10, ShiftRight: 0x10, ControlLeft: 0x11, ControlRight: 0x11,
  MetaLeft: 0x11, MetaRight: 0x11, AltLeft: 0x12, AltRight: 0x12, Escape: 0x1b, Space: 0x20, PageUp: 0x21, PageDown: 0x22, End: 0x23, Home: 0x24,
  ArrowLeft: 0x25, ArrowUp: 0x26, ArrowRight: 0x27, ArrowDown: 0x28, Insert: 0x2d, Delete: 0x2e,
  NumpadAdd: 0x6b, NumpadSubtract: 0x6d, NumpadDecimal: 0x6e, Equal: 0xbb, Semicolon: 0xba, Minus: 0xbd, Period: 0xbe, Comma: 0xbc,
};
function vkOf(e) {
  const c = e.code;
  if (c.startsWith('Key') && c.length === 4) return c.charCodeAt(3);
  if (c.startsWith('Digit')) return 0x30 + Number(c[5]);
  if (c.startsWith('Numpad') && /\d$/.test(c)) return 0x60 + Number(c[6]);
  if (/^F\d+$/.test(c)) return 0x6f + Number(c.slice(1));
  // 配列の違い（JIS の ; + キーなど）は文字で判定
  if (e.key === '+' || (c === 'Semicolon' && e.key === ';' && e.shiftKey)) return 0xbb;
  return VK[c] ?? 0;
}
// ブラウザの既定動作を止めるキー
function shouldPrevent(e, vk) {
  if (vk >= 0x70 && vk <= 0x7b && vk !== 0x7b /* F12 */) return true;
  if ([0x08, 0x09, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28].includes(vk)) return true;
  if ((e.ctrlKey || e.metaKey) && (vk === 0xbb || vk === 0xbd || vk === 0x6b || vk === 0x6d || (vk >= 0x30 && vk <= 0x39))) return true;
  return false;
}

let pointerInside = true;
const held = new Set();
function bindInput() {
  const pos = (e) => {
    const r = uiCanvas.getBoundingClientRect();
    const x = ((e.clientX - r.left) / r.width) * pw;
    const y = ((e.clientY - r.top) / r.height) * ph;
    // 縁の余白に触れたら、いちばん近い画面の端に触れたことにする
    if (e.pointerType === 'touch') return [Math.min(Math.max(x, 1), pw - 1), Math.min(Math.max(y, 1), ph - 1)];
    return [x, y];
  };
  const btn = (b) => (b === 0 ? 0 : b === 2 ? 1 : b === 1 ? 2 : -1);
  uiCanvas.addEventListener('contextmenu', (e) => e.preventDefault());
  // ---- タッチ操作
  //  画面部品：タップ＝押す（押した直後は説明を少し表示）
  //  地図：タップ＝選択（自軍を選んでいれば移動・攻撃・採集などの命令）、なぞる＝地図を動かす、
  //        長押ししてなぞる＝範囲選択、2本指＝移動・拡大縮小
  //  一覧：なぞる＝上下にスクロール　ミニマップ：触れた場所へ移動
  const touches = new Map();
  let one = null; // 1本指の操作
  let two = null; // 2本指の操作
  let ignore = false; // 2本指のあと、全部の指が離れるまで無視
  const LONG_MS = 420;
  const slop = () => 10 * (pw / Math.max(1, uiCanvas.clientWidth || window.innerWidth));
  const mid = () => {
    const ps = [...touches.values()];
    return [(ps[0][0] + ps[1][0]) / 2, (ps[0][1] + ps[1][1]) / 2, Math.hypot(ps[0][0] - ps[1][0], ps[0][1] - ps[1][1])];
  };
  const tap = (x, y) => {
    wasm.yy_event(0, x, y, 0);
    wasm.yy_event(1, 0, x, y);
    wasm.yy_event(2, 0, x, y);
  };
  // 1本指の操作を終える（2本指になったときなど）
  const endOne = (x, y) => {
    if (!one) return;
    clearTimeout(one.timer);
    if (one.mode === 'pan') wasm.yy_event(2, 2, x, y);
    else if (one.mode === 'left' || one.mode === 'box') wasm.yy_event(2, 0, x, y);
    one = null;
  };
  function touchDown(e) {
    const p = pos(e);
    touches.set(e.pointerId, p);
    setTouch(true);
    if (ignore) return;
    if (touches.size === 1) {
      const [x, y] = p;
      const kind = wasm.yy_hit(x, y);
      one = { id: e.pointerId, x0: x, y0: y, x, y, kind, mode: 'pending' };
      wasm.yy_event(0, x, y, 0);
      if (kind === 2) {
        // ミニマップ：押した場所へすぐ動かす
        one.mode = 'left';
        wasm.yy_event(1, 0, x, y);
      } else if (kind === 0) {
        one.timer = setTimeout(() => {
          if (one && one.mode === 'pending') {
            // 長押し：範囲選択を始める
            one.mode = 'box';
            wasm.yy_event(1, 0, one.x0, one.y0);
            wasm.yy_event(0, one.x, one.y, 0);
            navigator.vibrate?.(15);
          }
        }, LONG_MS);
      }
    } else if (touches.size === 2) {
      if (one) {
        const [x, y] = touches.get(one.id) || [one.x, one.y];
        endOne(x, y);
      }
      const [mx, my, d] = mid();
      two = { d };
      wasm.yy_event(0, mx, my, 0);
      wasm.yy_event(1, 2, mx, my);
    }
  }
  function touchMove(e) {
    if (!touches.has(e.pointerId)) return;
    const p = pos(e);
    touches.set(e.pointerId, p);
    if (two && touches.size >= 2) {
      const [mx, my, d] = mid();
      wasm.yy_event(0, mx, my, 0);
      if (two.d > 0 && d > 0) {
        const z = Math.log(d / two.d) / Math.log(1.15);
        if (Math.abs(z) > 0.15) {
          wasm.yy_event(3, z, 0, 0);
          two.d = d;
        }
      }
      return;
    }
    if (!one || one.id !== e.pointerId) return;
    const [x, y] = p;
    const dy = y - one.y;
    one.x = x;
    one.y = y;
    const far = Math.hypot(x - one.x0, y - one.y0) > slop();
    if (one.mode === 'pending' && far) {
      clearTimeout(one.timer);
      if (one.kind === 0) {
        // なぞる：地図を動かす（中ボタンのドラッグとして送る）
        one.mode = 'pan';
        wasm.yy_event(0, one.x0, one.y0, 0);
        wasm.yy_event(1, 2, one.x0, one.y0);
      } else if (one.kind === 3) {
        one.mode = 'scroll';
        wasm.yy_event(7, y - one.y0, 0, 0);
        return;
      } else {
        one.mode = 'moved';
      }
    }
    if (one.mode === 'scroll') wasm.yy_event(7, dy, 0, 0);
    else if (one.mode !== 'moved') wasm.yy_event(0, x, y, 0);
  }
  function touchUp(e) {
    if (!touches.has(e.pointerId)) return;
    const [x, y] = touches.get(e.pointerId);
    touches.delete(e.pointerId);
    if (two) {
      if (touches.size < 2) {
        wasm.yy_event(2, 2, x, y);
        two = null;
        ignore = touches.size > 0;
      }
      return;
    }
    if (touches.size === 0) ignore = false;
    if (!one || one.id !== e.pointerId) return;
    clearTimeout(one.timer);
    if (one.mode === 'pending') tap(one.x0, one.y0);
    else if (one.mode === 'moved' && one.kind === 1 && Math.hypot(x - one.x0, y - one.y0) < slop() * 3) tap(one.x0, one.y0);
    else endOne(x, y);
    one = null;
  }
  // OS が触れた指を横取りした（画面端のジェスチャーなど）：タップにしない
  function touchCancel(e) {
    if (!touches.has(e.pointerId)) return;
    const [x, y] = touches.get(e.pointerId);
    touches.delete(e.pointerId);
    if (two) {
      if (touches.size < 2) {
        wasm.yy_event(2, 2, x, y);
        two = null;
        ignore = touches.size > 0;
      }
      return;
    }
    if (touches.size === 0) ignore = false;
    if (one && one.id === e.pointerId) {
      clearTimeout(one.timer);
      endOne(x, y);
      one = null;
    }
  }
  // タッチとマウスの切り替えを伝える（タッチでは画面端のスクロールや常時の説明表示をしない）
  let touchNow = null;
  function setTouch(t) {
    if (t || touchNow !== t) wasm.yy_event(9, t ? 1 : 0, 0, 0);
    touchNow = t;
  }

  uiCanvas.addEventListener('pointermove', (e) => {
    if (e.pointerType === 'touch') return touchMove(e);
    if (touchNow && e.pointerType === 'mouse') setTouch(false);
    const [x, y] = pos(e);
    wasm.yy_event(0, x, y, 0);
  });
  uiCanvas.addEventListener('pointercancel', (e) => {
    if (e.pointerType === 'touch') touchCancel(e);
  });
  uiCanvas.addEventListener('pointerdown', (e) => {
    if (e.pointerType === 'touch') {
      e.preventDefault();
      try { uiCanvas.setPointerCapture(e.pointerId); } catch {}
      return touchDown(e);
    }
    const b = btn(e.button);
    if (b < 0) return;
    e.preventDefault();
    setTouch(false);
    uiCanvas.focus();
    try { uiCanvas.setPointerCapture(e.pointerId); } catch {}
    const [x, y] = pos(e);
    wasm.yy_event(0, x, y, 0);
    wasm.yy_event(1, b, x, y);
  });
  uiCanvas.addEventListener('pointerup', (e) => {
    if (e.pointerType === 'touch') return touchUp(e);
    const b = btn(e.button);
    if (b < 0) return;
    const [x, y] = pos(e);
    wasm.yy_event(2, b, x, y);
  });
  // 縁の余白（キャンバスの外）で始まったタッチも受け付ける
  const inMargin = (e) => e.pointerType === 'touch' && (e.target === document.body || e.target === document.documentElement);
  window.addEventListener('pointerdown', (e) => {
    if (!inMargin(e)) return;
    e.preventDefault();
    touchDown(e);
  }, { passive: false });
  window.addEventListener('pointermove', (e) => inMargin(e) && touchMove(e));
  window.addEventListener('pointerup', (e) => inMargin(e) && touchUp(e));
  window.addEventListener('pointercancel', (e) => inMargin(e) && touchCancel(e));
  uiCanvas.addEventListener('pointerenter', () => (pointerInside = true));
  uiCanvas.addEventListener('pointerleave', () => (pointerInside = false));
  uiCanvas.addEventListener(
    'wheel',
    (e) => {
      e.preventDefault();
      let d = e.deltaMode === 1 ? -e.deltaY / 3 : e.deltaMode === 2 ? -e.deltaY : -e.deltaY / 100;
      d = Math.max(-3, Math.min(3, d));
      if (d !== 0) wasm.yy_event(3, d, 0, 0);
    },
    { passive: false },
  );
  window.addEventListener('keydown', (e) => {
    const vk = vkOf(e);
    if (!vk) return;
    if (shouldPrevent(e, vk)) e.preventDefault();
    held.add(vk);
    wasm.yy_event(4, vk, 1, 0);
    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey) wasm.yy_event(5, e.key.codePointAt(0), 0, 0);
  });
  window.addEventListener('keyup', (e) => {
    const vk = vkOf(e);
    if (!vk) return;
    held.delete(vk);
    wasm.yy_event(4, vk, 0, 0);
  });
  // フォーカスを失ったら押しっぱなしのキーを離す
  window.addEventListener('blur', () => {
    for (const vk of held) wasm.yy_event(4, vk, 0, 0);
    held.clear();
  });
  window.addEventListener('resize', checkSize);
  window.visualViewport?.addEventListener('resize', checkSize);
  screen.orientation?.addEventListener?.('change', checkSize);
}

// 画面の大きさが変わったら描く領域を合わせる。
// スマートフォンでは回転・全画面・縁の余白（safe-area）の変化がリサイズの通知より遅れて反映されることがあるので、
// 毎フレームも確かめる（表示と描画の縦横比がずれると画面がつぶれる）
function checkSize() {
  if (!wasm) return;
  const c = isCompact();
  if (c !== compact) {
    compact = c;
    wasm.yy_event(8, c ? 1 : 0, 0, 0);
  }
  if (fit()) wasm.yy_event(6, pw, ph, 0);
}

// ------------------------------------------------------------------ 起動
async function loadWasm() {
  const url = new URL('./yyempire.wasm', import.meta.url);
  const refresh = new URL(import.meta.url).searchParams.get('_yyrefresh');
  if (refresh) url.searchParams.set('_yyrefresh', refresh);
  const options = { cache: refresh ? 'reload' : 'no-cache' };
  const imports = { env };
  try {
    const r = await WebAssembly.instantiateStreaming(fetch(url, options), imports);
    return r.instance.exports;
  } catch {
    const buf = await (await fetch(url, options)).arrayBuffer();
    const r = await WebAssembly.instantiate(buf, imports);
    return r.instance.exports;
  }
}

async function main() {
  loadingMsg.textContent = T('読み込み中…', 'Loading…', '加载中…');
  // 日本語の書体を待つ（最大3秒）
  const fontWait = Promise.race([
    Promise.all([document.fonts.load(`400 14px ${FAMILY}`, '春秋'), document.fonts.load(`700 14px ${FAMILY}`, '春秋')]),
    new Promise((r) => setTimeout(r, 3000)),
  ]);
  const [exports] = await Promise.all([loadWasm(), fontWait, loadSlotInfo()]);
  wasm = exports;
  document.fonts.addEventListener?.('loadingdone', clearTextCaches);
  fit();
  loadingMsg.textContent = T('天下を生成中…', 'Creating the world…', '正在生成天下…');
  await new Promise((r) => requestAnimationFrame(() => setTimeout(r, 0)));
  compact = isCompact();
  wasm.yy_init(pw, ph, compact ? 1 : 0);
  bindInput();
  loading.classList.add('hidden');
  uiCanvas.focus();
  let last = performance.now();
  const frame = (now) => {
    if (window.yyBrowserRecovery?.reloading) {
      last = now;
      requestAnimationFrame(frame);
      return;
    }
    const dt = Math.min(0.1, Math.max(0, (now - last) / 1000));
    last = now;
    try {
      checkSize();
      wasm.yy_frame(dt, document.hasFocus() && pointerInside ? 1 : 0);
      window.yyBrowserRecovery?.setTitleScreen(wasm.yy_is_title() !== 0);
    } catch (e) {
      console.error(e);
      showError(String(e && e.stack ? e.stack : e));
      return;
    }
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

main().catch((e) => {
  console.error(e);
  showError(String(e && e.message ? e.message : e));
});

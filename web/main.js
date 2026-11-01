// 春秋戦国 — ブラウザ側のつなぎ目
// Rust（WebAssembly）の描画命令を WebGL2 と Canvas2D に、ブラウザの入力を Rust に渡す。

const glCanvas = document.getElementById('gl');
const uiCanvas = document.getElementById('ui');
const loading = document.getElementById('loading');
const loadingMsg = document.getElementById('loading-msg');

const FAMILY = '"Yu Gothic UI", "Yu Gothic", "Hiragino Kaku Gothic ProN", "Hiragino Sans", Meiryo, "Noto Sans JP", sans-serif';
// gfx::Font と同じ並び：[大きさ, 中央揃え, 太字]
const FONTS = [
  [12, false, false], // Small
  [14, false, false], // Normal
  [14, true, false], // NormalCenter
  [18, false, true], // Big
  [20, true, true], // BigCenter
  [40, true, true], // Title
  [15, true, true], // Label
];
const LINE = 1.33;
// この割合まではみ出す行は横に詰めて1行に収める
const SQUEEZE = 1.18;
const fontCss = FONTS.map(([sz, , bold]) => `${bold ? '700' : '400'} ${sz}px ${FAMILY}`);

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
if (!gl) throw new Error('WebGL2 に対応したブラウザが必要です');
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

// ------------------------------------------------------------------ Rust から呼ばれる関数
const env = {
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
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';
    ctx.globalAlpha = 1;
  },
  js_2d_end: () => {},
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

  js_fullscreen_toggle: () => {
    if (document.fullscreenElement) document.exitFullscreen?.();
    else document.documentElement.requestFullscreen?.().catch(() => {});
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
  e.textContent = 'エラーが発生しました：\n' + s;
  loadingMsg.appendChild(e);
}

// ------------------------------------------------------------------ 画面寸法
let pw = 0;
let ph = 0;
function fit() {
  const dpr = Math.min(window.devicePixelRatio || 1, 3);
  const w = Math.max(1, Math.round(window.innerWidth * dpr));
  const h = Math.max(1, Math.round(window.innerHeight * dpr));
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
  if (vk >= 0x70 && vk <= 0x7b && vk !== 0x74 /* F5 */ && vk !== 0x7b /* F12 */) return true;
  if ([0x08, 0x09, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28].includes(vk)) return true;
  if ((e.ctrlKey || e.metaKey) && (vk === 0xbb || vk === 0xbd || vk === 0x6b || vk === 0x6d || (vk >= 0x30 && vk <= 0x39))) return true;
  return false;
}

let pointerInside = true;
const held = new Set();
function bindInput() {
  const pos = (e) => {
    const r = uiCanvas.getBoundingClientRect();
    return [((e.clientX - r.left) / r.width) * pw, ((e.clientY - r.top) / r.height) * ph];
  };
  const btn = (b) => (b === 0 ? 0 : b === 2 ? 1 : b === 1 ? 2 : -1);
  uiCanvas.addEventListener('contextmenu', (e) => e.preventDefault());
  // ---- タッチ操作：タップ＝左クリック、ドラッグ＝範囲選択、長押し＝右クリック、2本指＝移動・拡大縮小
  const touches = new Map();
  let pending = null; // 1本指：押した直後（まだ左押下を送っていない）
  let twoFinger = null;
  const LONG_MS = 550;
  const mid = () => {
    const ps = [...touches.values()];
    return [(ps[0][0] + ps[1][0]) / 2, (ps[0][1] + ps[1][1]) / 2, Math.hypot(ps[0][0] - ps[1][0], ps[0][1] - ps[1][1])];
  };
  const flushPending = () => {
    if (!pending) return;
    clearTimeout(pending.timer);
    wasm.yy_event(0, pending.x, pending.y, 0);
    wasm.yy_event(1, 0, pending.x, pending.y);
    pending.sent = true;
  };
  function touchDown(e) {
    touches.set(e.pointerId, pos(e));
    if (touches.size === 1) {
      const [x, y] = pos(e);
      pending = { id: e.pointerId, x, y, sent: false, long: false };
      pending.timer = setTimeout(() => {
        if (pending && !pending.sent) {
          // 長押し＝右クリック（命令）
          pending.long = true;
          wasm.yy_event(0, pending.x, pending.y, 0);
          wasm.yy_event(1, 1, pending.x, pending.y);
          wasm.yy_event(2, 1, pending.x, pending.y);
          navigator.vibrate?.(20);
        }
      }, LONG_MS);
    } else if (touches.size === 2) {
      if (pending) {
        clearTimeout(pending.timer);
        if (pending.sent) wasm.yy_event(2, 0, pending.x, pending.y);
        pending = null;
      }
      const [mx, my, d] = mid();
      twoFinger = { d };
      wasm.yy_event(0, mx, my, 0);
      wasm.yy_event(1, 2, mx, my);
    }
  }
  function touchMove(e) {
    if (!touches.has(e.pointerId)) return;
    const p = pos(e);
    touches.set(e.pointerId, p);
    if (twoFinger && touches.size >= 2) {
      const [mx, my, d] = mid();
      wasm.yy_event(0, mx, my, 0);
      if (twoFinger.d > 0 && d > 0) {
        const z = Math.log(d / twoFinger.d) / Math.log(1.15);
        if (Math.abs(z) > 0.2) {
          wasm.yy_event(3, z, 0, 0);
          twoFinger.d = d;
        }
      }
    } else if (pending && pending.id === e.pointerId && !pending.long) {
      if (!pending.sent && Math.hypot(p[0] - pending.x, p[1] - pending.y) > 10 * (window.devicePixelRatio || 1)) flushPending();
      if (pending.sent) wasm.yy_event(0, p[0], p[1], 0);
    }
  }
  function touchUp(e) {
    if (!touches.has(e.pointerId)) return;
    const p = touches.get(e.pointerId);
    touches.delete(e.pointerId);
    if (twoFinger) {
      if (touches.size < 2) {
        const [x, y] = p;
        wasm.yy_event(2, 2, x, y);
        twoFinger = null;
      }
      return;
    }
    if (pending && pending.id === e.pointerId) {
      clearTimeout(pending.timer);
      if (!pending.long) {
        if (!pending.sent) flushPending();
        wasm.yy_event(0, p[0], p[1], 0);
        wasm.yy_event(2, 0, p[0], p[1]);
      }
      pending = null;
    }
  }

  uiCanvas.addEventListener('pointermove', (e) => {
    if (e.pointerType === 'touch') return touchMove(e);
    const [x, y] = pos(e);
    wasm.yy_event(0, x, y, 0);
  });
  uiCanvas.addEventListener('pointercancel', (e) => {
    if (e.pointerType === 'touch') touchUp(e);
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
  window.addEventListener('resize', () => {
    if (fit()) wasm.yy_event(6, pw, ph, 0);
  });
}

// ------------------------------------------------------------------ 起動
async function loadWasm() {
  const url = new URL('./yyempire.wasm', import.meta.url);
  const imports = { env };
  try {
    const r = await WebAssembly.instantiateStreaming(fetch(url), imports);
    return r.instance.exports;
  } catch {
    const buf = await (await fetch(url)).arrayBuffer();
    const r = await WebAssembly.instantiate(buf, imports);
    return r.instance.exports;
  }
}

async function main() {
  // 日本語の書体を待つ（最大3秒）
  const fontWait = Promise.race([
    Promise.all([document.fonts.load(`400 14px ${FAMILY}`, '春秋'), document.fonts.load(`700 14px ${FAMILY}`, '春秋')]),
    new Promise((r) => setTimeout(r, 3000)),
  ]);
  const [exports] = await Promise.all([loadWasm(), fontWait]);
  wasm = exports;
  document.fonts.addEventListener?.('loadingdone', clearTextCaches);
  fit();
  loadingMsg.textContent = '天下を生成中…';
  await new Promise((r) => requestAnimationFrame(() => setTimeout(r, 0)));
  wasm.yy_init(pw, ph);
  bindInput();
  loading.classList.add('hidden');
  uiCanvas.focus();
  let last = performance.now();
  const frame = (now) => {
    const dt = Math.min(0.1, Math.max(0, (now - last) / 1000));
    last = now;
    try {
      wasm.yy_frame(dt, document.hasFocus() && pointerInside ? 1 : 0);
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

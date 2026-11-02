//! 描画層（ブラウザ版）
//!
//! デスクトップ版の Direct3D11 / Direct2D・DirectWrite と同じ API を、
//! WebGL2（地形・ユニットの三角形バッチ）と Canvas2D（文字・UI・スプライト）で実装する。
//! 1フレームは「WebGL の世界描画 → Canvas2D の UI 描画」の順で、Canvas2D は WebGL の上に重なる。

#[cfg(target_arch = "wasm32")]
use crate::platform::js;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
    pub c: [f32; 4],
}

const MAX_VERTS: usize = 240_000;

/// WebGL テクスチャ（RGBA、非乗算アルファ）
pub struct Texture {
    pub id: u32,
    pub w: u32,
    pub h: u32,
}

/// Canvas2D 用の画像（BGRA 乗算済みアルファから作る）。Direct2D の ID2D1Bitmap に相当
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bitmap(pub u32);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Font {
    Small,
    Normal,
    NormalCenter,
    Big,
    BigCenter,
    Title,
    /// 題字（Title が扁額に収まらないとき）
    TitleMid,
}

#[derive(Debug)]
pub struct GfxError;
pub type Result<T> = std::result::Result<T, GfxError>;

pub struct Gfx {
    /// 実ピクセル寸法
    pw: u32,
    ph: u32,
    /// UI 座標系（実ピクセル / ui）での寸法。ゲーム側の画面座標はすべてこちら
    pub width: u32,
    pub height: u32,
    /// UI 拡大率（画面の高さに比例 × 利用者設定）
    pub ui: f32,
    pub user_scale: f32,
    pub verts: Vec<Vertex>,
    xform: [f32; 4],
    pub d2d_gen: u32,
    /// スマートフォン向けに詰めた画面（UI を約 1000×470 に収める）
    pub compact: bool,
}

pub fn col(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    [r, g, b, a]
}

impl Gfx {
    pub fn new(width: u32, height: u32) -> Gfx {
        let mut g = Gfx {
            pw: width,
            ph: height,
            width,
            height,
            ui: 1.0,
            user_scale: 1.15,
            verts: Vec::with_capacity(MAX_VERTS),
            xform: [1.0, 1.0, 0.0, 0.0],
            d2d_gen: 1,
            compact: false,
        };
        g.update_scale();
        g.reload_fonts();
        g
    }

    /// 言語が変わったら書体とページの題名を切り替える（日本語・中国語・英語で書体が違う）
    pub fn reload_fonts(&mut self) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_font_lang(crate::lang::lang() as u32)
        }
    }

    /// UI 拡大率を決め、UI 座標系での画面寸法を更新する。
    /// 画面の高さ 900px を基準に拡大し、UI が収まる最小寸法（1360×720）を下回らないよう制限する。
    pub fn update_scale(&mut self) {
        let (pw, ph) = (self.pw as f32, self.ph as f32);
        if self.compact {
            // タッチ端末：UI を約 1000×480 に収めて文字を大きくする（最小 900×420）。
            // スマートフォンは高さで、iPad などのタブレットは幅で大きさが決まる
            let cap = (ph / 420.0).min(pw / 900.0).max(0.3);
            self.ui = ((ph / 480.0).min(pw / 1000.0) * self.user_scale / 1.15).clamp(0.3, cap);
            self.width = ((pw / self.ui).round() as u32).max(1);
            self.height = ((ph / self.ui).round() as u32).max(1);
            return;
        }
        let cap = (ph / 720.0).min(pw / 1360.0).max(0.5);
        self.ui = (ph / 900.0 * self.user_scale).clamp(0.5, cap);
        self.width = ((pw / self.ui).round() as u32).max(1);
        self.height = ((ph / self.ui).round() as u32).max(1);
    }

    /// スマートフォン向けの詰めた画面に切り替える
    pub fn set_compact(&mut self, c: bool) {
        if self.compact != c {
            self.compact = c;
            self.update_scale();
        }
    }

    /// 文字・UI の描画を縦にずらす（一覧のスクロール用、UI 座標）
    pub fn set_2d_offset(&mut self, dy: f32) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_2d_offset(dy * self.ui)
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = dy;
    }

    /// 利用者の文字サイズ設定を変える（d: +1 で拡大、-1 で縮小）
    pub fn adjust_scale(&mut self, d: i32) {
        self.user_scale = (self.user_scale * 1.12f32.powi(d)).clamp(0.6, 2.5);
        self.update_scale();
    }

    /// 全画面表示の切り替え
    pub fn toggle_fullscreen(&mut self) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_fullscreen_toggle()
        }
    }

    pub fn is_fullscreen(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            return js::js_is_fullscreen() != 0;
        }
        #[allow(unreachable_code)]
        false
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 || (w == self.pw && h == self.ph) {
            return;
        }
        self.pw = w;
        self.ph = h;
        self.update_scale();
    }

    pub fn create_texture(&self, w: u32, h: u32, data: &[u32]) -> Result<Texture> {
        #[cfg(target_arch = "wasm32")]
        let id = unsafe { js::js_tex_create(w, h, data.as_ptr() as *const u8) };
        #[cfg(not(target_arch = "wasm32"))]
        let id = {
            let _ = data;
            1
        };
        Ok(Texture { id, w, h })
    }

    pub fn update_texture(&self, t: &Texture, x: u32, y: u32, w: u32, h: u32, data: &[u32]) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_tex_update(t.id, x, y, w, h, data.as_ptr() as *const u8)
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (t, x, y, w, h, data);
    }

    pub fn begin(&mut self, clear: [f32; 4]) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_gl_begin(clear[0], clear[1], clear[2], clear[3])
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = clear;
        self.verts.clear();
    }

    /// world -> NDC 変換: ndc = pos * scale + offset
    pub fn set_xform(&mut self, sx: f32, sy: f32, ox: f32, oy: f32) {
        self.flush();
        self.xform = [sx, sy, ox, oy];
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_gl_xform(sx, sy, ox, oy)
        }
    }

    pub fn set_screen_xform(&mut self) {
        let (w, h) = (self.width as f32, self.height as f32);
        self.set_xform(2.0 / w, -2.0 / h, -1.0, 1.0);
    }

    fn upload_draw(&mut self, tex: u32, linear: bool) {
        if self.verts.is_empty() {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_gl_draw(self.verts.as_ptr() as *const u8, self.verts.len() as u32, tex, linear as u32)
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (tex, linear);
        self.verts.clear();
    }

    /// 単色三角形バッチを描画
    pub fn flush(&mut self) {
        self.upload_draw(0, false);
    }

    pub fn draw_textured(&mut self, t: &Texture, x0: f32, y0: f32, x1: f32, y1: f32, c: [f32; 4], linear: bool) {
        self.flush();
        let v = |x, y, u, vv| Vertex { x, y, u, v: vv, c };
        self.verts.extend_from_slice(&[v(x0, y0, 0.0, 0.0), v(x1, y0, 1.0, 0.0), v(x1, y1, 1.0, 1.0), v(x0, y0, 0.0, 0.0), v(x1, y1, 1.0, 1.0), v(x0, y1, 0.0, 1.0)]);
        self.upload_draw(t.id, linear);
    }

    #[inline]
    pub fn tri(&mut self, a: (f32, f32), b: (f32, f32), c2: (f32, f32), c: [f32; 4]) {
        if self.verts.len() + 3 > MAX_VERTS * 4 {
            return;
        }
        self.verts.push(Vertex { x: a.0, y: a.1, u: 0.0, v: 0.0, c });
        self.verts.push(Vertex { x: b.0, y: b.1, u: 0.0, v: 0.0, c });
        self.verts.push(Vertex { x: c2.0, y: c2.1, u: 0.0, v: 0.0, c });
    }

    pub fn rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, c: [f32; 4]) {
        self.tri((x0, y0), (x1, y0), (x1, y1), c);
        self.tri((x0, y0), (x1, y1), (x0, y1), c);
    }

    pub fn rect_outline(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, c: [f32; 4]) {
        self.rect(x0, y0, x1, y0 + t, c);
        self.rect(x0, y1 - t, x1, y1, c);
        self.rect(x0, y0, x0 + t, y1, c);
        self.rect(x1 - t, y0, x1, y1, c);
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, seg: usize, c: [f32; 4]) {
        let step = std::f32::consts::TAU / seg as f32;
        for i in 0..seg {
            let a0 = i as f32 * step;
            let a1 = a0 + step;
            self.tri((cx, cy), (cx + a0.cos() * r, cy + a0.sin() * r), (cx + a1.cos() * r, cy + a1.sin() * r), c);
        }
    }

    pub fn ring(&mut self, cx: f32, cy: f32, r: f32, t: f32, seg: usize, c: [f32; 4]) {
        let step = std::f32::consts::TAU / seg as f32;
        let r2 = r - t;
        for i in 0..seg {
            let a0 = i as f32 * step;
            let a1 = a0 + step;
            let p0 = (cx + a0.cos() * r, cy + a0.sin() * r);
            let p1 = (cx + a1.cos() * r, cy + a1.sin() * r);
            let q0 = (cx + a0.cos() * r2, cy + a0.sin() * r2);
            let q1 = (cx + a1.cos() * r2, cy + a1.sin() * r2);
            self.tri(p0, p1, q1, c);
            self.tri(p0, q1, q0, c);
        }
    }

    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), t: f32, c: [f32; 4]) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt().max(1e-5);
        let (nx, ny) = (-dy / l * t * 0.5, dx / l * t * 0.5);
        self.tri((a.0 + nx, a.1 + ny), (b.0 + nx, b.1 + ny), (b.0 - nx, b.1 - ny), c);
        self.tri((a.0 + nx, a.1 + ny), (b.0 - nx, b.1 - ny), (a.0 - nx, a.1 - ny), c);
    }

    // ------------------------------------------------------------ Canvas2D（Direct2D 相当）
    pub fn d2d_begin(&mut self) -> bool {
        self.flush();
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_2d_begin(self.ui)
        }
        true
    }

    pub fn d2d_end(&mut self) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_2d_end()
        }
    }

    pub fn fill_rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, c: [f32; 4]) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_fill_rect(x0, y0, x1, y1, c[0], c[1], c[2], c[3])
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (x0, y0, x1, y1, c);
    }

    pub fn stroke_rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: [f32; 4]) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_stroke_rect(x0, y0, x1, y1, w, c[0], c[1], c[2], c[3])
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (x0, y0, x1, y1, w, c);
    }

    pub fn fill_round(&self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, c: [f32; 4]) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_fill_round(x0, y0, x1, y1, r, c[0], c[1], c[2], c[3])
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (x0, y0, x1, y1, r, c);
    }

    pub fn stroke_round(&self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, w: f32, c: [f32; 4]) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_stroke_round(x0, y0, x1, y1, r, w, c[0], c[1], c[2], c[3])
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (x0, y0, x1, y1, r, w, c);
    }

    pub fn fill_circle(&self, x: f32, y: f32, r: f32, c: [f32; 4]) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_fill_circle(x, y, r, c[0], c[1], c[2], c[3])
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (x, y, r, c);
    }

    pub fn stroke_circle(&self, x: f32, y: f32, r: f32, w: f32, c: [f32; 4]) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_stroke_circle(x, y, r, w, c[0], c[1], c[2], c[3])
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (x, y, r, w, c);
    }

    /// 矩形内に文字を描く（幅で折り返す。中央揃えの書体は上下左右中央）
    pub fn text(&mut self, s: &str, x: f32, y: f32, w: f32, h: f32, f: Font, c: [f32; 4]) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_text(s.as_ptr(), s.len(), x, y, w, h, f as u32, c[0], c[1], c[2], c[3])
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (s, x, y, w, h, f, c);
    }

    /// 影付き文字
    pub fn text_shadow(&mut self, s: &str, x: f32, y: f32, w: f32, h: f32, f: Font, c: [f32; 4]) {
        self.text(s, x + 1.0, y + 1.0, w, h, f, [0.0, 0.0, 0.0, c[3] * 0.85]);
        self.text(s, x, y, w, h, f, c);
    }

    /// テキスト幅の計測
    pub fn measure(&mut self, s: &str, f: Font) -> f32 {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            return js::js_measure(s.as_ptr(), s.len(), f as u32);
        }
        #[allow(unreachable_code)]
        {
            let _ = f;
            s.chars().count() as f32 * 14.0
        }
    }

    /// 幅 width で折り返したときの文章の高さ
    pub fn measure_h(&mut self, s: &str, f: Font, width: f32) -> f32 {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            return js::js_measure_h(s.as_ptr(), s.len(), f as u32, width);
        }
        #[allow(unreachable_code)]
        {
            let _ = (f, width);
            20.0 * (1 + s.chars().count() / 24) as f32
        }
    }

    /// BGRA 乗算済みアルファの画素から画像を作る
    pub fn create_bitmap(&self, w: u32, h: u32, data: &[u32]) -> Option<Bitmap> {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            return Some(Bitmap(js::js_bmp_create(w, h, data.as_ptr() as *const u8)));
        }
        #[allow(unreachable_code)]
        {
            let _ = (w, h, data);
            Some(Bitmap(0))
        }
    }

    pub fn update_bitmap(&self, bmp: &Bitmap, w: u32, data: &[u32]) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_bmp_update(bmp.0, w, data.as_ptr() as *const u8)
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (bmp, w, data);
    }

    pub fn draw_bitmap(&self, bmp: &Bitmap, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.draw_bitmap_alpha(bmp, x0, y0, x1, y1, 1.0);
    }

    pub fn draw_bitmap_alpha(&self, bmp: &Bitmap, x0: f32, y0: f32, x1: f32, y1: f32, alpha: f32) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js::js_bmp_draw(bmp.0, x0, y0, x1, y1, alpha)
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (bmp, x0, y0, x1, y1, alpha);
    }

    /// 画面の保存（デスクトップ版の自動撮影用。ブラウザ版では何もしない）
    pub fn save_backbuffer(&self, _path: &str) -> Result<()> {
        Ok(())
    }

    pub fn present(&mut self) {}
}

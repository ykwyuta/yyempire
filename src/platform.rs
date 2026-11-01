//! ブラウザ（JavaScript）とのつなぎ目。wasm32 以外（ネイティブのテスト）では代替実装を使う。

#[cfg(target_arch = "wasm32")]
pub mod js {
    #[link(wasm_import_module = "env")]
    extern "C" {
        pub fn js_now() -> f64;
        pub fn js_random() -> u32;
        pub fn js_log(ptr: *const u8, len: usize, level: u32);

        pub fn js_tex_create(w: u32, h: u32, ptr: *const u8) -> u32;
        pub fn js_tex_update(id: u32, x: u32, y: u32, w: u32, h: u32, ptr: *const u8);
        pub fn js_gl_begin(r: f32, g: f32, b: f32, a: f32);
        pub fn js_gl_xform(sx: f32, sy: f32, ox: f32, oy: f32);
        pub fn js_gl_draw(ptr: *const u8, nverts: u32, tex: u32, linear: u32);

        pub fn js_2d_begin(ui: f32);
        pub fn js_2d_end();
        pub fn js_fill_rect(x0: f32, y0: f32, x1: f32, y1: f32, r: f32, g: f32, b: f32, a: f32);
        pub fn js_stroke_rect(x0: f32, y0: f32, x1: f32, y1: f32, w: f32, r: f32, g: f32, b: f32, a: f32);
        pub fn js_fill_round(x0: f32, y0: f32, x1: f32, y1: f32, rad: f32, r: f32, g: f32, b: f32, a: f32);
        pub fn js_stroke_round(x0: f32, y0: f32, x1: f32, y1: f32, rad: f32, w: f32, r: f32, g: f32, b: f32, a: f32);
        pub fn js_fill_circle(x: f32, y: f32, rad: f32, r: f32, g: f32, b: f32, a: f32);
        pub fn js_stroke_circle(x: f32, y: f32, rad: f32, w: f32, r: f32, g: f32, b: f32, a: f32);
        pub fn js_text(ptr: *const u8, len: usize, x: f32, y: f32, w: f32, h: f32, font: u32, r: f32, g: f32, b: f32, a: f32);
        pub fn js_measure(ptr: *const u8, len: usize, font: u32) -> f32;
        pub fn js_measure_h(ptr: *const u8, len: usize, font: u32, width: f32) -> f32;
        pub fn js_bmp_create(w: u32, h: u32, ptr: *const u8) -> u32;
        pub fn js_bmp_update(id: u32, w: u32, ptr: *const u8);
        pub fn js_bmp_draw(id: u32, x0: f32, y0: f32, x1: f32, y1: f32, alpha: f32);

        pub fn js_fullscreen_toggle();
        pub fn js_is_fullscreen() -> u32;
    }
}

/// 経過時間（ミリ秒）
pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        js::js_now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::OnceLock;
        static T0: OnceLock<std::time::Instant> = OnceLock::new();
        T0.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
    }
}

/// 乱数の種（起動ごとに変わる）
pub fn random_u32() -> u32 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        js::js_random()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() ^ d.as_secs() as u32).unwrap_or(0)
    }
}

pub fn log(s: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        js::js_log(s.as_ptr(), s.len(), 0)
    }
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{s}");
}

/// パニック時にブラウザのコンソールへ出す
pub fn install_panic_hook() {
    #[cfg(target_arch = "wasm32")]
    std::panic::set_hook(Box::new(|info| {
        let s = info.to_string();
        unsafe { js::js_log(s.as_ptr(), s.len(), 2) }
    }));
}

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
        pub fn js_2d_offset(dy: f32);
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

        pub fn js_save(slot: u32, ptr: *const u8, len: usize, meta: *const u8, meta_len: usize);
        pub fn js_load(slot: u32);
        pub fn js_slot_info(slot: u32, out: *mut u8, cap: usize) -> usize;
        pub fn js_export(slot: u32);
        pub fn js_import();

        pub fn js_lang_get() -> u32;
        pub fn js_lang_save(lang: u32);
        pub fn js_font_lang(lang: u32);

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

// ------------------------------------------------------------ 記録（ブラウザの IndexedDB）

/// 記録画面に出す説明
pub fn storage_note() -> &'static str {
    t!(
        "記録はこのブラウザに保存される（サイトのデータを消すと失われる）。別の端末へ移すときや控えには、書き出したファイルを読み込む",
        "Saves are kept in this browser (clearing the site's data deletes them). To move them to another device or keep a backup, export a file and load it there.",
        "存档保存在此浏览器中（清除网站数据后会丢失）。如需转移到其他设备或备份，请导出文件后再读取。"
    )
}

// ------------------------------------------------------------ 表示言語

/// 起動時の表示言語：URL の ?lang=、保存した設定、ブラウザの言語の順（日本語・中国語・それ以外は英語）
pub fn initial_lang() -> crate::lang::Lang {
    use crate::lang::Lang;
    #[cfg(target_arch = "wasm32")]
    {
        return match unsafe { js::js_lang_get() } {
            0 => Lang::Ja,
            2 => Lang::Zh,
            _ => Lang::En,
        };
    }
    #[allow(unreachable_code)]
    Lang::Ja
}

/// 表示言語をブラウザに覚えさせる
pub fn save_lang(l: crate::lang::Lang) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        js::js_lang_save(l as u32)
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = l;
}

/// 記録を枠に保存する（非同期。結果はブラウザ側から通知される）
pub fn save_slot(slot: u8, data: &[u8], meta: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        js::js_save(slot as u32, data.as_ptr(), data.len(), meta.as_ptr(), meta.len())
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (slot, data, meta);
}

/// 枠の記録を読み込むよう頼む（読み込めたら yy_loaded が呼ばれる）
pub fn load_slot(slot: u8) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        js::js_load(slot as u32)
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = slot;
}

/// 枠の見出しと保存日時（空きなら None）
pub fn slot_info(slot: u8) -> Option<(String, String)> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = vec![0u8; 512];
        let n = unsafe { js::js_slot_info(slot as u32, buf.as_mut_ptr(), buf.len()) };
        if n == 0 || n > buf.len() {
            return None;
        }
        let s = String::from_utf8_lossy(&buf[..n]).to_string();
        let mut it = s.splitn(2, '\t');
        let meta = it.next().unwrap_or("").to_string();
        let date = it.next().unwrap_or("").to_string();
        return Some((meta, date));
    }
    #[allow(unreachable_code)]
    {
        let _ = slot;
        None
    }
}

/// 枠の記録をファイルとして書き出す
pub fn export_slot(slot: u8) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        js::js_export(slot as u32)
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = slot;
}

/// 記録ファイルを選んで読み込む
pub fn import_file() {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        js::js_import()
    }
}

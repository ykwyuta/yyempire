//! 春秋戦国 RTS — Rust → WebAssembly（ブラウザ版）
//!
//! デスクトップ版の Win32 メッセージループの代わりに、ブラウザ側（web/main.js）から
//! 入力イベントとフレーム更新を受け取る。描画は gfx.rs が WebGL2 / Canvas2D に橋渡しする。

#[macro_use]
mod lang;
mod ai;
mod art;
mod game;
mod geo;
mod gfx;
mod i18n;
mod path;
pub mod platform;
mod rng;
mod save;
mod world;

use std::cell::RefCell;

#[derive(Clone, Copy, Debug)]
pub enum Ev {
    MouseMove(f32, f32),
    Down(u8, f32, f32),
    Up(u8, f32, f32),
    Wheel(f32),
    Key(u32, bool),
    Char(char),
    Resize(u32, u32),
    /// スマートフォン向けの詰めた画面か
    Compact(bool),
    /// 続く入力がタッチ操作か
    Touch(bool),
    /// 画面（記録・外交などの一覧）を指でなぞったスクロール（実ピクセル）
    Scroll(f32),
}

thread_local! {
    static EVENTS: RefCell<Vec<Ev>> = const { RefCell::new(Vec::new()) };
    static GAME: RefCell<Option<game::Game>> = const { RefCell::new(None) };
    /// ブラウザから届いた記録（次のフレームで読み込む）
    static PENDING_LOAD: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
    static PENDING_NOTICE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn push(e: Ev) {
    EVENTS.with(|v| v.borrow_mut().push(e));
}

/// 初期化：実ピクセルでの画面寸法を受け取る
#[no_mangle]
pub extern "C" fn yy_init(w: u32, h: u32, compact: u32) {
    platform::install_panic_hook();
    lang::set_lang(platform::initial_lang());
    let mut gfx = gfx::Gfx::new(w.max(1), h.max(1));
    gfx.set_compact(compact != 0);
    let game = game::Game::new(gfx);
    GAME.with(|g| *g.borrow_mut() = Some(game));
}

/// 入力イベント
/// kind: 0=移動(x,y) 1=押下(b,x,y) 2=離す(b,x,y) 3=ホイール(d) 4=キー(vk,down) 5=文字(code) 6=リサイズ(w,h)
#[no_mangle]
pub extern "C" fn yy_event(kind: u32, a: f32, b: f32, c: f32) {
    let e = match kind {
        0 => Ev::MouseMove(a, b),
        1 => Ev::Down(a as u8, b, c),
        2 => Ev::Up(a as u8, b, c),
        3 => Ev::Wheel(a),
        4 => Ev::Key(a as u32, b != 0.0),
        5 => match char::from_u32(a as u32) {
            Some(ch) => Ev::Char(ch),
            None => return,
        },
        6 => Ev::Resize(a as u32, b as u32),
        7 => Ev::Scroll(a),
        8 => Ev::Compact(a != 0.0),
        9 => Ev::Touch(a != 0.0),
        _ => return,
    };
    push(e);
}

/// 1フレーム進めて描画する。終了要求なら 1
#[no_mangle]
pub extern "C" fn yy_frame(dt: f32, focused: u32) -> u32 {
    let evs: Vec<Ev> = EVENTS.with(|v| std::mem::take(&mut *v.borrow_mut()));
    let load = PENDING_LOAD.with(|p| p.borrow_mut().take());
    let notes: Vec<String> = PENDING_NOTICE.with(|p| std::mem::take(&mut *p.borrow_mut()));
    GAME.with(|g| match g.borrow_mut().as_mut() {
        Some(game) => {
            if let Some(data) = load {
                game.apply_loaded(&data);
            }
            for n in notes {
                game.notify(&n);
            }
            game.frame(&evs, dt.min(0.1), focused != 0) as u32
        }
        None => 0,
    })
}

/// Current screen for browser controls (independent of overlays and hit targets).
#[no_mangle]
pub extern "C" fn yy_is_title() -> u32 {
    GAME.with(|g| g.borrow().as_ref().map_or(1, |game| game.is_title_screen() as u32))
}

/// タッチした場所に何があるか（実ピクセル）。ブラウザ側が指の動きの解釈を決めるのに使う
/// 0=地図 1=画面部品 2=ミニマップ 3=なぞってスクロールできる一覧 4=タイトル画面の地図
#[no_mangle]
pub extern "C" fn yy_hit(x: f32, y: f32) -> u32 {
    GAME.with(|g| match g.borrow().as_ref() {
        Some(game) => game.hit_kind(x, y),
        None => 1,
    })
}

/// ブラウザが記録のデータを書き込むための領域を確保する
#[no_mangle]
pub extern "C" fn yy_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// 記録のデータが届いた（yy_alloc で確保した領域。所有権を受け取る）
#[no_mangle]
pub extern "C" fn yy_loaded(ptr: *mut u8, len: usize) {
    let data = unsafe { Vec::from_raw_parts(ptr, len, len.max(1)) };
    PENDING_LOAD.with(|p| *p.borrow_mut() = Some(data));
}

/// ブラウザ側からの通知（yy_alloc で確保した UTF-8 文字列）
#[no_mangle]
pub extern "C" fn yy_notice(ptr: *mut u8, len: usize) {
    let data = unsafe { Vec::from_raw_parts(ptr, len, len.max(1)) };
    let s = String::from_utf8_lossy(&data).to_string();
    PENDING_NOTICE.with(|p| p.borrow_mut().push(s));
}

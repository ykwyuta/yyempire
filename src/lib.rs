//! 春秋戦国 RTS — Rust → WebAssembly（ブラウザ版）
//!
//! デスクトップ版の Win32 メッセージループの代わりに、ブラウザ側（web/main.js）から
//! 入力イベントとフレーム更新を受け取る。描画は gfx.rs が WebGL2 / Canvas2D に橋渡しする。

mod ai;
mod art;
mod game;
mod geo;
mod gfx;
mod path;
pub mod platform;
mod rng;
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
}

thread_local! {
    static EVENTS: RefCell<Vec<Ev>> = const { RefCell::new(Vec::new()) };
    static GAME: RefCell<Option<game::Game>> = const { RefCell::new(None) };
}

fn push(e: Ev) {
    EVENTS.with(|v| v.borrow_mut().push(e));
}

/// 初期化：実ピクセルでの画面寸法を受け取る
#[no_mangle]
pub extern "C" fn yy_init(w: u32, h: u32) {
    platform::install_panic_hook();
    let gfx = gfx::Gfx::new(w.max(1), h.max(1));
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
        _ => return,
    };
    push(e);
}

/// 1フレーム進めて描画する。終了要求なら 1
#[no_mangle]
pub extern "C" fn yy_frame(dt: f32, focused: u32) -> u32 {
    let evs: Vec<Ev> = EVENTS.with(|v| std::mem::take(&mut *v.borrow_mut()));
    GAME.with(|g| match g.borrow_mut().as_mut() {
        Some(game) => game.frame(&evs, dt.min(0.1), focused != 0) as u32,
        None => 0,
    })
}

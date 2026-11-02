//! 画面・入力・HUD（独自実装の UI）

mod chronicle;
mod heroes;
mod market;
mod mobile;
mod save;
mod tutorial;

use crate::art::{paint_aoe_terrain_pixel, Art, CmdIcon, ResIcon};
use crate::geo::*;
use crate::gfx::{Font, Gfx, Texture};
use crate::rng::hash2;
use crate::world::*;
use crate::Ev;
use crate::gfx::Bitmap as ID2D1Bitmap;

const TEX_SCALE: usize = 8;
const OVERLAY_SCALE: usize = 4;
const TOP_H: f32 = 36.0;
const BOT_H: f32 = 176.0;
const MM_W: f32 = 196.0;
const MM_H: f32 = 168.0;

const VK_BACK: u32 = 0x08;
const VK_TAB: u32 = 0x09;
const VK_SHIFT: u32 = 0x10;
const VK_CONTROL: u32 = 0x11;
const VK_ESCAPE: u32 = 0x1B;
const VK_SPACE: u32 = 0x20;
const VK_HOME: u32 = 0x24;
const VK_LEFT: u32 = 0x25;
const VK_UP: u32 = 0x26;
const VK_RIGHT: u32 = 0x27;
const VK_DOWN: u32 = 0x28;
const VK_DELETE: u32 = 0x2E;
const VK_F1: u32 = 0x70;
const VK_F2: u32 = 0x71;
const VK_F3: u32 = 0x72;
const VK_F4: u32 = 0x73;
const VK_F5: u32 = 0x74;
/// 下の画面への操作を遮る透明なボタンの目印
const BLOCK_LABEL: &str = "#block";
const VK_F11: u32 = 0x7A;
const VK_ADD: u32 = 0x6B;
const VK_SUB: u32 = 0x6D;
const VK_OEM_PLUS: u32 = 0xBB;
const VK_OEM_MINUS: u32 = 0xBD;
const VK_OEM_PERIOD: u32 = 0xBE;

/// 費用表示用の短い資源名（英語は F/W/B）
fn res_short(k: usize) -> &'static str {
    match k {
        0 => t!("糧", "F", "粮"),
        1 => t!("木", "W", "木"),
        _ => t!("銅", "B", "铜"),
    }
}

/// 資源の名前（0:糧 1:木 2:銅）
fn res_name(k: usize) -> &'static str {
    match k {
        0 => t!("糧", "Food", "粮"),
        1 => t!("木", "Wood", "木"),
        _ => t!("銅", "Bronze", "铜"),
    }
}
const RES_COLORS: [[f32; 4]; 3] = [[0.95, 0.85, 0.35, 1.0], [0.55, 0.85, 0.45, 1.0], [0.95, 0.6, 0.35, 1.0]];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Title,
    Playing,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Place(BKind),
    AttackMove,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Act {
    Train(UnitKind),
    Place(BKind),
    Upgrade,
    Convert,
    Stop,
    AttackMove,
    SelectAllWorkers,
    SelectAllMilitary,
    SelectKind(UnitKind),
    FilterKind(UnitKind),
    WorkersAuto,
    AllWorkersAuto,
    AllWorkersWood,
    AllWorkersMine,
    AllWorkersUpgrade,
    ArmyDefend,
    AllArmyAttack,
    AllArmyDefend,
    AllArmyUpgrade,
    Speed(u32),
    Diplo,
    Help,
    War(usize),
    Peace(usize),
    /// 和睦の申し入れへの返答（true=受諾）
    PeaceAnswer(bool),
    Start,
    Home,
    Restart,
    CloseOverlay,
    Demolish,
    ResumeBuild,
    IdleWorker,
    Chron,
    ChronPage(i32),
    Market,
    Tax(u8),
    Buy(usize),
    Hire(usize),
    ToggleHeroic,
    Summon,
    SummonHero(usize),
    Tutorial,
    TutNext,
    TutQuit,
    Scale(i32),
    Fullscreen,
    /// 表示言語を切り替える
    Lang,
    SaveMenu,
    /// 選択・建設の取り消し（タッチ操作用の ✕）
    Cancel,
    SaveSlot(u8),
    LoadSlot(u8),
    ExportSlot(u8),
    ImportFile,
    Noop,
}

struct Button {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    label: String,
    key: u32,
    enabled: bool,
    act: Act,
    tip: String,
}

pub struct Game {
    gfx: Gfx,
    art: Art,
    world: World,
    seed: u32,
    terrain_px: Vec<u32>,
    terrain_tex: Texture,
    overlay_px: Vec<u32>,
    overlay_tex: Texture,
    overlay_ver: u32,
    minimap_px: Vec<u32>,
    minimap_bmp: Option<ID2D1Bitmap>,
    minimap_gen: u32,
    minimap_ver: u32,
    minimap_t: f32,
    minimap_fog: u32,
    /// 視界：0=未踏、1=探索済み（霧）、2=視界内
    fog: Vec<u8>,
    fog_px: Vec<u32>,
    fog_tex: Texture,
    fog_t: f32,
    fog_ver: u32,
    cam: V2,
    zoom: f32,
    mouse: (f32, f32),
    keys: [bool; 256],
    ldown: Option<(f32, f32)>,
    mdrag: Option<(f32, f32, V2)>,
    mm_drag: bool,
    screen: Screen,
    mode: Mode,
    sel: Vec<Handle>,
    sel_b: Option<Handle>,
    groups: Vec<Vec<Handle>>,
    speed: u32,
    paused: bool,
    /// 部隊を選んでから命令するまでの一時停止
    cmd_pause: bool,
    last_sel: Vec<Handle>,
    last_orders: u32,
    acc: f32,
    show_diplo: bool,
    show_help: bool,
    show_chron: bool,
    show_market: bool,
    show_summon: bool,
    /// 記録（セーブ・ロード）画面
    show_save: bool,
    /// いまの入力がタッチ操作か
    touch: bool,
    /// スマートフォン向け画面で一覧（外交・交易など）をなぞってスクロールした量（UI 座標）
    ov_scroll: f32,
    ov_key: u8,
    ov_max: f32,
    /// タッチ操作でツールチップを出しておく時刻
    tip_until: f32,
    /// タッチ操作の建設：1回目のタップで場所を示し、同じ場所を2回目にタップで建てる
    place_tap: Option<(f32, f32)>,
    /// 最後に記録した年（自動記録の間隔）
    autosave_years: f32,
    /// タイトル画面で選ぶ英霊モード（既定は無効）
    heroic_opt: bool,
    chron_scroll: usize,
    title_sel: Option<usize>,
    notice: Option<(String, f32)>,
    buttons: Vec<Button>,
    last_click: (f32, Option<Handle>),
    clock: f32,
    marker: Option<(V2, f32, bool)>,
    idle_cycle: usize,
    /// 自動撮影（YY_AUTO="国番号,秒,出力パス[,ズーム]"）
    auto: Option<(Option<usize>, f32, String, f32)>,
    script: Vec<(f32, Ev)>,
    tut: Option<tutorial::Tut>,
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn rgba(c: [f32; 3], a: f32) -> u32 {
    let r = (c[0].clamp(0.0, 1.0) * 255.0) as u32;
    let g = (c[1].clamp(0.0, 1.0) * 255.0) as u32;
    let b = (c[2].clamp(0.0, 1.0) * 255.0) as u32;
    let a = (a.clamp(0.0, 1.0) * 255.0) as u32;
    r | (g << 8) | (b << 16) | (a << 24)
}

fn bgra(c: [f32; 3]) -> u32 {
    let r = (c[0].clamp(0.0, 1.0) * 255.0) as u32;
    let g = (c[1].clamp(0.0, 1.0) * 255.0) as u32;
    let b = (c[2].clamp(0.0, 1.0) * 255.0) as u32;
    b | (g << 8) | (r << 16) | (255 << 24)
}

fn c4(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

fn darker(c: [f32; 3], k: f32) -> [f32; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

/// タイルの基本色
fn base_color(t: Terrain, x: i32, y: i32, shade: f32) -> [f32; 3] {
    let lat_t = (y as f32 / MAP_H as f32).clamp(0.0, 1.0);
    match t {
        Terrain::Sea => lerp3([0.10, 0.22, 0.42], [0.14, 0.30, 0.52], shade),
        Terrain::Lake => [0.20, 0.38, 0.58],
        Terrain::River => [0.24, 0.44, 0.66],
        Terrain::Plain => {
            let north = [0.70, 0.64, 0.42];
            let south = [0.40, 0.58, 0.30];
            let c = lerp3(north, south, ((lat_t - 0.30) * 2.2).clamp(0.0, 1.0));
            lerp3(c, darker(c, 0.85), shade)
        }
        Terrain::Steppe => lerp3([0.74, 0.70, 0.48], [0.66, 0.64, 0.42], shade),
        Terrain::Forest => lerp3([0.16, 0.36, 0.17], [0.22, 0.42, 0.20], shade),
        Terrain::Hill => lerp3([0.58, 0.50, 0.34], [0.50, 0.44, 0.30], shade),
        Terrain::Mountain => {
            let h = hash2(x, y, 99);
            if h > 0.93 {
                [0.78, 0.76, 0.74]
            } else {
                lerp3([0.42, 0.38, 0.35], [0.52, 0.48, 0.44], h)
            }
        }
    }
}

impl Game {
    pub fn new(gfx: Gfx) -> Game {
        let seed = 20240 + crate::platform::random_u32() % 1000;
        let world = World::new(seed);
        let tw = (MAP_W * TEX_SCALE) as u32;
        let th = (MAP_H * TEX_SCALE) as u32;
        let ow = (MAP_W * OVERLAY_SCALE) as u32;
        let oh = (MAP_H * OVERLAY_SCALE) as u32;
        let terrain_px = vec![0u32; (tw * th) as usize];
        let overlay_px = vec![0u32; (ow * oh) as usize];
        let terrain_tex = gfx.create_texture(tw, th, &terrain_px).expect("texture");
        let overlay_tex = gfx.create_texture(ow, oh, &overlay_px).expect("texture");
        let fog_px = vec![0u32; MAP_W * MAP_H];
        let fog_tex = gfx.create_texture(MAP_W as u32, MAP_H as u32, &fog_px).expect("texture");
        let mut g = Game {
            gfx,
            art: Art::new(),
            world,
            seed,
            terrain_px,
            terrain_tex,
            overlay_px,
            overlay_tex,
            overlay_ver: u32::MAX,
            minimap_px: vec![0; MAP_W * MAP_H],
            minimap_bmp: None,
            minimap_gen: 0,
            minimap_ver: u32::MAX,
            minimap_t: 0.0,
            minimap_fog: 0,
            fog: vec![2; MAP_W * MAP_H],
            fog_px,
            fog_tex,
            fog_t: 0.0,
            fog_ver: 0,
            cam: v2(MAP_W as f32 / 2.0, MAP_H as f32 / 2.0),
            zoom: 4.0,
            mouse: (-100.0, -100.0),
            keys: [false; 256],
            ldown: None,
            cmd_pause: false,
            last_sel: Vec::new(),
            last_orders: 0,
            mdrag: None,
            mm_drag: false,
            screen: Screen::Title,
            mode: Mode::Normal,
            sel: Vec::new(),
            sel_b: None,
            groups: vec![Vec::new(); 10],
            speed: 1,
            paused: false,
            acc: 0.0,
            show_diplo: false,
            show_help: false,
            show_chron: false,
            show_market: false,
            show_summon: false,
            show_save: false,
            touch: false,
            ov_scroll: 0.0,
            ov_key: 0,
            ov_max: 0.0,
            tip_until: 0.0,
            place_tap: None,
            autosave_years: 0.0,
            heroic_opt: false,
            chron_scroll: 0,
            title_sel: None,
            notice: None,
            buttons: Vec::new(),
            last_click: (-10.0, None),
            clock: 0.0,
            marker: None,
            idle_cycle: 0,
            script: Vec::new(),
            tut: None,
            auto: std::env::var("YY_AUTO").ok().and_then(|v| {
                let p: Vec<&str> = v.split(',').collect();
                Some((p.first()?.parse().ok(), p.get(1)?.parse().ok()?, p.get(2)?.to_string(), p.get(3).and_then(|z| z.parse().ok()).unwrap_or(22.0)))
            }),
        };
        g.rebuild_terrain();
        g.fit_title_camera();
        if std::env::var("YY_FULL").is_ok() {
            g.gfx.toggle_fullscreen();
        }
        g
    }

    fn fit_title_camera(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let avail_w = (w - 380.0).max(300.0);
        self.zoom = (avail_w / MAP_W as f32).min(h / MAP_H as f32) * 0.98;
        self.cam = v2(MAP_W as f32 / 2.0 + 380.0 / 2.0 / self.zoom, MAP_H as f32 / 2.0);
    }

    fn rebuild_terrain(&mut self) {
        let tw = MAP_W * TEX_SCALE;
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                self.paint_tile(x, y, tw);
            }
        }
        let t = &self.terrain_tex;
        self.gfx.update_texture(t, 0, 0, t.w, t.h, &self.terrain_px);
        self.overlay_ver = u32::MAX;
        self.minimap_ver = u32::MAX;
    }

    fn paint_tile(&mut self, x: i32, y: i32, tw: usize) {
        let s = TEX_SCALE as i32;
        for py in 0..s {
            for px in 0..s {
                let gx = x * s + px;
                let gy = y * s + py;
                let c = paint_aoe_terrain_pixel(&self.world.map.terrain, &self.world.map.shade, x, y, px, py, s);
                self.terrain_px[gy as usize * tw + gx as usize] = rgba(c, 1.0);
            }
        }
    }

    fn update_dirty_terrain(&mut self) {
        if self.world.terrain_dirty.is_empty() {
            return;
        }
        let tw = MAP_W * TEX_SCALE;
        let dirty = std::mem::take(&mut self.world.terrain_dirty);
        for &(x, y) in dirty.iter() {
            self.paint_tile(x, y, tw);
            let s = TEX_SCALE;
            let mut buf = vec![0u32; s * s];
            for py in 0..s {
                for px in 0..s {
                    buf[py * s + px] = self.terrain_px[(y as usize * s + py) * tw + x as usize * s + px];
                }
            }
            self.gfx.update_texture(&self.terrain_tex, (x as usize * s) as u32, (y as usize * s) as u32, s as u32, s as u32, &buf);
        }
        self.minimap_ver = u32::MAX;
    }

    fn update_overlay(&mut self) {
        if self.overlay_ver == self.world.territory_version {
            return;
        }
        self.overlay_ver = self.world.territory_version;
        let w = &self.world;
        let os = OVERLAY_SCALE;
        let ow = MAP_W * os;
        let owner_at = |nx: i32, ny: i32| -> u8 {
            let cx = nx.clamp(0, MAP_W as i32 - 1);
            let cy = ny.clamp(0, MAP_H as i32 - 1);
            let ni = Map::idx(cx, cy);
            if w.map.terrain[ni] == Terrain::Sea {
                NONE
            } else {
                w.owner[ni]
            }
        };
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                let o = owner_at(x, y);
                for sy in 0..os {
                    for sx in 0..os {
                        let fx = x as f32 + (sx as f32 + 0.5) / os as f32 - 0.5;
                        let fy = y as f32 + (sy as f32 + 0.5) / os as f32 - 0.5;
                        let x0 = fx.floor() as i32;
                        let y0 = fy.floor() as i32;
                        let tx = fx - x0 as f32;
                        let ty = fy - y0 as f32;
                        let ux = tx * tx * (3.0 - 2.0 * tx);
                        let uy = ty * ty * (3.0 - 2.0 * ty);

                        let o00 = owner_at(x0, y0);
                        let o10 = owner_at(x0 + 1, y0);
                        let o01 = owner_at(x0, y0 + 1);
                        let o11 = owner_at(x0 + 1, y0 + 1);

                        // 最も近い有効な領主を決定
                        let dom = if o != NONE {
                            o
                        } else if o00 != NONE {
                            o00
                        } else if o10 != NONE {
                            o10
                        } else if o01 != NONE {
                            o01
                        } else {
                            o11
                        };
                        if dom == NONE {
                            self.overlay_px[(y as usize * os + sy) * ow + x as usize * os + sx] = 0;
                            continue;
                        }
                        let w00 = if o00 == dom { 1.0 } else { 0.0 };
                        let w10 = if o10 == dom { 1.0 } else { 0.0 };
                        let w01 = if o01 == dom { 1.0 } else { 0.0 };
                        let w11 = if o11 == dom { 1.0 } else { 0.0 };
                        let wt = (w00 * (1.0 - ux) + w10 * ux) * (1.0 - uy) + (w01 * (1.0 - ux) + w11 * ux) * uy;

                        let c = w.nations[dom as usize].color;
                        let px = if wt < 0.36 {
                            0
                        } else if wt < 0.60 {
                            rgba([c[0].min(1.0), c[1].min(1.0), c[2].min(1.0)], 0.86)
                        } else if wt < 0.80 {
                            rgba(c, 0.30)
                        } else {
                            rgba(c, 0.065)
                        };
                        self.overlay_px[(y as usize * os + sy) * ow + x as usize * os + sx] = px;
                    }
                }
            }
        }
        self.gfx.update_texture(&self.overlay_tex, 0, 0, ow as u32, (MAP_H * os) as u32, &self.overlay_px);
    }

    fn update_minimap(&mut self, dt: f32) {
        self.minimap_t -= dt;
        let gen_changed = self.minimap_gen != self.gfx.d2d_gen;
        let unchanged = self.minimap_ver == self.world.territory_version && self.minimap_fog == self.fog_ver;
        if !gen_changed && (self.minimap_t > 0.0 || unchanged) {
            return;
        }
        self.minimap_t = 1.0;
        self.minimap_ver = self.world.territory_version;
        self.minimap_fog = self.fog_ver;
        let fogged = self.fog_active();
        let w = &self.world;
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                let i = Map::idx(x, y);
                let mut c = base_color(w.map.terrain[i], x, y, w.map.shade[i]);
                let o = w.owner[i];
                if o != NONE {
                    c = lerp3(c, w.nations[o as usize].color, 0.55);
                }
                if fogged {
                    match self.fog[i] {
                        0 => c = [0.06, 0.06, 0.07],
                        1 => c = darker(c, 0.55),
                        _ => {}
                    }
                }
                self.minimap_px[i] = bgra(c);
            }
        }
        if gen_changed || self.minimap_bmp.is_none() {
            self.minimap_bmp = self.gfx.create_bitmap(MAP_W as u32, MAP_H as u32, &self.minimap_px);
            self.minimap_gen = self.gfx.d2d_gen;
        } else if let Some(b) = &self.minimap_bmp {
            self.gfx.update_bitmap(b, MAP_W as u32, &self.minimap_px);
        }
    }

    // ------------------------------------------------------------ 視界（戦場の霧）
    /// 霧を表示・適用するか（タイトル画面と終局後は全域を見せる）
    fn fog_active(&self) -> bool {
        self.screen == Screen::Playing && self.world.game_over.is_none()
    }

    fn reset_fog(&mut self) {
        self.fog.fill(0);
        // 諸侯の都の所在は天下に知られている
        let homes: Vec<V2> = self.world.nations.iter().filter(|n| n.alive).map(|n| n.home).collect();
        for h in homes {
            let (x, y) = h.tile();
            Self::reveal(&mut self.fog, x, y, 3, 1);
        }
        self.fog_t = 0.0;
        self.update_fog(0.0);
    }

    fn reveal(fog: &mut [u8], cx: i32, cy: i32, r: i32, level: u8) {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r + r {
                    continue;
                }
                let (x, y) = (cx + dx, cy + dy);
                if Map::inb(x, y) {
                    let f = &mut fog[Map::idx(x, y)];
                    *f = (*f).max(level);
                }
            }
        }
    }

    fn update_fog(&mut self, dt: f32) {
        self.fog_t -= dt;
        if self.fog_t > 0.0 {
            return;
        }
        self.fog_t = 0.15;
        for f in self.fog.iter_mut() {
            if *f == 2 {
                *f = 1;
            }
        }
        let pl = self.world.player;
        for u in self.world.units.iter() {
            if u.alive && u.nation as usize == pl {
                let (x, y) = u.pos.tile();
                Self::reveal(&mut self.fog, x, y, u.kind.vision(), 2);
            }
        }
        for b in self.world.blds.iter() {
            if b.alive && b.nation as usize == pl {
                let (x, y) = b.center().tile();
                let r = if b.kind.city() { b.kind.radius() as i32 + 3 } else { 5 };
                Self::reveal(&mut self.fog, x, y, r, 2);
            }
        }
        let fog_c = [0.04, 0.05, 0.07];
        for (px, &f) in self.fog_px.iter_mut().zip(self.fog.iter()) {
            *px = match f {
                0 => rgba(fog_c, 1.0),
                1 => rgba(fog_c, 0.5),
                _ => 0,
            };
        }
        self.gfx.update_texture(&self.fog_tex, 0, 0, MAP_W as u32, MAP_H as u32, &self.fog_px);
        self.fog_ver = self.fog_ver.wrapping_add(1);
    }

    /// その地点が今見えているか
    fn seen(&self, p: V2) -> bool {
        if !self.fog_active() {
            return true;
        }
        let (x, y) = p.tile();
        Map::inb(x, y) && self.fog[Map::idx(x, y)] == 2
    }

    /// その地点を一度でも見たか
    fn explored(&self, p: V2) -> bool {
        if !self.fog_active() {
            return true;
        }
        let (x, y) = p.tile();
        Map::inb(x, y) && self.fog[Map::idx(x, y)] >= 1
    }

    // ------------------------------------------------------------ 座標
    fn screen_to_world(&self, sx: f32, sy: f32) -> V2 {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        v2(self.cam.x + (sx - w / 2.0) / self.zoom, self.cam.y + (sy - h / 2.0) / self.zoom)
    }
    fn world_to_screen(&self, p: V2) -> (f32, f32) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        ((p.x - self.cam.x) * self.zoom + w / 2.0, (p.y - self.cam.y) * self.zoom + h / 2.0)
    }
    fn clamp_cam(&mut self) {
        self.cam.x = self.cam.x.clamp(0.0, MAP_W as f32);
        self.cam.y = self.cam.y.clamp(0.0, MAP_H as f32);
    }
    fn mm_rect(&self) -> (f32, f32, f32, f32) {
        let h = self.gfx.height as f32;
        (2.0, h - MM_H - 2.0, 2.0 + MM_W, h - 2.0)
    }
    /// 上部バーの高さ（スマートフォンでは指で押しやすいよう高くする）
    fn top_h(&self) -> f32 {
        if self.gfx.compact {
            mobile::TOP_H_COMPACT
        } else {
            TOP_H
        }
    }
    fn over_ui(&self, x: f32, y: f32) -> bool {
        if self.show_save {
            return true;
        }
        if self.screen == Screen::Title {
            return x > self.gfx.width as f32 - 380.0;
        }
        if let Some(t) = &self.tut {
            let (x0, y0, x1, y1) = t.panel;
            if x >= x0 && x <= x1 && y >= y0 && y <= y1 {
                return true;
            }
        }
        if self.show_diplo || self.show_help || self.show_chron || self.show_market || self.show_summon || self.show_save || self.world.game_over.is_some() {
            return true;
        }
        if self.peace_pending() {
            return true;
        }
        y < self.top_h() || y > self.gfx.height as f32 - BOT_H
    }

    fn notice(&mut self, s: &str) {
        if !s.is_empty() {
            self.notice = Some((s.to_string(), 3.0));
        }
    }

    // ------------------------------------------------------------ 指揮中の一時停止
    /// 自国の部隊を新たに選ぶと時間を止め、命令を出すと再開する
    fn update_cmd_pause(&mut self) {
        let pl = self.world.player;
        let own = self.sel.iter().any(|&h| self.world.unit(h).map_or(false, |u| u.nation as usize == pl));
        if self.sel != self.last_sel {
            // 戦死などで減っただけなら選び直しとはみなさない
            let added = self.sel.iter().any(|h| !self.last_sel.contains(h));
            if added {
                self.cmd_pause = own;
            }
            self.last_sel = self.sel.clone();
        }
        if !own {
            self.cmd_pause = false;
        }
        if self.world.player_orders != self.last_orders {
            self.last_orders = self.world.player_orders;
            self.cmd_pause = false;
        }
    }

    /// プレイヤーの返答を待つ和睦の申し入れがあるか（返答するまで時間を止める）
    fn peace_pending(&self) -> bool {
        self.screen == Screen::Playing && self.world.game_over.is_none() && !self.world.peace_asks.is_empty()
    }

    /// 時間を止めているか（部隊の選択中・範囲選択のドラッグ中）
    fn cmd_hold(&self) -> bool {
        let dragging = self.ldown.map_or(false, |(sx, sy)| (sx - self.mouse.0).abs() > 4.0 || (sy - self.mouse.1).abs() > 4.0);
        self.cmd_pause || dragging
    }

    // ------------------------------------------------------------ フレーム
    /// true を返すと終了
    pub fn frame(&mut self, evs: &[Ev], dt: f32, focused: bool) -> bool {
        self.clock += dt;
        // OS からのマウス座標（実ピクセル）を UI 座標系へ
        let ui = self.gfx.ui;
        let mut evs: Vec<Ev> = evs
            .iter()
            .map(|e| match *e {
                Ev::MouseMove(x, y) => Ev::MouseMove(x / ui, y / ui),
                Ev::Down(b, x, y) => Ev::Down(b, x / ui, y / ui),
                Ev::Up(b, x, y) => Ev::Up(b, x / ui, y / ui),
                o => o,
            })
            .collect();
        while !self.script.is_empty() && self.script[0].0 <= self.clock {
            evs.push(self.script.remove(0).1);
        }
        for e in evs {
            if self.handle_event(e) {
                return true;
            }
        }
        if self.screen == Screen::Playing {
            self.scroll(dt, focused);
            self.update_cmd_pause();
            self.world.prune_peace_asks();
            if !self.paused && !self.cmd_hold() && self.world.game_over.is_none() && !self.show_help && !self.peace_pending() {
                self.acc += dt * self.speed as f32;
                let mut steps = 0;
                while self.acc >= DT && steps < 16 {
                    self.world.step();
                    self.acc -= DT;
                    steps += 1;
                    // 和睦の申し入れが届いたら、返答するまで時間を止める
                    if self.peace_pending() {
                        self.acc = 0.0;
                        break;
                    }
                }
                if steps >= 16 {
                    self.acc = 0.0;
                }
            }
            self.autosave_tick();
            self.update_fog(dt);
            self.update_tutorial(dt);
            let pl = self.world.player;
            let sel: Vec<Handle> = self.sel.iter().copied().filter(|&h| self.world.unit(h).map_or(false, |u| u.nation as usize == pl || self.seen(u.pos))).collect();
            self.sel = sel;
            if let Some(b) = self.sel_b {
                if self.world.bld(b).is_none() {
                    self.sel_b = None;
                }
            }
        }
        if let Some((_, t)) = &mut self.notice {
            *t -= dt;
            if *t <= 0.0 {
                self.notice = None;
            }
        }
        if let Some(m) = &mut self.marker {
            m.1 -= dt;
            if m.1 <= 0.0 {
                self.marker = None;
            }
        }
        if let Some((nation, secs, _, z)) = self.auto.clone() {
            if self.screen == Screen::Title {
                if let Some(step) = std::env::var("YY_TUT").ok().and_then(|v| v.parse::<usize>().ok()) {
                    self.start_tutorial();
                    self.tut_enter(step);
                    self.zoom = z;
                    if let Some(k) = std::env::var("YY_PRESS").ok().and_then(|v| u32::from_str_radix(&v, 16).ok()) {
                        self.script = vec![(self.clock + 0.5, Ev::Key(k, true)), (self.clock + 0.55, Ev::Key(k, false))];
                    }
                }
            }
            if self.screen == Screen::Title {
                if nation.is_none() {
                    // タイトル画面のまま国を選んでおく（YY_SEL=国番号）
                    if let Some(s) = std::env::var("YY_SEL").ok().and_then(|v| v.parse().ok()) {
                        self.title_sel = Some(s);
                    }
                    if std::env::var("YY_HEROIC").is_ok() {
                        self.heroic_opt = true;
                    }
                }
                if let Some(n) = nation {
                    self.title_sel = Some(n);
                    self.do_act(Act::Start);
                    // 撮影用：英霊モード（YY_HEROIC=1）
                    if std::env::var("YY_HEROIC").is_ok() {
                        self.world.heroic = true;
                    }
                    // 撮影用：自国も AI に任せる（YY_AIPLAY=1）
                    self.world.ai_player = std::env::var("YY_AIPLAY").is_ok();
                    // 撮影用：指定の年（紀元前）から始める（YY_YEAR=222 など）
                    if let Some(y) = std::env::var("YY_YEAR").ok().and_then(|v| v.parse::<f32>().ok()) {
                        self.world.years = START_BC - y;
                    }
                    // 撮影用：指定の国と交戦させ、和睦を申し入れさせる（YY_PEACE=国番号）
                    if let Some(o) = std::env::var("YY_PEACE").ok().and_then(|v| v.parse::<usize>().ok()) {
                        let pl = self.world.player;
                        self.world.declare_war(pl, o, true);
                        self.world.war_start[pl][o] -= 12.0;
                        self.world.propose_peace(o, pl, 8.0, t!("連合の解散", "the coalition disbanded", "联军解散"));
                    }
                    // 撮影用：民を1人選び、都の近くに田を3つ予約させる（YY_QUEUE=1）
                    if std::env::var("YY_QUEUE").is_ok() {
                        let pl = self.world.player;
                        let home = self.world.nations[pl].home;
                        let wk = (0..self.world.units.len()).find(|&i| self.world.units[i].alive && self.world.units[i].nation as usize == pl && self.world.units[i].kind == UnitKind::Worker);
                        if let Some(i) = wk {
                            let h = self.world.uh(i);
                            let mut spots: Vec<(i32, i32)> = Vec::new();
                            for r in 4..10 {
                                for dy in -r..=r {
                                    for dx in -r..=r {
                                        let (tx, ty) = (home.x as i32 + dx, home.y as i32 + dy);
                                        if spots.len() < 3 && self.world.can_place(pl, BKind::Farm, tx, ty).is_ok() && !spots.iter().any(|&(x, y)| (x - tx).abs() < 4 && (y - ty).abs() < 4) {
                                            spots.push((tx, ty));
                                        }
                                    }
                                }
                            }
                            for (tx, ty) in spots {
                                if let Ok(b) = self.world.place_building(pl, BKind::Farm, tx, ty) {
                                    self.world.queue_build(&[h], b);
                                }
                            }
                            self.set_sel(vec![h]);
                        }
                    }
                    self.speed = std::env::var("YY_SPEED").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
                    self.zoom = z;
                    // 撮影直前にキーを押す（例：72 で史記を開く）
                    // YY_PRESS は「47,48」のように複数のキーを0.5秒おきに押せる
                    if let Ok(v) = std::env::var("YY_PRESS") {
                        let at = std::env::var("YY_PRESS_AT").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(secs - 0.6);
                        let t = at.max(self.clock + 0.3);
                        for (i, k) in v.split(',').filter_map(|s| u32::from_str_radix(s.trim(), 16).ok()).enumerate() {
                            let tk = t + i as f32 * 0.5;
                            self.script.push((tk, Ev::Key(k, true)));
                            self.script.push((tk + 0.05, Ev::Key(k, false)));
                        }
                    }
                    if std::env::var("YY_DEMO").is_ok() {
                        self.speed = 1;
                        let (cx, cy) = (self.gfx.width as f32 / 2.0, self.gfx.height as f32 / 2.0);
                        let c = self.clock;
                        self.script = vec![
                            (c + 0.3, Ev::MouseMove(cx, cy)),
                            (c + 0.4, Ev::Down(0, cx, cy)),
                            (c + 0.45, Ev::Up(0, cx, cy)),
                            (c + 0.6, Ev::Key(0x51, true)),
                            (c + 0.65, Ev::Key(0x51, false)),
                            (c + 0.9, Ev::Down(0, cx - 160.0, cy - 160.0)),
                            (c + 0.95, Ev::MouseMove(cx + 160.0, cy + 160.0)),
                            (c + 1.0, Ev::Up(0, cx + 160.0, cy + 160.0)),
                            (c + 1.2, Ev::Key(0x51, true)),
                            (c + 1.25, Ev::Key(0x51, false)),
                            (c + 1.4, Ev::MouseMove(cx + 140.0, cy + 80.0)),
                            (c + 1.6, Ev::Down(0, cx + 140.0, cy + 80.0)),
                            (c + 1.65, Ev::Up(0, cx + 140.0, cy + 80.0)),
                            (c + 1.8, Ev::Key(0x57, true)),
                            (c + 1.85, Ev::Key(0x57, false)),
                            (c + 2.0, Ev::MouseMove(cx - 150.0, cy + 60.0)),
                            (c + 2.4, Ev::Key(0x71, true)),
                            (c + 2.45, Ev::Key(0x71, false)),
                        ];
                    }
                }
            }
            if let Some(n) = nation {
                let _ = n;
                // 自国の兵に追従
                if self.clock > 1.0 {
                    let pl = self.world.player;
                    if let Some(u) = self.world.units.iter().find(|u| u.alive && u.nation as usize == pl && u.kind.military()) {
                        let _ = u;
                    }
                }
            }
            if self.clock >= secs {
                // 撮影時のカーソル位置（YY_MOUSE="x,y"、画面ピクセル）：ツールチップの確認用
                if let Some((mx, my)) = std::env::var("YY_MOUSE").ok().and_then(|v| {
                    let mut it = v.split(',').filter_map(|s| s.trim().parse::<f32>().ok());
                    Some((it.next()?, it.next()?))
                }) {
                    self.mouse = (mx / self.gfx.ui, my / self.gfx.ui);
                }
                self.update_dirty_terrain();
                self.update_overlay();
                self.render_nopresent(dt);
                let path = self.auto.as_ref().unwrap().2.clone();
                let _ = self.gfx.save_backbuffer(&path);
                return true;
            }
        }
        self.update_dirty_terrain();
        self.update_overlay();
        self.render(dt);
        false
    }

    fn scroll(&mut self, dt: f32, focused: bool) {
        let sp = 900.0 / self.zoom * dt;
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let (mx, my) = self.mouse;
        let mut d = v2(0.0, 0.0);
        if self.keys[VK_LEFT as usize] {
            d.x -= 1.0;
        }
        if self.keys[VK_RIGHT as usize] {
            d.x += 1.0;
        }
        if self.keys[VK_UP as usize] {
            d.y -= 1.0;
        }
        if self.keys[VK_DOWN as usize] {
            d.y += 1.0;
        }
        if focused && !self.touch && self.mdrag.is_none() && self.ldown.is_none() {
            if mx <= 3.0 && mx >= 0.0 {
                d.x -= 1.0;
            }
            if mx >= w - 4.0 && mx <= w {
                d.x += 1.0;
            }
            if my <= 3.0 && my >= 0.0 {
                d.y -= 1.0;
            }
            if my >= h - 4.0 && my <= h {
                d.y += 1.0;
            }
        }
        self.cam = self.cam + d * sp;
        self.clamp_cam();
    }

    fn handle_event(&mut self, e: Ev) -> bool {
        match e {
            Ev::Resize(w, h) => {
                self.gfx.resize(w, h);
                self.clamp_cam();
                if self.screen == Screen::Title {
                    self.fit_title_camera();
                }
            }
            Ev::MouseMove(x, y) => {
                self.mouse = (x, y);
                if let Some((sx, sy, c0)) = self.mdrag {
                    self.cam = c0 - v2(x - sx, y - sy) * (1.0 / self.zoom);
                    self.clamp_cam();
                }
                if self.mm_drag {
                    self.minimap_jump(x, y);
                }
            }
            Ev::Compact(c) => {
                self.gfx.set_compact(c);
                self.clamp_cam();
                if self.screen == Screen::Title {
                    self.fit_title_camera();
                }
            }
            Ev::Touch(t) => {
                self.touch = t;
                if t {
                    self.tip_until = self.clock + 2.5;
                }
            }
            Ev::Scroll(dy) => {
                self.ov_scroll = (self.ov_scroll - dy / self.gfx.ui).clamp(0.0, self.ov_max);
            }
            Ev::Wheel(d) if self.ov_max > 0.0 && self.overlay_open() => {
                self.ov_scroll = (self.ov_scroll - d * 60.0).clamp(0.0, self.ov_max);
            }
            Ev::Wheel(d) => {
                if self.screen == Screen::Playing && !self.over_ui(self.mouse.0, self.mouse.1) {
                    let before = self.screen_to_world(self.mouse.0, self.mouse.1);
                    self.zoom = (self.zoom * 1.15f32.powf(d)).clamp(1.2, 64.0);
                    let after = self.screen_to_world(self.mouse.0, self.mouse.1);
                    self.cam = self.cam + (before - after);
                    self.clamp_cam();
                }
            }
            Ev::Down(b, x, y) => {
                self.mouse = (x, y);
                match b {
                    0 => {
                        if let Some(a) = self.button_at(x, y) {
                            self.do_act(a);
                            return false;
                        }
                        if self.screen == Screen::Title {
                            self.title_click(x, y);
                            return false;
                        }
                        let (mx0, my0, mx1, my1) = self.mm_rect();
                        if x >= mx0 && x <= mx1 && y >= my0 && y <= my1 && !self.show_diplo && !self.show_help && !self.show_chron && !self.show_market && !self.show_summon && !self.show_save {
                            self.mm_drag = true;
                            self.minimap_jump(x, y);
                            return false;
                        }
                        if self.over_ui(x, y) {
                            return false;
                        }
                        match self.mode {
                            Mode::Place(_) if self.touch && self.place_tap.map_or(true, |(px, py)| (px - x).abs() > 22.0 || (py - y).abs() > 22.0) => {
                                // タッチ：1回目のタップは置き場所を見せるだけ
                                self.place_tap = Some((x, y));
                                self.notice(t!("同じ場所をもう一度タップすると建てる", "Tap the same spot again to build", "再次点击同一位置即可建造"));
                            }
                            Mode::Place(k) => {
                                self.place_tap = None;
                                // Ctrl を押しながら置くと建設の予約（続けて置ける）
                                let queue = self.keys[VK_CONTROL as usize];
                                self.try_place(k, queue);
                                if !self.keys[VK_SHIFT as usize] && !queue {
                                    self.mode = Mode::Normal;
                                }
                            }
                            Mode::AttackMove => {
                                let p = self.screen_to_world(x, y);
                                let hs = self.own_sel();
                                self.world.order_move(&hs, p, true);
                                self.marker = Some((p, 0.6, true));
                                self.mode = Mode::Normal;
                                if let Some(t) = &mut self.tut {
                                    t.moved = true;
                                }
                            }
                            Mode::Normal => self.ldown = Some((x, y)),
                        }
                    }
                    1 => {
                        if self.mode != Mode::Normal {
                            self.mode = Mode::Normal;
                            return false;
                        }
                        if self.screen != Screen::Playing {
                            return false;
                        }
                        let (mx0, my0, mx1, my1) = self.mm_rect();
                        if x >= mx0 && x <= mx1 && y >= my0 && y <= my1 && !self.show_diplo {
                            let p = v2((x - mx0) / MM_W * MAP_W as f32, (y - my0) / MM_H * MAP_H as f32);
                            self.right_click_world(p);
                            return false;
                        }
                        if !self.over_ui(x, y) {
                            let p = self.screen_to_world(x, y);
                            self.right_click_world(p);
                        }
                    }
                    _ => {
                        if self.screen == Screen::Playing {
                            self.mdrag = Some((x, y, self.cam));
                        }
                    }
                }
            }
            Ev::Up(b, x, y) => {
                self.mouse = (x, y);
                match b {
                    0 => {
                        self.mm_drag = false;
                        if let Some((sx, sy)) = self.ldown.take() {
                            // タッチ：自国の部隊を選んでいるとき、自国の兵・民以外をタップすれば命令（右クリックと同じ）
                            let tap = (sx - x).abs() < 12.0 && (sy - y).abs() < 12.0;
                            if self.touch && tap && self.screen == Screen::Playing && !self.own_sel().is_empty() {
                                let wp = self.screen_to_world(x, y);
                                let own_unit = self.unit_at(wp).and_then(|h| self.world.unit(h)).map_or(false, |u| u.nation as usize == self.world.player);
                                if !own_unit {
                                    self.right_click_world(wp);
                                    return false;
                                }
                            }
                            self.select_release(sx, sy, x, y);
                        }
                    }
                    2 => self.mdrag = None,
                    _ => {}
                }
            }
            Ev::Key(k, down) => {
                if (k as usize) < 256 {
                    self.keys[k as usize] = down;
                }
                if down {
                    return self.key_down(k);
                }
            }
            Ev::Char(_) => {}
        }
        false
    }

    fn minimap_jump(&mut self, x: f32, y: f32) {
        let (mx0, my0, _, _) = self.mm_rect();
        self.cam = v2(((x - mx0) / MM_W * MAP_W as f32).clamp(0.0, MAP_W as f32), ((y - my0) / MM_H * MAP_H as f32).clamp(0.0, MAP_H as f32));
    }

    fn key_down(&mut self, k: u32) -> bool {
        let ctrl = self.keys[VK_CONTROL as usize];
        match k {
            VK_F5 => {
                self.do_act(Act::SaveMenu);
                return false;
            }
            VK_F11 => {
                self.do_act(Act::Fullscreen);
                return false;
            }
            VK_ADD | VK_OEM_PLUS if ctrl => {
                self.do_act(Act::Scale(1));
                return false;
            }
            VK_SUB | VK_OEM_MINUS if ctrl => {
                self.do_act(Act::Scale(-1));
                return false;
            }
            _ => {}
        }
        if self.screen == Screen::Title {
            if k == VK_ESCAPE {
                if self.show_save {
                    self.show_save = false;
                    return false;
                }
                return true;
            }
            if k == VK_SPACE || k == 0x0D {
                if self.title_sel.is_some() {
                    self.do_act(Act::Start);
                }
            }
            return false;
        }
        // ボタンのホットキー
        if !ctrl {
            let mut hit = None;
            for b in self.buttons.iter() {
                if b.key == k && b.enabled {
                    hit = Some(b.act);
                    break;
                }
            }
            if let Some(a) = hit {
                self.do_act(a);
                return false;
            }
        }
        match k {
            VK_ESCAPE => {
                if self.mode != Mode::Normal {
                    self.mode = Mode::Normal;
                } else if self.show_diplo || self.show_help || self.show_chron || self.show_market || self.show_summon || self.show_save {
                    self.show_summon = false;
                    self.show_diplo = false;
                    self.show_help = false;
                    self.show_chron = false;
                    self.show_market = false;
                    self.show_save = false;
                } else {
                    self.clear_sel();
                }
            }
            VK_SPACE => self.paused = !self.paused,
            VK_F1 => self.show_help = !self.show_help,
            VK_F2 => self.show_diplo = !self.show_diplo,
            VK_F3 => self.do_act(Act::Chron),
            VK_F4 => self.do_act(Act::Market),
            VK_HOME => self.do_act(Act::Home),
            VK_ADD | VK_OEM_PLUS => self.speed = (self.speed * 2).min(8),
            VK_SUB | VK_OEM_MINUS => self.speed = (self.speed / 2).max(1),
            VK_OEM_PERIOD => self.do_act(Act::IdleWorker),
            VK_DELETE => self.do_act(Act::Demolish),
            VK_TAB | VK_BACK => {}
            0x30..=0x39 => {
                let g = (k - 0x30) as usize;
                if ctrl {
                    self.groups[g] = self.own_sel();
                    self.notice(&tf!("部隊{}に登録", "Saved as group {}", "编为第{}队", g));
                } else {
                    let hs: Vec<Handle> = self.groups[g].iter().copied().filter(|&h| self.world.unit(h).is_some()).collect();
                    if !hs.is_empty() {
                        let double = self.last_click.0 > self.clock - 0.4 && self.last_click.1 == Some(hs[0]);
                        self.set_sel(hs.clone());
                        if double {
                            let p = self.world.unit(hs[0]).unwrap().pos;
                            self.cam = p;
                        }
                        self.last_click = (self.clock, Some(hs[0]));
                    }
                }
            }
            _ => {}
        }
        false
    }

    fn clear_sel(&mut self) {
        for h in self.sel.drain(..) {
            if let Some(u) = self.world.unit_mut(h) {
                u.selected = false;
            }
        }
        self.sel_b = None;
    }

    fn set_sel(&mut self, hs: Vec<Handle>) {
        self.clear_sel();
        for &h in hs.iter() {
            if let Some(u) = self.world.unit_mut(h) {
                u.selected = true;
            }
        }
        self.sel = hs;
    }

    fn own_sel(&self) -> Vec<Handle> {
        let p = self.world.player;
        self.sel.iter().copied().filter(|&h| self.world.unit(h).map_or(false, |u| u.nation as usize == p)).collect()
    }

    fn all_workers(&self) -> Vec<Handle> {
        let p = self.world.player;
        (0..self.world.units.len())
            .filter(|&i| {
                let u = &self.world.units[i];
                u.alive && u.nation as usize == p && u.kind == UnitKind::Worker
            })
            .map(|i| self.world.uh(i))
            .collect()
    }

    fn all_military(&self) -> Vec<Handle> {
        let p = self.world.player;
        (0..self.world.units.len())
            .filter(|&i| {
                let u = &self.world.units[i];
                u.alive && u.nation as usize == p && u.kind.military()
            })
            .map(|i| self.world.uh(i))
            .collect()
    }

    fn upgrade_handles(&mut self, mut hs: Vec<Handle>) {
        let mut err = None;
        let mut ok = 0;
        hs.sort_by(|a, b| self.world.unit(*a).unwrap().power.partial_cmp(&self.world.unit(*b).unwrap().power).unwrap());
        for h in hs {
            match self.world.upgrade_unit(h) {
                Ok(()) => ok += 1,
                Err(e) => err = Some(e),
            }
        }
        if ok == 0 {
            if let Some(e) = err {
                self.notice(e);
            }
        } else {
            self.notice(&tf!("{}部隊の能力を高めた", "Raised the power of {} units", "提升了{}个单位的能力", ok));
        }
    }

    /// 民（選択民または全民）を空いている田・森・銅山へバランス配分する
    fn order_workers_balanced(&mut self, hs: &[Handle], skip_builders: bool) {
        let pl = self.world.player;
        let mut gathering = [0usize; 3];
        for u in self.world.units.iter() {
            if u.alive && u.nation as usize == pl && u.kind == UnitKind::Worker && !hs.iter().any(|&h| self.world.unit(h).map_or(false, |x| std::ptr::eq(x, u))) {
                if let Order::Gather { src, .. } = u.order {
                    gathering[src.res()] += 1;
                }
            }
        }
        let mut assigned = 0;
        let mut built = 0;
        let res = self.world.nations[pl].res;
        let mut want = [0.44f32, 0.33, 0.23];
        if self.world.nations[pl].upkeep > self.world.nations[pl].income[0] * 0.55 {
            want[0] += 0.18;
        }
        for k in 0..3 {
            if res[k] > 700.0 {
                want[k] *= 0.45;
            }
        }
        let sumw: f32 = want.iter().sum();

        for &h in hs {
            let Some(u) = self.world.unit(h) else { continue };
            if u.kind != UnitKind::Worker {
                continue;
            }
            if skip_builders && matches!(u.order, Order::Build(_)) {
                continue;
            }
            if let Order::Build(bh) = u.order {
                if self.world.bld(bh).map_or(false, |b| !b.done()) && self.world.bld_builders(bh) <= 1 {
                    continue;
                }
            }
            let p = u.pos;
            // 作業者のいない未完成の建物があれば最優先で建設を再開させる
            if let Some(bh) = self.world.unfinished_bld(pl, p) {
                self.world.order_build(&[h], bh);
                built += 1;
                assigned += 1;
                continue;
            }
            let total = (gathering[0] + gathering[1] + gathering[2] + 1) as f32;
            let mut best_k = 0;
            let mut best_def = f32::MIN;
            for k in 0..3 {
                let def = want[k] / sumw * total - gathering[k] as f32;
                if def > best_def {
                    best_def = def;
                    best_k = k;
                }
            }
            let src = match best_k {
                0 => self.world.free_farm(pl, p).map(Src::Farm),
                2 => self.world.nearest_mine(p, 28.0).map(Src::Mine),
                _ => self.world.nearest_forest(p, 22).map(|(x, y)| Src::Forest(x, y)),
            }
            .or_else(|| self.world.free_farm(pl, p).map(Src::Farm))
            .or_else(|| self.world.nearest_forest(p, 28).map(|(x, y)| Src::Forest(x, y)))
            .or_else(|| self.world.nearest_mine(p, 36.0).map(Src::Mine));
            if let Some(s) = src {
                gathering[s.res()] += 1;
                self.world.order_gather(&[h], s);
                assigned += 1;
            }
        }
        if assigned > 0 {
            if built > 0 {
                self.notice(&tf!("未完成の建物へ民{}体を派遣し、計{}体を自動配分した", "Sent {} workers to unfinished buildings; {} assigned in all", "派{}名民前往未完工建筑，共自动分配{}名", built, assigned));
            } else {
                self.notice(&tf!("民{}体を田・森・銅山へ自動配分した", "Assigned {} workers to fields, forests and mines", "已将{}名民分配到田、森林、铜矿", assigned));
            }
        } else {
            self.notice(t!("配分できる民や資源が近くにない", "No workers or resources nearby to assign", "附近没有可分配的民或资源"));
        }
    }

    /// 指定した未完成建物へ、近くの民を1体派遣して建設を再開・加速させる
    fn dispatch_worker_to_build(&mut self, bh: Handle) {
        let pl = self.world.player;
        let Some(b) = self.world.bld(bh) else { return };
        if b.nation as usize != pl || b.done() {
            return;
        }
        let c = b.center();
        let bname = b.kind.name();
        let mut best: Option<(f32, Handle)> = None;
        for (i, u) in self.world.units.iter().enumerate() {
            if !u.alive || u.nation as usize != pl || u.kind != UnitKind::Worker {
                continue;
            }
            if u.order == Order::Build(bh) {
                continue;
            }
            let pen = match u.order {
                Order::Idle => 0.0,
                Order::Build(_) => 100.0,
                _ => 8.0,
            };
            let score = u.pos.dist(c) + pen;
            if best.map_or(true, |bs| score < bs.0) {
                best = Some((score, self.world.uh(i)));
            }
        }
        if let Some((_, wh)) = best {
            self.world.order_build(&[wh], bh);
            self.marker = Some((c, 0.6, false));
            self.notice(&tf!("民を{}の建設へ派遣した", "Sent workers to build the {}", "已派民去建造{}", bname));
        } else if self.world.bld_builders(bh) > 0 {
            self.notice(&tf!("すでに民が{}を建設中", "Workers are already building the {}", "已有民在建造{}", bname));
        } else {
            self.notice(t!("派遣できる民がいない", "No workers to send", "没有可派遣的民"));
        }
    }

    /// 民に一斉伐採（最寄りの森）または一斉採掘（最寄りの銅山）を命じる
    fn order_workers_resource(&mut self, hs: &[Handle], wood: bool) {
        let mut assigned = 0;
        let home = self.world.nations[self.world.player].home;
        for &h in hs {
            let Some(u) = self.world.unit(h) else { continue };
            if u.kind != UnitKind::Worker || matches!(u.order, Order::Build(_)) {
                continue;
            }
            let p = u.pos;
            let src = if wood {
                self.world
                    .nearest_forest(p, 28)
                    .or_else(|| self.world.nearest_forest(home, 36))
                    .map(|(x, y)| Src::Forest(x, y))
            } else {
                self.world
                    .nearest_mine(p, 35.0)
                    .or_else(|| self.world.nearest_mine(home, 45.0))
                    .map(Src::Mine)
            };
            if let Some(s) = src {
                self.world.order_gather(&[h], s);
                assigned += 1;
            }
        }
        if assigned > 0 {
            let kind_str = if wood { t!("森の伐採", "cut wood", "伐木") } else { t!("銅山の採掘", "mine bronze", "采铜") };
            self.notice(&tf!("民{}体に{}を命じた", "Ordered {} workers to {}", "命令{}名民{}", assigned, kind_str));
        } else {
            self.notice(if wood { t!("近くに伐採できる森がない", "No forest nearby to cut", "附近没有可砍伐的森林") } else { t!("近くに採掘できる銅山がない", "No mine nearby", "附近没有可开采的铜矿") });
        }
    }

    /// 軍事ユニットに領内防衛・侵入者迎撃・城邑集結を命じる
    fn order_army_defend(&mut self, hs: &[Handle]) {
        if hs.is_empty() {
            self.notice(t!("軍事ユニットがいない", "No soldiers", "没有军事单位"));
            return;
        }
        let pl = self.world.player;
        let home = self.world.nations[pl].home;
        let mut intruder: Option<V2> = None;
        for u in self.world.units.iter() {
            if !u.alive || !self.world.at_war(pl, u.nation as usize) {
                continue;
            }
            let (x, y) = u.pos.tile();
            if Map::inb(x, y) && self.world.owner[Map::idx(x, y)] as usize == pl {
                if intruder.map_or(true, |p| p.dist(home) > u.pos.dist(home)) {
                    intruder = Some(u.pos);
                }
            }
        }
        if let Some(p) = intruder {
            self.world.order_move(hs, p, true);
            self.marker = Some((p, 0.6, true));
            self.notice(&tf!("{}部隊に領内侵入者の迎撃を命じた", "Ordered {} units to intercept intruders", "命令{}个单位迎击入侵者", hs.len()));
            if let Some(t) = &mut self.tut {
                t.moved = true;
            }
            return;
        }
        let cities: Vec<V2> = self
            .world
            .blds
            .iter()
            .filter(|b| b.alive && b.nation as usize == pl && b.kind.city() && b.done())
            .map(|b| b.center())
            .collect();
        let rally = if cities.is_empty() {
            home
        } else {
            // 各部隊から最も近い自国城邑へ帰還・防衛配置
            for (k, &h) in hs.iter().enumerate() {
                if let Some(u) = self.world.unit(h) {
                    let up = u.pos;
                    let c = cities.iter().copied().min_by(|a, b| a.dist(up).partial_cmp(&b.dist(up)).unwrap()).unwrap_or(home);
                    let ang = (k as f32) * 1.1;
                    let goal = c + v2(ang.cos() * 2.2, ang.sin() * 2.2);
                    self.world.order_move(&[h], goal, true);
                }
            }
            cities[0]
        };
        self.marker = Some((rally, 0.6, false));
        self.notice(&tf!("{}部隊に城邑の防衛集結を命じた", "Ordered {} units to gather and defend the cities", "命令{}个单位集结防守城邑", hs.len()));
        if let Some(t) = &mut self.tut {
            t.moved = true;
        }
    }

    /// 全軍に総進撃（交戦国の最寄り城邑・敵兵への攻撃移動＋進軍モード）を命じる
    fn order_all_army_attack(&mut self) {
        let hs = self.all_military();
        if hs.is_empty() {
            self.notice(t!("軍事ユニットがいない", "No soldiers", "没有军事单位"));
            return;
        }
        self.set_sel(hs.clone());
        let pl = self.world.player;
        let home = self.world.nations[pl].home;
        // 交戦中の敵城邑または敵部隊を探す
        let mut best_target: Option<(f32, V2)> = None;
        for b in self.world.blds.iter() {
            if b.alive && self.world.at_war(pl, b.nation as usize) && b.kind.city() {
                let d = b.center().dist(home);
                if best_target.map_or(true, |x| d < x.0) {
                    best_target = Some((d, b.center()));
                }
            }
        }
        if best_target.is_none() {
            for u in self.world.units.iter() {
                if u.alive && self.world.at_war(pl, u.nation as usize) {
                    let d = u.pos.dist(home);
                    if best_target.map_or(true, |x| d < x.0) {
                        best_target = Some((d, u.pos));
                    }
                }
            }
        }
        if let Some((_, goal)) = best_target {
            self.world.order_move(&hs, goal, true);
            self.marker = Some((goal, 0.6, true));
            self.mode = Mode::AttackMove;
            self.notice(&tf!("全軍{}部隊に敵陣への総進撃を命じた（地図クリックで目標変更可）", "All {} units ordered to advance on the enemy (click the map to change the target)", "全军{}个单位向敌阵总攻（点击地图可改变目标）", hs.len()));
            if let Some(t) = &mut self.tut {
                t.moved = true;
            }
        } else {
            self.mode = Mode::AttackMove;
            self.notice(&tf!("全軍{}部隊を選択：進軍先を地図上でクリックしてください", "All {} units selected: click the map to choose where to march", "已选全军{}个单位：请在地图上点击进军目标", hs.len()));
        }
    }

    fn unit_at(&self, p: V2) -> Option<Handle> {
        let mut best: Option<(f32, Handle)> = None;
        for (i, u) in self.world.units.iter().enumerate() {
            if !u.alive {
                continue;
            }
            if u.nation as usize != self.world.player && !self.seen(u.pos) {
                continue;
            }
            let r: f32 = if u.kind == UnitKind::Worker { 0.45 } else { 0.55 };
            // タッチは指の太さぶん広く拾う（画面上で約 18 単位）
            let r = if self.touch { r.max(18.0 / self.zoom) } else { r };
            let d = u.pos.dist(p);
            if d < r && best.map_or(true, |b| d < b.0) {
                best = Some((d, self.world.uh(i)));
            }
        }
        best.map(|b| b.1)
    }

    fn bld_at(&self, p: V2) -> Option<Handle> {
        let (x, y) = p.tile();
        if Map::inb(x, y) {
            let b = self.world.bmap[Map::idx(x, y)];
            if b != 0 && self.explored(p) {
                return Some(self.world.bh(b as usize - 1));
            }
        }
        let mut best: Option<(f32, Handle)> = None;
        for (i, b) in self.world.blds.iter().enumerate() {
            if !b.alive || !self.explored(b.center()) {
                continue;
            }
            let d = b.dist_to_footprint(p);
            if d < 0.45 && best.map_or(true, |bs| d < bs.0) {
                best = Some((d, self.world.bh(i)));
            }
        }
        best.map(|bs| bs.1)
    }

    fn select_release(&mut self, sx: f32, sy: f32, x: f32, y: f32) {
        let p = self.world.player;
        if (sx - x).abs() < 5.0 && (sy - y).abs() < 5.0 {
            let wp = self.screen_to_world(x, y);
            if let Some(h) = self.unit_at(wp) {
                let shift = self.keys[VK_SHIFT as usize];
                let double = self.last_click.0 > self.clock - 0.35 && self.last_click.1 == Some(h);
                self.last_click = (self.clock, Some(h));
                let u = self.world.unit(h).unwrap();
                if double && u.nation as usize == p {
                    // 画面内の同種ユニットを全選択
                    let kind = u.kind;
                    let (w, hh) = (self.gfx.width as f32, self.gfx.height as f32);
                    let a = self.screen_to_world(0.0, self.top_h());
                    let b = self.screen_to_world(w, hh - BOT_H);
                    let hs: Vec<Handle> = (0..self.world.units.len())
                        .filter(|&i| {
                            let u = &self.world.units[i];
                            u.alive && u.nation as usize == p && u.kind == kind && u.pos.x >= a.x && u.pos.x <= b.x && u.pos.y >= a.y && u.pos.y <= b.y
                        })
                        .map(|i| self.world.uh(i))
                        .collect();
                    self.set_sel(hs);
                } else if shift && u.nation as usize == p {
                    let mut hs = self.own_sel();
                    if let Some(k) = hs.iter().position(|&x| x == h) {
                        hs.remove(k);
                    } else {
                        hs.push(h);
                    }
                    self.set_sel(hs);
                } else {
                    self.set_sel(vec![h]);
                }
            } else if let Some(b) = self.bld_at(wp) {
                self.clear_sel();
                self.sel_b = Some(b);
            } else {
                self.clear_sel();
            }
        } else {
            let a = self.screen_to_world(sx.min(x), sy.min(y));
            let b = self.screen_to_world(sx.max(x), sy.max(y));
            let hs: Vec<Handle> = (0..self.world.units.len())
                .filter(|&i| {
                    let u = &self.world.units[i];
                    u.alive && u.nation as usize == p && u.pos.x >= a.x && u.pos.x <= b.x && u.pos.y >= a.y && u.pos.y <= b.y
                })
                .map(|i| self.world.uh(i))
                .collect();
            if !hs.is_empty() {
                let mut hs = hs;
                if self.keys[VK_SHIFT as usize] {
                    for h in self.own_sel() {
                        if !hs.contains(&h) {
                            hs.push(h);
                        }
                    }
                }
                self.set_sel(hs);
            } else if !self.keys[VK_SHIFT as usize] {
                self.clear_sel();
            }
        }
    }

    fn right_click_world(&mut self, p: V2) {
        let pl = self.world.player;
        // ユニット未選択時：未完成建物の右クリックなら民を派遣、建物選択中なら集合地点設定
        if self.sel.is_empty() {
            if let Some(bh) = self.bld_at(p) {
                if let Some(b) = self.world.bld(bh) {
                    if b.nation as usize == pl && !b.done() {
                        self.dispatch_worker_to_build(bh);
                        return;
                    }
                }
            }
            if let Some(bh) = self.sel_b {
                if let Some(b) = self.world.bld_mut(bh) {
                    if b.nation as usize == pl && b.kind != BKind::Farm {
                        b.rally = Some(p);
                        self.marker = Some((p, 0.6, false));
                    }
                }
            }
            return;
        }
        let hs = self.own_sel();
        if hs.is_empty() {
            return;
        }
        let workers: Vec<Handle> = hs.iter().copied().filter(|&h| self.world.unit(h).map_or(false, |u| u.kind == UnitKind::Worker)).collect();
        let soldiers: Vec<Handle> = hs.iter().copied().filter(|&h| self.world.unit(h).map_or(false, |u| u.kind.military())).collect();
        if !soldiers.is_empty() {
            if let Some(t) = &mut self.tut {
                t.moved = true;
            }
        }
        // 敵ユニット
        if let Some(t) = self.unit_at(p) {
            let tn = self.world.unit(t).unwrap().nation as usize;
            if tn != pl {
                if self.world.at_war(pl, tn) {
                    self.world.order_attack(&hs, Target::Unit(t));
                    self.marker = Some((p, 0.6, true));
                } else {
                    self.notice(&tf!("{} とは交戦していない（外交 F2 で宣戦できる）", "Not at war with {} (declare war in Diplomacy, F2)", "未与{}交战（可在外交 F2 中宣战）", crate::lang::tx(self.world.nations[tn].name)));
                    self.world.order_move(&hs, p, false);
                }
                return;
            }
        }
        if let Some(bh) = self.bld_at(p) {
            let b = self.world.bld(bh).unwrap();
            let (bn, kind, done, bc) = (b.nation as usize, b.kind, b.done(), b.center());
            if bn != pl {
                if self.world.at_war(pl, bn) {
                    self.world.order_attack(&hs, Target::Bld(bh));
                    self.marker = Some((p, 0.6, true));
                } else {
                    self.notice(&tf!("{} とは交戦していない（外交 F2 で宣戦できる）", "Not at war with {} (declare war in Diplomacy, F2)", "未与{}交战（可在外交 F2 中宣战）", crate::lang::tx(self.world.nations[bn].name)));
                }
                return;
            }
            if !done {
                if !workers.is_empty() {
                    self.world.order_build(&workers, bh);
                    self.marker = Some((bc, 0.6, false));
                    self.notice(&tf!("{}の建設へ民{}体を向かわせた", "Sent {1} workers to build the {0}", "派{1}名民去建造{0}", kind.name(), workers.len()));
                } else if self.world.bld_builders(bh) == 0 {
                    self.dispatch_worker_to_build(bh);
                }
                if !soldiers.is_empty() {
                    self.world.order_move(&soldiers, p, false);
                }
                return;
            }
            if kind == BKind::Farm && !workers.is_empty() {
                self.world.order_gather(&workers[..1], Src::Farm(bh));
                if workers.len() > 1 {
                    self.world.order_move(&workers[1..], p, false);
                }
                self.world.order_move(&soldiers, p, false);
                return;
            }
        }
        let (tx, ty) = p.tile();
        if !workers.is_empty() && Map::inb(tx, ty) {
            if self.world.map.terrain[Map::idx(tx, ty)] == Terrain::Forest {
                self.world.order_gather(&workers, Src::Forest(tx, ty));
                self.world.order_move(&soldiers, p, false);
                self.marker = Some((p, 0.6, false));
                return;
            }
            if let Some(m) = self.world.map.mines.iter().position(|m| m.x == tx && m.y == ty && m.amount > 0.0) {
                self.world.order_gather(&workers, Src::Mine(m));
                self.world.order_move(&soldiers, p, false);
                self.marker = Some((p, 0.6, false));
                return;
            }
        }
        self.world.order_move(&hs, p, false);
        self.marker = Some((p, 0.6, false));
    }

    fn placement_tile(&self) -> (i32, i32) {
        let p = self.screen_to_world(self.mouse.0, self.mouse.1);
        ((p.x - 0.5).round() as i32, (p.y - 0.5).round() as i32)
    }

    /// 建物を置く。queue なら建設の予約（選んだ民は今の建設を終えてから取りかかる）
    fn try_place(&mut self, k: BKind, queue: bool) {
        let (tx, ty) = self.placement_tile();
        let pl = self.world.player;
        match self.world.place_building(pl, k, tx, ty) {
            Ok(bh) => {
                let ws: Vec<Handle> = self.own_sel().into_iter().filter(|&h| self.world.unit(h).map_or(false, |u| u.kind == UnitKind::Worker)).collect();
                if queue {
                    self.world.queue_build(&ws, bh);
                } else {
                    self.world.order_build(&ws, bh);
                }
            }
            Err(e) => self.notice(e),
        }
    }

    fn title_click(&mut self, x: f32, y: f32) {
        let p = self.screen_to_world(x, y);
        let mut best: Option<(f32, usize)> = None;
        for (i, n) in self.world.nations.iter().enumerate() {
            if !n.alive {
                continue;
            }
            let d = n.home.dist(p);
            if d < 10.0 && best.map_or(true, |b| d < b.0) {
                best = Some((d, i));
            }
        }
        if let Some((_, i)) = best {
            self.title_sel = Some(i);
        }
    }

    fn button_at(&self, x: f32, y: f32) -> Option<Act> {
        for b in self.buttons.iter().rev() {
            if x >= b.x0 && x <= b.x1 && y >= b.y0 && y <= b.y1 {
                return Some(if b.enabled { b.act } else { Act::Noop });
            }
        }
        // タッチは指先がずれやすいので、すぐ近くのボタンも拾う
        if self.touch {
            let near = self
                .buttons
                .iter()
                .rev()
                .map(|b| ((b.x0 - x).max(x - b.x1).max(0.0).hypot((b.y0 - y).max(y - b.y1).max(0.0)), b))
                .filter(|(d, _)| *d <= 10.0)
                .min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, b)) = near {
                return Some(if b.enabled { b.act } else { Act::Noop });
            }
        }
        None
    }

    fn do_act(&mut self, a: Act) {
        // 部隊への指示（移動・攻撃以外）でも時間を再開する
        if matches!(a, Act::Upgrade | Act::Convert | Act::Stop | Act::WorkersAuto | Act::ArmyDefend | Act::Demolish) {
            self.cmd_pause = false;
        }
        let pl = self.world.player;
        match a {
            Act::Noop => {}
            Act::Start => {
                // 押した場所に出るボタンの説明を、始めた直後に出さない
                self.tip_until = 0.0;
                if let Some(n) = self.title_sel {
                    self.world.player = n;
                    self.world.heroic = self.heroic_opt;
                    self.screen = Screen::Playing;
                    self.zoom = 22.0;
                    self.cam = self.world.nations[n].home;
                    self.reset_fog();
                    self.autosave_years = self.world.years;
                    let name = crate::lang::tx(self.world.nations[n].name);
                    self.world.push_msg(tf!("紀元前770年、周の東遷。あなたは {} の君主となった。", "770 BC: Zhou moves its capital east. You are now ruler of {}.", "公元前770年，周室东迁。你成为了{}的君主。", name), [1.0, 0.9, 0.6]);
                    self.world.push_msg(t!("F1でヘルプ、F2で外交画面を開く", "F1 opens Help, F2 opens Diplomacy", "F1 打开帮助，F2 打开外交画面").into(), [0.8, 0.8, 0.8]);
                }
            }
            Act::Tutorial => self.start_tutorial(),
            Act::TutNext => self.tut_next(),
            Act::TutQuit => self.end_tutorial(),
            Act::Scale(d) => {
                self.gfx.adjust_scale(d);
                if self.screen == Screen::Title {
                    self.fit_title_camera();
                }
                self.notice(&tf!("文字の大きさ ×{:.2}", "Text size ×{:.2}", "文字大小 ×{:.2}", self.gfx.ui));
            }
            Act::Fullscreen => self.gfx.toggle_fullscreen(),
            Act::Lang => {
                let l = crate::lang::lang().next();
                crate::lang::set_lang(l);
                crate::platform::save_lang(l);
                self.gfx.reload_fonts();
            }
            Act::Cancel => {
                if self.mode != Mode::Normal {
                    self.mode = Mode::Normal;
                    self.place_tap = None;
                } else {
                    self.clear_sel();
                }
            }
            Act::SaveMenu => {
                self.show_save = !self.show_save;
                if self.show_save {
                    self.show_diplo = false;
                    self.show_help = false;
                    self.show_chron = false;
                    self.show_market = false;
                    self.show_summon = false;
                }
            }
            Act::SaveSlot(s) => self.save_slot(s, false),
            Act::LoadSlot(s) => {
                self.notice(t!("記録を読み込んでいる…", "Loading the save...", "正在读取存档…"));
                crate::platform::load_slot(s);
            }
            Act::ExportSlot(s) => crate::platform::export_slot(s),
            Act::ImportFile => crate::platform::import_file(),
            Act::Restart => {
                self.tut = None;
                self.seed = self.seed.wrapping_add(17);
                self.world = World::new(self.seed);
                self.rebuild_terrain();
                self.screen = Screen::Title;
                self.title_sel = None;
                self.sel.clear();
                self.sel_b = None;
                self.show_diplo = false;
                self.show_help = false;
                self.show_chron = false;
                self.show_market = false;
                self.show_save = false;
                self.show_summon = false;
                self.paused = false;
                self.speed = 1;
                self.fit_title_camera();
            }
            Act::Train(k) => {
                if let Some(bh) = self.sel_b {
                    if let Err(e) = self.world.enqueue(bh, k) {
                        self.notice(e);
                    }
                }
            }
            Act::Place(k) => {
                let cost = k.cost();
                let r = self.world.nations[pl].res;
                if r[0] < cost[0] || r[1] < cost[1] || r[2] < cost[2] {
                    self.notice(t!("資源が足りない", "Not enough resources", "资源不足"));
                } else {
                    self.mode = Mode::Place(k);
                    self.place_tap = None;
                }
            }
            Act::Upgrade => {
                let hs = self.own_sel();
                self.upgrade_handles(hs);
            }
            Act::Convert => {
                let mut err = None;
                for h in self.own_sel() {
                    if let Err(e) = self.world.convert_unit(h) {
                        err = Some(e);
                    }
                }
                if let Some(e) = err {
                    self.notice(e);
                }
            }
            Act::Stop => {
                for h in self.own_sel() {
                    if let Some(u) = self.world.unit_mut(h) {
                        u.order = Order::Idle;
                        u.resume = None;
                        u.path.clear();
                        u.build_queue.clear();
                    }
                }
            }
            Act::AttackMove => {
                if !self.own_sel().is_empty() {
                    self.mode = Mode::AttackMove;
                }
            }
            Act::SelectAllWorkers => {
                let hs = self.all_workers();
                if hs.is_empty() {
                    self.notice(t!("民がいない", "No workers", "没有民"));
                } else {
                    self.notice(&tf!("民全体（{}体）を選択した", "Selected all workers ({})", "已选择全部民（{}名）", hs.len()));
                    self.set_sel(hs);
                }
            }
            Act::SelectAllMilitary => {
                let hs = self.all_military();
                if hs.is_empty() {
                    self.notice(t!("軍事ユニットがいない", "No soldiers", "没有军事单位"));
                } else {
                    self.notice(&tf!("全軍事ユニット（{}体）を選択した", "Selected all soldiers ({})", "已选择全部军事单位（{}个）", hs.len()));
                    self.set_sel(hs);
                }
            }
            Act::SelectKind(kind) => {
                let hs: Vec<Handle> = (0..self.world.units.len())
                    .filter(|&i| {
                        let u = &self.world.units[i];
                        u.alive && u.nation as usize == pl && u.kind == kind
                    })
                    .map(|i| self.world.uh(i))
                    .collect();
                if hs.is_empty() {
                    self.notice(&tf!("{}はいない", "No {} units", "没有{}", kind.name(self.world.era)));
                } else {
                    self.notice(&tf!("{}全体（{}体）を選択した", "Selected all {} units ({})", "已选择全部{}（{}个）", kind.name(self.world.era), hs.len()));
                    self.set_sel(hs);
                }
            }
            Act::FilterKind(kind) => {
                let hs: Vec<Handle> = self
                    .own_sel()
                    .into_iter()
                    .filter(|&h| self.world.unit(h).map_or(false, |u| u.kind == kind))
                    .collect();
                if !hs.is_empty() {
                    self.set_sel(hs);
                }
            }
            Act::WorkersAuto => {
                let hs = self.own_sel();
                self.order_workers_balanced(&hs, false);
            }
            Act::AllWorkersAuto => {
                let hs = self.all_workers();
                self.order_workers_balanced(&hs, true);
            }
            Act::AllWorkersWood => {
                let hs = self.all_workers();
                self.order_workers_resource(&hs, true);
            }
            Act::AllWorkersMine => {
                let hs = self.all_workers();
                self.order_workers_resource(&hs, false);
            }
            Act::AllWorkersUpgrade => {
                let hs = self.all_workers();
                if hs.is_empty() {
                    self.notice(t!("民がいない", "No workers", "没有民"));
                } else {
                    self.upgrade_handles(hs);
                }
            }
            Act::ArmyDefend => {
                let hs: Vec<Handle> = self.own_sel().into_iter().filter(|&h| self.world.unit(h).map_or(false, |u| u.kind.military())).collect();
                self.order_army_defend(&hs);
            }
            Act::AllArmyAttack => {
                self.order_all_army_attack();
            }
            Act::AllArmyDefend => {
                let hs = self.all_military();
                self.order_army_defend(&hs);
            }
            Act::AllArmyUpgrade => {
                let hs = self.all_military();
                if hs.is_empty() {
                    self.notice(t!("軍事ユニットがいない", "No soldiers", "没有军事单位"));
                } else {
                    self.upgrade_handles(hs);
                }
            }
            Act::Speed(s) => {
                if s == 0 {
                    self.paused = !self.paused;
                } else {
                    self.speed = s;
                    self.paused = false;
                }
            }
            Act::Diplo => {
                self.show_diplo = !self.show_diplo;
                self.show_help = false;
                self.show_chron = false;
                self.show_market = false;
                self.show_save = false;
            }
            Act::Help => {
                self.show_help = !self.show_help;
                self.show_diplo = false;
                self.show_chron = false;
                self.show_market = false;
                self.show_save = false;
            }
            Act::Chron => {
                self.show_chron = !self.show_chron;
                self.chron_scroll = 0;
                self.show_diplo = false;
                self.show_help = false;
                self.show_market = false;
                self.show_save = false;
            }
            Act::Market => {
                self.show_market = !self.show_market;
                self.show_diplo = false;
                self.show_help = false;
                self.show_chron = false;
            }
            Act::Tax(t) => {
                self.world.nations[pl].tax = t;
                self.notice(&tf!("税率を{}にした", "Tax set to {}", "税率改为{}", tax_name(t as usize)));
            }
            Act::Buy(k) => match self.world.buy_resource(pl, k) {
                Ok(()) => self.notice(&tf!("市場で{}を{}買った", "Bought {1} {0} at the market", "在市场购入{0}{1}", res_name(k), MARKET_LOT as i32)),
                Err(e) => self.notice(e),
            },
            Act::Hire(t) => match self.world.hire_mercs(pl, t) {
                Ok(()) => self.notice(&tf!("{}の傭兵を雇った", "Hired {} mercenaries", "雇佣了{}佣兵", crate::lang::tx(TRIBES[t].name))),
                Err(e) => self.notice(e),
            },
            Act::ChronPage(d) => {
                self.chron_scroll = (self.chron_scroll as i32 + d).max(0) as usize;
            }
            Act::ToggleHeroic => {
                self.heroic_opt = !self.heroic_opt;
            }
            Act::Summon => {
                if self.world.heroic {
                    self.show_summon = !self.show_summon;
                    self.show_diplo = false;
                    self.show_help = false;
                    self.show_chron = false;
                    self.show_market = false;
                    self.show_save = false;
                }
            }
            Act::SummonHero(k) => {
                let own = self.own_sel();
                match self.world.summon_hero(pl, k, &own) {
                    Ok(h) => {
                        self.show_summon = false;
                        self.sel = vec![h];
                    }
                    Err(e) => self.notice(e),
                }
            }
            Act::CloseOverlay => {
                self.show_summon = false;
                self.show_diplo = false;
                self.show_help = false;
                self.show_chron = false;
                self.show_market = false;
                self.show_save = false;
            }
            Act::War(o) => {
                if self.world.truce[pl][o] > self.world.years {
                    let r = (self.world.truce[pl][o] - self.world.years).ceil() as i32;
                    self.notice(&tf!("休戦期間中（残り{}年）", "Truce in effect ({} years left)", "休战期内（剩余{}年）", r));
                } else {
                    self.world.declare_war(pl, o, false);
                }
            }
            Act::Peace(o) => {
                let w = &mut self.world;
                if w.coalition_locked(pl, o) {
                    self.notice(t!("連合軍の討伐中は和睦できない", "Cannot make peace while a coalition campaign is on", "联军讨伐期间无法议和"));
                    return;
                }
                let dur = w.years - w.war_start[pl][o];
                let accept = w.peace_offer[o] || (dur > 5.0 && (w.nations[o].strength < w.nations[pl].strength * 1.1 || w.rng.chance(0.25)));
                if accept {
                    w.make_peace(pl, o, 10.0);
                } else {
                    let t = tf!("{} は和睦を拒否した", "{} refused peace", "{}拒绝议和", crate::lang::tx(w.nations[o].name));
                    self.notice(&t);
                }
            }
            Act::PeaceAnswer(accept) => self.world.answer_peace(accept),
            Act::Home => {
                if let Some(c) = self.world.nations[pl].capital.and_then(|h| self.world.bld(h)) {
                    self.cam = c.center();
                }
            }
            Act::Demolish => {
                if let Some(bh) = self.sel_b {
                    if let Some(b) = self.world.bld(bh) {
                        if b.nation as usize == pl && (!b.kind.city() || !b.done()) {
                            let i = bh.idx as usize;
                            self.world.remove_building(i);
                            self.sel_b = None;
                        }
                    }
                }
            }
            Act::ResumeBuild => {
                if let Some(bh) = self.sel_b {
                    self.dispatch_worker_to_build(bh);
                }
            }
            Act::IdleWorker => {
                let idle: Vec<usize> = (0..self.world.units.len())
                    .filter(|&i| {
                        let u = &self.world.units[i];
                        u.alive && u.nation as usize == pl && u.kind == UnitKind::Worker && u.order == Order::Idle
                    })
                    .collect();
                if idle.is_empty() {
                    self.notice(t!("待機中の民はいない", "No idle workers", "没有闲置的民"));
                } else {
                    self.idle_cycle = (self.idle_cycle + 1) % idle.len();
                    let i = idle[self.idle_cycle];
                    let h = self.world.uh(i);
                    self.cam = self.world.units[i].pos;
                    self.set_sel(vec![h]);
                }
            }
        }
    }

    // ------------------------------------------------------------ 描画
    fn render(&mut self, dt: f32) {
        self.render_nopresent(dt);
        self.gfx.present();
    }

    fn render_nopresent(&mut self, dt: f32) {
        self.update_minimap(dt);
        self.gfx.begin([0.07, 0.06, 0.05, 1.0]);
        self.render_world();
        if self.gfx.d2d_begin() {
            self.art.ensure(&self.gfx);
            self.buttons.clear();
            self.render_world_labels();
            if self.screen == Screen::Playing {
                self.render_hero_labels();
                self.render_build_queue();
            }
            // スマートフォン向け画面では、縦に収まらない一覧は指でなぞってスクロールする
            let key = self.overlay_key();
            if key != self.ov_key {
                self.ov_key = key;
                self.ov_scroll = 0.0;
            }
            self.ov_max = 0.0;
            if self.screen == Screen::Title {
                self.render_title();
                if self.show_save {
                    self.scrolled(Self::render_save);
                }
            } else {
                self.render_hud();
                if self.show_diplo {
                    self.scrolled(Self::render_diplo);
                }
                if self.show_help {
                    self.scrolled(Self::render_help);
                }
                if self.show_chron {
                    self.scrolled(Self::render_chron);
                }
                if self.show_market {
                    self.scrolled(Self::render_market);
                }
                if self.show_summon {
                    self.scrolled(Self::render_summon);
                }
                if self.show_save {
                    self.scrolled(Self::render_save);
                }
                if self.peace_pending() {
                    self.render_peace_ask();
                }
                if self.world.game_over.is_some() {
                    self.render_gameover();
                }
                self.render_tutorial();
                self.render_tooltip();
            }
            self.gfx.d2d_end();
        }
    }

    fn render_world(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let z = self.zoom;
        let sx = 2.0 * z / w;
        let sy = -2.0 * z / h;
        self.gfx.set_xform(sx, sy, -self.cam.x * sx, -self.cam.y * sy);
        let place_tile = self.placement_tile();
        let fog_on = self.fog_active();
        let pl = self.world.player;
        let fog = &self.fog;
        let fog_at = move |p: V2| -> u8 {
            if !fog_on {
                return 2;
            }
            let (x, y) = p.tile();
            if Map::inb(x, y) {
                fog[Map::idx(x, y)]
            } else {
                0
            }
        };
        let g = &mut self.gfx;
        let mw = MAP_W as f32;
        let mh = MAP_H as f32;
        crate::art::draw_map_frame(g, mw, mh);
        g.draw_textured(&self.terrain_tex, 0.0, 0.0, mw, mh, [1.0; 4], true);
        g.draw_textured(&self.overlay_tex, 0.0, 0.0, mw, mh, [1.0; 4], true);
        let wd = &self.world;
        let view0 = v2(self.cam.x - w / 2.0 / z - 2.0, self.cam.y - h / 2.0 / z - 2.0);
        let view1 = v2(self.cam.x + w / 2.0 / z + 2.0, self.cam.y + h / 2.0 / z + 2.0);
        let inview = |p: V2| p.x >= view0.x && p.x <= view1.x && p.y >= view0.y && p.y <= view1.y;
        let px = 1.0 / z; // 1ピクセル
        // 森林の樹木・山岳の立体表現
        crate::art::draw_world_terrain_props(
            g,
            &wd.map,
            view0.x.floor() as i32,
            view0.y.floor() as i32,
            view1.x.ceil() as i32,
            view1.y.ceil() as i32,
            z,
            &fog_at,
        );
        // 銅山
        for m in wd.map.mines.iter() {
            if m.amount <= 0.0 {
                continue;
            }
            let c = v2(m.x as f32 + 0.5, m.y as f32 + 0.5);
            if !inview(c) || fog_at(c) == 0 {
                continue;
            }
            crate::art::draw_world_mine(g, c, m.amount, px);
        }
        // 建物
        for (bi, b) in wd.blds.iter().enumerate() {
            if !b.alive {
                continue;
            }
            let c = b.center();
            if !inview(c) || fog_at(c) == 0 {
                continue;
            }
            let nc = wd.nations[b.nation as usize].color;
            let (x0, y0, x1, y1) = (b.tx as f32, b.ty as f32, b.tx as f32 + 2.0, b.ty as f32 + 2.0);
            if wd.descent.is_some() && b.nation as usize == 2 && b.kind.city() {
                let dr = if b.kind == BKind::Capital { 1.58 } else { 1.32 };
                g.circle(c.x, c.y + 0.08, dr, 24, [1.0, 0.92, 0.45, 0.25]);
                g.ring(c.x, c.y + 0.08, dr, (2.2 * px).max(0.06), 24, [1.0, 0.90, 0.42, 0.90]);
            }
            crate::art::draw_world_building(g, b, nc, px);
            let sel = self.sel_b == Some(wd.bh(bi));
            if sel {
                g.rect_outline(x0 - 0.32, y0 - 0.32, x1 + 0.32, y1 + 0.32, (2.2 * px).max(0.05), [1.0, 0.92, 0.55, 0.95]);
                if let Some(r) = b.rally {
                    g.line((c.x, c.y), (r.x, r.y), 1.5 * px, [1.0, 0.92, 0.55, 0.65]);
                    g.rect(r.x - 0.05, r.y - 0.65, r.x + 0.05, r.y, [0.92, 0.85, 0.65, 1.0]);
                    g.tri((r.x + 0.05, r.y - 0.65), (r.x + 0.48, r.y - 0.48), (r.x + 0.05, r.y - 0.32), c4(nc, 1.0));
                }
            }
        }
        // 隊商（商人・西域の商人・部族）
        for c in wd.trade.caravans.iter() {
            if !inview(c.pos) || fog_at(c.pos) != 2 {
                continue;
            }
            crate::game::market::draw_caravan(g, c, px);
        }
        // ユニット
        let era = wd.era;
        for u in wd.units.iter() {
            if !u.alive || !inview(u.pos) || (u.nation as usize != pl && fog_at(u.pos) != 2) {
                continue;
            }
            let nc = wd.nations[u.nation as usize].color;
            let ue = if (u.tribe as usize) < TRIBES.len() && TRIBES[u.tribe as usize].nomad { Era::Warring } else { era };
            if u.hero != NONE {
                g.circle(u.pos.x, u.pos.y + 0.15, 0.75, 16, [1.0, 0.85, 0.35, 0.35]);
                g.ring(u.pos.x, u.pos.y + 0.15, 0.75, (2.0 * px).max(0.05), 16, [1.0, 0.85, 0.35, 0.95]);
            } else if u.divine {
                g.circle(u.pos.x, u.pos.y + 0.12, 0.56, 16, [1.0, 0.94, 0.55, 0.28]);
                g.ring(u.pos.x, u.pos.y + 0.12, 0.56, (1.8 * px).max(0.045), 16, [1.0, 0.92, 0.48, 0.92]);
            }
            crate::art::draw_world_unit(g, u, nc, ue, &RES_COLORS, px);
        }
        // 矢
        for s in wd.shots.iter() {
            let t = 1.0 - s.t / 0.3;
            let p = s.a + (s.b - s.a) * t.clamp(0.0, 1.0);
            if fog_at(p) != 2 {
                continue;
            }
            let d = (s.b - s.a).norm();
            g.line((p.x - d.x * 0.34, p.y - d.y * 0.34), (p.x, p.y), (2.2 * px).max(0.045), [0.85, 0.72, 0.45, 0.95]);
            g.circle(p.x, p.y, (1.8 * px).max(0.04), 5, [1.0, 0.95, 0.65, 1.0]);
        }
        if fog_on {
            g.draw_textured(&self.fog_tex, 0.0, 0.0, mw, mh, [1.0; 4], true);
        }
        // 移動目標の印
        if let Some((p, t, atk)) = self.marker {
            let r = 0.2 + t * 0.6;
            let c = if atk { [1.0, 0.3, 0.2, t / 0.6] } else { [0.4, 1.0, 0.4, t / 0.6] };
            g.ring(p.x, p.y, r, 2.0 * px, 16, c);
        }
        // 建設予定地
        if let Mode::Place(k) = self.mode {
            let (tx, ty) = place_tile;
            let ok = wd.can_place(wd.player, k, tx, ty).is_ok();
            let c = if ok { [0.3, 1.0, 0.3, 0.45] } else { [1.0, 0.25, 0.2, 0.45] };
            g.rect(tx as f32, ty as f32, tx as f32 + 2.0, ty as f32 + 2.0, c);
            if k.city() {
                g.ring(tx as f32 + 1.0, ty as f32 + 1.0, k.radius() as f32, 2.0 * px, 48, [1.0, 1.0, 1.0, 0.5]);
            }
        }
        g.flush();
        // 画面座標での選択矩形
        if let Some((sx, sy)) = self.ldown {
            let (mx, my) = self.mouse;
            if (sx - mx).abs() > 4.0 || (sy - my).abs() > 4.0 {
                self.gfx.set_screen_xform();
                let g = &mut self.gfx;
                g.rect(sx.min(mx), sy.min(my), sx.max(mx), sy.max(my), [0.4, 1.0, 0.4, 0.12]);
                g.rect_outline(sx.min(mx), sy.min(my), sx.max(mx), sy.max(my), 1.5, [0.4, 1.0, 0.4, 0.9]);
                g.flush();
            }
        }
    }

    /// 選んだ民の建設の順番（建設中＝1、予約＝2, 3, …）を建設地に示す
    fn render_build_queue(&mut self) {
        let pl = self.world.player;
        let mut order: Vec<(Handle, usize)> = Vec::new();
        for h in self.sel.iter() {
            let Some(u) = self.world.unit(*h) else { continue };
            if u.nation as usize != pl || u.kind != UnitKind::Worker || u.build_queue.is_empty() {
                continue;
            }
            let cur = if let Order::Build(b) = u.order { Some(b) } else { None };
            for (k, &b) in cur.iter().chain(u.build_queue.iter()).enumerate() {
                match order.iter_mut().find(|x| x.0 == b) {
                    Some(x) => x.1 = x.1.min(k + 1),
                    None => order.push((b, k + 1)),
                }
            }
        }
        for (b, k) in order {
            let Some(bd) = self.world.bld(b) else { continue };
            if bd.done() {
                continue;
            }
            let (sx, sy) = self.world_to_screen(bd.center());
            let r = 11.0;
            self.gfx.fill_rect(sx - r, sy - r - 18.0, sx + r, sy + r - 18.0, [0.12, 0.10, 0.06, 0.85]);
            self.gfx.fill_rect(sx - r, sy + r - 20.0, sx + r, sy + r - 18.0, [1.0, 0.8, 0.35, 1.0]);
            let s = format!("{}", k);
            let tw = self.gfx.measure(&s, Font::Normal);
            self.gfx.text(&s, sx - tw / 2.0, sy - r - 18.0, tw + 4.0, 2.0 * r, Font::Normal, [1.0, 0.88, 0.5, 1.0]);
        }
    }

    fn render_world_labels(&mut self) {
        let z = self.zoom;
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let fog_on = self.fog_active();
        let pl = self.world.player;

        enum WorldItem {
            Mountain { sx: f32, sy: f32, r: f32, flip_x: bool, fogged: bool },
            Forest { sx: f32, sy: f32, r: f32, flip_x: bool, fogged: bool },
            Mine { sx: f32, sy: f32, r: f32, fogged: bool },
            Building {
                sx: f32,
                sy: f32,
                r: f32,
                kind: BKind,
                nc: [f32; 3],
                done: bool,
                progress: f32,
                hp_frac: Option<f32>,
                fogged: bool,
                divine: bool,
            },
            Unit {
                sx: f32,
                sy: f32,
                kind: UnitKind,
                ue: Era,
                pw: f32,
                nc: [f32; 3],
                carry: Option<usize>,
                flip_x: bool,
                hp_frac: Option<f32>,
                hero: u8,
                divine: bool,
            },
        }

        if z >= 2.2 {
            let mut items: Vec<(f32, WorldItem)> = Vec::new();
            let view0 = v2(self.cam.x - w / 2.0 / z - 2.5, self.cam.y - h / 2.0 / z - 2.5);
            let view1 = v2(self.cam.x + w / 2.0 / z + 2.5, self.cam.y + h / 2.0 / z + 2.5);
            let x0 = (view0.x.floor() as i32).max(0);
            let y0 = (view0.y.floor() as i32).max(0);
            let x1 = (view1.x.ceil() as i32).min(MAP_W as i32 - 1);
            let y1 = (view1.y.ceil() as i32).min(MAP_H as i32 - 1);

            // 山岳・森林のAI生成スプライト
            let thin = z < 8.5;
            for ty in y0..=y1 {
                for tx in x0..=x1 {
                    let idx = Map::idx(tx, ty);
                    let t = self.world.map.terrain[idx];
                    if !matches!(t, Terrain::Mountain | Terrain::Forest) {
                        continue;
                    }
                    if z < 5.5 {
                        if ((tx * 3 + ty * 5) & 3) != 0 {
                            continue;
                        }
                    } else if thin && ((tx + ty) & 1) != 0 {
                        continue;
                    }
                    let f = if fog_on { self.fog[idx] } else { 2 };
                    if f == 0 {
                        continue;
                    }
                    let fogged = f == 1;
                    let h1 = crate::rng::hash2(tx, ty, 31);
                    let h2 = crate::rng::hash2(tx, ty, 47);
                    let flip_x = h1 > 0.5;
                    let ox = (h1 - 0.5) * 0.28;
                    let oy = (h2 - 0.5) * 0.24;
                    let wp = v2(tx as f32 + 0.5 + ox, ty as f32 + 0.5 + oy);
                    let (sx, sy) = self.world_to_screen(wp);
                    if sx < -64.0 || sy < -64.0 || sx > w + 64.0 || sy > h + 64.0 {
                        continue;
                    }
                    match t {
                        Terrain::Mountain => {
                            let mult = if z < 5.5 { 1.65 } else if thin { 1.25 } else { 0.92 + 0.22 * h2 };
                            let r = (z * mult).clamp(5.5, 44.0);
                            items.push((sy + r * 0.25, WorldItem::Mountain { sx, sy, r, flip_x, fogged }));
                        }
                        Terrain::Forest => {
                            let mult = if z < 5.5 { 1.45 } else if thin { 1.12 } else { 0.80 + 0.20 * h2 };
                            let r = (z * mult).clamp(5.0, 38.0);
                            items.push((sy + r * 0.22, WorldItem::Forest { sx, sy, r, flip_x, fogged }));
                        }
                        _ => {}
                    }
                }
            }

            // 銅山のAI生成スプライト
            for m in self.world.map.mines.iter() {
                if m.amount <= 0.0 {
                    continue;
                }
                let wp = v2(m.x as f32 + 0.5, m.y as f32 + 0.5);
                if !self.explored(wp) {
                    continue;
                }
                let (sx, sy) = self.world_to_screen(wp);
                if sx < -64.0 || sy < -64.0 || sx > w + 64.0 || sy > h + 64.0 {
                    continue;
                }
                let fogged = fog_on && !self.seen(wp);
                let s = 0.78 + 0.24 * (m.amount / 2500.0).clamp(0.2, 1.0);
                let r = (z * s).clamp(6.5, 38.0);
                items.push((sy + r * 0.28, WorldItem::Mine { sx, sy, r, fogged }));
            }

            // 建物（都・邑・兵営・田）のAI生成スプライト
            for (bi, b) in self.world.blds.iter().enumerate() {
                if !b.alive {
                    continue;
                }
                let c = b.center();
                if !self.explored(c) {
                    continue;
                }
                let (sx, sy) = self.world_to_screen(c);
                if sx < -96.0 || sy < -96.0 || sx > w + 96.0 || sy > h + 96.0 {
                    continue;
                }
                let fogged = fog_on && !self.seen(c);
                let nc = self.world.nations[b.nation as usize].color;
                let mult = match b.kind {
                    BKind::Capital => if z < 5.5 { 2.25 } else { 1.62 },
                    BKind::Town => if z < 5.5 { 1.75 } else { 1.38 },
                    BKind::Barracks => 1.25,
                    BKind::Farm => 1.16,
                    BKind::Fort => 1.3,
                };
                let r = (z * mult).clamp(9.0, 68.0);
                let sel = self.sel_b == Some(self.world.bh(bi));
                let hp_frac = if b.done() && (b.hp < b.kind.max_hp() - 0.5 || sel) {
                    Some((b.hp / b.kind.max_hp()).clamp(0.0, 1.0))
                } else {
                    None
                };
                let sort_y = if b.kind == BKind::Farm { sy - r * 0.2 } else { sy + r * 0.45 };
                let divine = self.world.descent.is_some() && b.nation as usize == 2 && b.kind.city();
                items.push((
                    sort_y,
                    WorldItem::Building {
                        sx,
                        sy,
                        r,
                        kind: b.kind,
                        nc,
                        done: b.done(),
                        progress: b.progress,
                        hp_frac,
                        fogged,
                        divine,
                    },
                ));
            }

            // ユニットの高精細AI生成スプライト（マップや建物に対して適切なサイズに縮小）
            if z >= 6.5 && self.screen == Screen::Playing {
                for u in self.world.units.iter() {
                    if !u.alive || (u.nation as usize != pl && !self.seen(u.pos)) {
                        continue;
                    }
                    let (sx, sy) = self.world_to_screen(u.pos);
                    if sx < -48.0 || sy < -48.0 || sx > w + 48.0 || sy > h + 48.0 {
                        continue;
                    }
                    let nc = self.world.nations[u.nation as usize].color;
                    let carry = if u.carry > 0.5 { Some(u.carry_kind) } else { None };
                    let flip_x = u.dir.x < -0.05;
                    let mhp = u.max_hp();
                    let hp_frac = if u.hp < mhp - 0.5 || u.selected {
                        Some((u.hp / mhp).clamp(0.0, 1.0))
                    } else {
                        None
                    };
                    items.push((
                        sy + 2.0,
                        WorldItem::Unit {
                            sx,
                            sy,
                            kind: u.kind,
                            ue: if (u.tribe as usize) < TRIBES.len() && TRIBES[u.tribe as usize].nomad { Era::Warring } else { self.world.era },
                            pw: u.power,
                            nc,
                            carry,
                            flip_x,
                            hp_frac,
                            hero: u.hero,
                            divine: u.divine,
                        },
                    ));
                }
            }

            items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

            for (_, item) in items {
                match item {
                    WorldItem::Mountain { sx, sy, r, flip_x, fogged } => {
                        if let Some(bmp) = self.art.mountain_sprite(flip_x, fogged) {
                            self.gfx.draw_bitmap(bmp, sx - r, sy - r * 1.18, sx + r, sy + r * 0.82);
                        }
                    }
                    WorldItem::Forest { sx, sy, r, flip_x, fogged } => {
                        if let Some(bmp) = self.art.forest_sprite(flip_x, fogged) {
                            self.gfx.draw_bitmap(bmp, sx - r, sy - r * 1.12, sx + r, sy + r * 0.88);
                        }
                    }
                    WorldItem::Mine { sx, sy, r, fogged } => {
                        if let Some(bmp) = self.art.mine_sprite(fogged) {
                            self.gfx.draw_bitmap(bmp, sx - r, sy - r * 1.05, sx + r, sy + r * 0.95);
                        }
                    }
                    WorldItem::Building { sx, sy, r, kind, nc, done, progress, hp_frac, fogged, divine } => {
                        let alpha = if done { 1.0 } else { 0.65 };
                        let b_bmp = if divine {
                            self.art.divine_bld_sprite(kind, fogged)
                        } else {
                            self.art.bld_sprite(kind, fogged)
                        };
                        if let Some(bmp) = b_bmp {
                            self.gfx.draw_bitmap_alpha(bmp, sx - r, sy - r * 1.08, sx + r, sy + r * 0.92, alpha);
                        }
                        // 建物右下の国色紋章バッジ（どの国の城邑・兵営か一目で判別可能に）
                        if kind != BKind::Farm && z >= 7.0 {
                            let br = (r * 0.16).clamp(3.2, 6.5);
                            let bx = sx + r * 0.56;
                            let by = sy + r * 0.48;
                            let k = if fogged { 0.55 } else { 1.0 };
                            self.gfx.fill_circle(bx, by, br + 1.2, [0.06, 0.05, 0.04, 0.92]);
                            if divine {
                                self.gfx.fill_circle(bx, by, br, [1.0 * k, 0.90 * k, 0.42 * k, 1.0]);
                                self.gfx.stroke_circle(bx, by, br + 0.6, 1.3, [0.95 * k, 0.98 * k, 1.0 * k, 0.98]);
                            } else {
                                self.gfx.fill_circle(bx, by, br, [nc[0] * k, nc[1] * k, nc[2] * k, 1.0]);
                                self.gfx.stroke_circle(bx, by, br + 0.5, 1.1, [0.95 * k, 0.82 * k, 0.38 * k, 0.95]);
                            }
                        }
                        if !done {
                            let bw = (r * 1.4).max(16.0);
                            let by = sy + r * 0.72;
                            self.gfx.fill_round(sx - bw * 0.5, by, sx + bw * 0.5, by + 3.5, 1.5, [0.08, 0.06, 0.04, 0.68]);
                            if progress > 0.01 {
                                self.gfx.fill_round(sx - bw * 0.5, by, sx - bw * 0.5 + bw * progress, by + 3.5, 1.5, [0.35, 0.78, 1.0, 0.95]);
                            }
                            self.gfx.stroke_round(sx - bw * 0.5, by, sx + bw * 0.5, by + 3.5, 1.5, 0.8, [0.78, 0.64, 0.34, 0.75]);
                        } else if let Some(f) = hp_frac {
                            let bw = (r * 1.4).max(16.0);
                            let by = sy - r * 1.02;
                            self.gfx.fill_round(sx - bw * 0.5, by, sx + bw * 0.5, by + 3.5, 1.5, [0.08, 0.06, 0.04, 0.75]);
                            self.gfx.fill_round(sx - bw * 0.5, by, sx - bw * 0.5 + bw * f, by + 3.5, 1.5, [1.0 - f, 0.22 + 0.75 * f, 0.15, 0.95]);
                        }
                    }
                    WorldItem::Unit { sx, sy, kind, ue, pw, nc, carry, flip_x, hp_frac, hero, divine } => {
                        let is_hero = (hero as usize) < HEROES.len();
                        let scale = if is_hero {
                            if kind == UnitKind::Chariot { 0.88 } else { 0.78 }
                        } else if divine {
                            if kind == UnitKind::Chariot { 0.82 } else { 0.70 }
                        } else if kind == UnitKind::Chariot {
                            0.76
                        } else {
                            0.62
                        };
                        let max_r = if is_hero { 27.0 } else if divine { 24.0 } else { 22.0 };
                        let r = (z * scale * (1.0 + 0.04 * (pw - 1.0).min(4.0))).clamp(7.5, max_r);
                        let u_bmp = if is_hero {
                            self.art.hero_sprite(hero as usize, flip_x)
                        } else if divine {
                            self.art.divine_unit_sprite(kind, flip_x)
                        } else {
                            self.art.unit_sprite(kind, ue, flip_x)
                        };
                        if let Some(bmp) = u_bmp {
                            self.gfx.draw_bitmap(bmp, sx - r, sy - r * 1.35, sx + r, sy + r * 0.65);
                        }
                        // 足元右下の国色小紋章（上帝の軍は神々しい白金紋章）
                        let pr = (r * 0.19).clamp(2.2, 3.8);
                        let px0 = sx + r * 0.42;
                        let py0 = sy + r * 0.32;
                        self.gfx.fill_circle(px0, py0, pr + 0.9, [0.08, 0.06, 0.04, 0.92]);
                        if divine {
                            self.gfx.fill_circle(px0, py0, pr, [1.0, 0.92, 0.48, 1.0]);
                            self.gfx.stroke_circle(px0, py0, pr + 0.5, 0.9, [0.92, 0.98, 1.0, 0.95]);
                        } else {
                            self.gfx.fill_circle(px0, py0, pr, c4(nc, 1.0));
                            self.gfx.stroke_circle(px0, py0, pr + 0.4, 0.8, [0.95, 0.82, 0.42, 0.9]);
                        }
                        // 資源運搬中はミニ資源アイコンを左上に表示
                        if let Some(ck) = carry {
                            if z >= 11.0 {
                                let cr = (r * 0.38).clamp(3.8, 6.2);
                                let cx = sx - r * 0.55;
                                let cy = sy - r * 0.65;
                                self.gfx.fill_circle(cx, cy, cr + 0.8, [0.08, 0.06, 0.04, 0.85]);
                                if let Some(rbmp) = &self.art.res_icons[ck] {
                                    self.gfx.draw_bitmap(rbmp, cx - cr, cy - cr, cx + cr, cy + cr);
                                }
                            }
                        }
                        // 強化済みユニットは金の星（ランクピップ）を右下に表示
                        if pw > 1.01 && z >= 12.0 {
                            let stars = ((pw - 1.0) / 0.5).round().clamp(1.0, 6.0) as i32;
                            for s in 0..stars {
                                let px = sx + r * 0.44 - s as f32 * 3.8;
                                let py = sy + r * 0.48;
                                self.gfx.fill_circle(px, py, 2.3, [0.12, 0.08, 0.04, 0.95]);
                                self.gfx.fill_circle(px, py, 1.5, [1.0, 0.88, 0.32, 1.0]);
                            }
                        }
                        if let Some(f) = hp_frac {
                            let bw = (r * 1.4).max(11.0);
                            let by = sy - r * 1.40;
                            self.gfx.fill_rect(sx - bw * 0.5, by, sx + bw * 0.5, by + 2.8, [0.0, 0.0, 0.0, 0.8]);
                            self.gfx.fill_rect(sx - bw * 0.5, by, sx - bw * 0.5 + bw * f, by + 2.8, [1.0 - f, 0.25 + 0.7 * f, 0.15, 1.0]);
                        }
                    }
                }
            }
        }
        // 城邑名（Age of Empires 風の銘板バナー付き）
        let mut labels = Vec::new();
        for b in self.world.blds.iter() {
            if !b.alive || !b.kind.city() || !self.explored(b.center()) {
                continue;
            }
            let (sx, sy) = self.world_to_screen(b.center());
            if sx < -100.0 || sy < -50.0 || sx > w + 100.0 || sy > h + 50.0 {
                continue;
            }
            let n = &self.world.nations[b.nation as usize];
            let cap = b.kind == BKind::Capital;
            if z < 9.0 && !cap {
                continue;
            }
            let divine = self.world.descent.is_some() && b.nation as usize == 2;
            let name = if divine {
                if cap { t!("上帝・秦", "Deity · Qin", "上帝・秦").to_string() } else { t!("上帝・邑", "Deity · Town", "上帝・邑").to_string() }
            } else if cap {
                crate::lang::tx(n.name).to_string()
            } else {
                tf!("{}・邑", "{} town", "{}・邑", crate::lang::tx(n.name))
            };
            let col = if divine { [1.0, 0.88, 0.38] } else { n.color };
            labels.push((sx, sy, name, cap, col, divine));
        }
        for (sx, sy, name, cap, c, divine) in labels {
            let f = if cap && z >= 9.0 { Font::BigCenter } else { Font::NormalCenter };
            let off = if z >= 9.0 { 1.58 * z + 14.0 } else { 1.25 * z + 6.0 };
            let ty = sy - off - 14.0;
            if z >= 8.0 {
                let bw = if divine { 82.0 } else if cap { 64.0 } else { 72.0 };
                let bh = if cap { 24.0 } else { 20.0 };
                let bx0 = sx - bw * 0.5;
                let by0 = ty + (28.0 - bh) * 0.5;
                self.gfx.fill_round(bx0, by0, bx0 + bw, by0 + bh, 4.0, [0.08, 0.06, 0.04, 0.85]);
                self.gfx.stroke_round(bx0, by0, bx0 + bw, by0 + bh, 4.0, if divine { 1.6 } else { 1.2 }, [c[0] * 0.7 + 0.25, c[1] * 0.7 + 0.25, c[2] * 0.7 + 0.2, 0.95]);
            }
            let col = [c[0] * 0.45 + 0.55, c[1] * 0.45 + 0.55, c[2] * 0.45 + 0.55, 1.0];
            self.gfx.text_shadow(&name, sx - 80.0, ty, 160.0, 28.0, f, col);
        }
    }

    fn act_cmd_icon(&self, act: Act) -> Option<CmdIcon> {
        let era = self.world.era;
        match act {
            Act::Train(UnitKind::Worker) | Act::SelectAllWorkers | Act::SelectKind(UnitKind::Worker) | Act::FilterKind(UnitKind::Worker) | Act::ResumeBuild => Some(CmdIcon::Worker),
            Act::Train(UnitKind::Infantry) | Act::SelectKind(UnitKind::Infantry) | Act::FilterKind(UnitKind::Infantry) => Some(CmdIcon::Infantry),
            Act::Train(UnitKind::Archer) | Act::SelectKind(UnitKind::Archer) | Act::FilterKind(UnitKind::Archer) => Some(CmdIcon::Archer),
            Act::Train(UnitKind::Chariot) | Act::SelectKind(UnitKind::Chariot) | Act::FilterKind(UnitKind::Chariot) => {
                Some(if era == Era::Spring { CmdIcon::Chariot } else { CmdIcon::Cavalry })
            }
            Act::SelectAllMilitary => Some(CmdIcon::AllArmy),
            Act::WorkersAuto | Act::AllWorkersAuto => Some(CmdIcon::GatherAuto),
            Act::AllWorkersWood => Some(CmdIcon::GatherWood),
            Act::AllWorkersMine => Some(CmdIcon::GatherBronze),
            Act::Place(BKind::Farm) => Some(CmdIcon::Farm),
            Act::Place(BKind::Barracks) => Some(CmdIcon::Barracks),
            Act::Place(BKind::Town) => Some(CmdIcon::Town),
            Act::Place(BKind::Capital) => Some(CmdIcon::Capital),
            Act::Place(BKind::Fort) => Some(CmdIcon::Fort),
            Act::Upgrade | Act::AllWorkersUpgrade | Act::AllArmyUpgrade => Some(CmdIcon::Upgrade),
            Act::Convert => Some(CmdIcon::Convert),
            Act::Stop => Some(CmdIcon::Stop),
            Act::AttackMove | Act::AllArmyAttack => Some(CmdIcon::AttackMove),
            Act::ArmyDefend | Act::AllArmyDefend => Some(CmdIcon::Defend),
            Act::Demolish => Some(CmdIcon::Demolish),
            Act::Diplo => Some(CmdIcon::Diplo),
            Act::Help => Some(CmdIcon::Help),
            _ => None,
        }
    }

    fn add_button(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, label: &str, sub: &str, key: u32, enabled: bool, act: Act, tip: &str) {
        let (mx, my) = self.mouse;
        let hover = mx >= x0 && mx <= x1 && my >= y0 && my <= y1;
        let btn_bmp = if !enabled {
            self.art.btn_dis.as_ref()
        } else if hover {
            self.art.btn_hover.as_ref()
        } else {
            self.art.btn_norm.as_ref()
        };
        if let Some(bmp) = btn_bmp {
            self.gfx.draw_bitmap(bmp, x0, y0, x1, y1);
        } else {
            let bg = if !enabled {
                [0.18, 0.17, 0.15, 0.9]
            } else if hover {
                [0.45, 0.36, 0.20, 0.95]
            } else {
                [0.30, 0.24, 0.15, 0.95]
            };
            self.gfx.fill_round(x0, y0, x1, y1, 4.0, bg);
            self.gfx.stroke_round(x0, y0, x1, y1, 4.0, 1.0, [0.75, 0.62, 0.35, if enabled { 0.9 } else { 0.4 }]);
        }

        let bh = y1 - y0;
        let mut tx0 = x0;
        if let Some(ci) = self.act_cmd_icon(act) {
            if let Some(ibmp) = &self.art.cmd_icons[ci as usize] {
                if bh >= 36.0 && !(self.gfx.compact && y1 <= self.top_h()) {
                    let isz = (bh - 10.0).min(34.0);
                    let iy = y0 + (bh - isz) * 0.5;
                    self.gfx.draw_bitmap(ibmp, x0 + 3.0, iy, x0 + 3.0 + isz, iy + isz);
                    if !enabled {
                        self.gfx.fill_rect(x0 + 3.0, iy, x0 + 3.0 + isz, iy + isz, [0.05, 0.05, 0.05, 0.55]);
                    }
                    tx0 = x0 + isz + 4.0;
                } else if x1 - x0 >= 68.0 {
                    let isz = (bh - 6.0).max(14.0);
                    self.gfx.draw_bitmap(ibmp, x0 + 4.0, y0 + 3.0, x0 + 4.0 + isz, y0 + 3.0 + isz);
                    tx0 = x0 + isz + 4.0;
                }
            }
        }

        let tc = if enabled { [1.0, 0.95, 0.85, 1.0] } else { [0.55, 0.52, 0.48, 1.0] };
        let tw = (x1 - tx0).max(10.0);
        if sub.is_empty() {
            self.gfx.text_shadow(label, tx0, y0, tw, bh, Font::NormalCenter, tc);
        } else {
            self.gfx.text_shadow(label, tx0 + 2.0, y0 + 2.0, tw + 24.0, bh * 0.52, Font::Normal, tc);
            // 小見出しが入りきらなければ、アイコンの下まで左へ寄せる
            let sw = self.gfx.measure(sub, Font::Small);
            let sx = (x1 - 4.0 - sw).clamp(x0 + 4.0, tx0 + 2.0);
            self.gfx.text(sub, sx, y0 + bh * 0.53, x1 - sx, bh * 0.45, Font::Small, [tc[0] * 0.92, tc[1] * 0.86, tc[2] * 0.68, 1.0]);
        }
        if key >= 0x41 && key <= 0x5A {
            let ks = ((key as u8) as char).to_string();
            self.gfx.fill_round(x0 + 2.0, y0 + 2.0, x0 + 15.0, y0 + 15.0, 2.0, [0.08, 0.06, 0.04, 0.84]);
            self.gfx.text(&ks, x0 + 4.0, y0 + 1.0, 11.0, 14.0, Font::Small, [1.0, 0.88, 0.45, 0.98]);
        }
        if hover && !tip.is_empty() {
            self.notice = None;
        }
        self.buttons.push(Button { x0, y0, x1, y1, label: label.to_string(), key, enabled, act, tip: tip.to_string() });
    }

    fn cost_str(c: [f32; 3]) -> String {
        let cnt = c.iter().filter(|&&v| v > 0.0).count();
        let sep = if cnt >= 3 && crate::lang::lang() != crate::lang::Lang::En { "" } else { " " };
        let mut s = String::new();
        for k in 0..3 {
            if c[k] > 0.0 {
                if !s.is_empty() {
                    s.push_str(sep);
                }
                s.push_str(&format!("{}{}", res_short(k), c[k] as i32));
            }
        }
        s
    }

    fn render_title(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        // 国都マーカーの強調
        let hov = {
            let p = self.screen_to_world(self.mouse.0, self.mouse.1);
            let mut best: Option<(f32, usize)> = None;
            for (i, n) in self.world.nations.iter().enumerate() {
                if n.alive && n.home.dist(p) < 10.0 && best.map_or(true, |b| n.home.dist(p) < b.0) {
                    best = Some((n.home.dist(p), i));
                }
            }
            best.map(|b| b.1)
        };
        for (i, n) in self.world.nations.iter().enumerate() {
            if !n.alive {
                continue;
            }
            let (sx, sy) = self.world_to_screen(n.home);
            let seven = NATIONS[i].seven;
            if Some(i) == self.title_sel || Some(i) == hov {
                self.gfx.fill_circle(sx, sy, 15.0, [1.0, 0.92, 0.55, 0.38]);
                self.gfx.stroke_circle(sx, sy, 15.0, 2.0, [1.0, 0.88, 0.40, 0.9]);
            }
            if seven {
                self.gfx.fill_circle(sx, sy, 4.5, [1.0, 0.85, 0.3, 1.0]);
            }
        }
        let px = w - 380.0;
        self.art.draw_stone_panel(&self.gfx, px, 0.0, w, h);
        self.art.draw_corner_ornaments(&self.gfx, px + 4.0, 4.0, w - 4.0, h - 4.0, 18.0);
        // 英語の題字は長いので、扁額を広げた飾りを使う
        let wide = crate::lang::lang() == crate::lang::Lang::En;
        let crest = if wide { &self.art.title_crest_wide } else { &self.art.title_crest };
        if let Some(crest) = crest {
            self.gfx.draw_bitmap(crest, px + 20.0, 10.0, w - 20.0, 126.0);
        }
        let title = t!("春秋戦国", "Warring States", "春秋战国");
        let plaque = if wide { crate::art::TITLE_PLAQUE_W_WIDE } else { crate::art::TITLE_PLAQUE_W };
        // 扁額（両端に少し余白）に収まらなければ一回り小さい書体で書く
        let tf = if self.gfx.measure(title, Font::Title) <= plaque - 8.0 { Font::Title } else { Font::TitleMid };
        self.gfx.text_shadow(title, px, 19.0, 380.0, 56.0, tf, [1.0, 0.86, 0.42, 1.0]);
        self.gfx.text_shadow(t!("— 中華統一への道 —", "— The Road to Unification —", "— 中华统一之路 —"), px, 76.0, 380.0, 28.0, Font::BigCenter, [0.96, 0.90, 0.78, 1.0]);

        let compact = self.gfx.compact;
        if !compact || self.title_sel.is_none() {
        self.art.draw_inset_box(&self.gfx, px + 18.0, 132.0, w - 18.0, 306.0);
        self.gfx.text(
            t!("紀元前770年、周の東遷。数多の諸侯が並び立つ春秋の世から、七雄が争う戦国の世まで、時は全ての国に等しく流れる。\n\n地図上の国都をクリックして、率いる国を選べ。金の印は戦国の七雄（生産力が高い）。", "770 BC: the Zhou court moves east. From the Spring and Autumn period of countless lords to the Warring States of seven great powers, time flows equally for every state.\n\nClick a capital on the map to choose the state you will lead. Gold marks are the Seven Powers of the Warring States (high production).", "公元前770年，周室东迁。从诸侯林立的春秋之世，到七雄争霸的战国之世，时间对所有国家一视同仁。\n\n点击地图上的国都，选择你要率领的国家。金色标记为战国七雄（生产力高）。"),
            px + 28.0,
            140.0,
            324.0,
            162.0,
            Font::Normal,
            [0.90, 0.86, 0.78, 1.0],
        );
        }
        if let Some(i) = self.title_sel {
            let n = &self.world.nations[i];
            let d = &NATIONS[i];
            // スマートフォンでは説明文の欄を隠して、国の情報をその位置に詰める
            let oy = if compact { 132.0 - 316.0 } else { 0.0 };
            self.art.draw_inset_box(&self.gfx, px + 18.0, oy + 316.0, w - 18.0, oy + 522.0);
            self.art.draw_corner_ornaments(&self.gfx, px + 20.0, oy + 318.0, w - 20.0, oy + 520.0, 12.0);
            if let Some(cap_ic) = self.art.bld_icon(BKind::Capital) {
                self.gfx.draw_bitmap(cap_ic, px + 30.0, oy + 328.0, px + 78.0, oy + 376.0);
            }
            self.gfx.fill_rect(px + 86.0, oy + 336.0, px + 104.0, oy + 368.0, c4(n.color, 1.0));
            self.gfx.stroke_rect(px + 86.0, oy + 336.0, px + 104.0, oy + 368.0, 1.2, [0.92, 0.78, 0.40, 1.0]);
            self.gfx.text_shadow(crate::lang::tx(n.name), px + 112.0, oy + 332.0, 230.0, 40.0, Font::Big, [1.0, 0.95, 0.85, 1.0]);
            self.gfx.text(crate::lang::tx(d.desc), px + 32.0, oy + 382.0, 316.0, 44.0, Font::Normal, [0.88, 0.84, 0.76, 1.0]);
            let s = tf!("生産力：×{:.2}{}", "Production: ×{:.2}{}", "生产力：×{:.2}{}", d.bias, if d.seven { t!("（戦国七雄）", " (Seven Powers)", "（战国七雄）") } else { "" });
            self.gfx.text(&s, px + 32.0, oy + 426.0, 316.0, 20.0, Font::Normal, [0.98, 0.86, 0.48, 1.0]);
            let cities = self.world.blds.iter().filter(|b| b.alive && b.nation as usize == i && b.kind.city()).count();
            let s = tf!("初期城邑：{}　君主家：{}", "Cities: {}   House: {}", "初始城邑：{}　君主家：{}", cities, crate::lang::tx(INITIAL_HOUSE[i]));
            self.gfx.text(&s, px + 32.0, oy + 448.0, 316.0, 20.0, Font::Normal, [0.88, 0.84, 0.76, 1.0]);
            let diff = if d.seven {
                t!("難易度：易", "Difficulty: Easy", "难度：易")
            } else if d.bias >= 0.95 {
                t!("難易度：中", "Difficulty: Normal", "难度：中")
            } else {
                t!("難易度：難", "Difficulty: Hard", "难度：难")
            };
            self.gfx.text(diff, px + 32.0, oy + 470.0, 316.0, 20.0, Font::Normal, [0.88, 0.84, 0.76, 1.0]);
            // 史実に基づく国の特徴（説明はツールチップ）
            let trs: Vec<&Trait> = all_traits_of(i).collect();
            if !trs.is_empty() {
                let names: Vec<String> = trs
                    .iter()
                    .map(|t| if t.from_bc < START_BC as i32 { tf!("{}（前{}年〜）", "{} (from {} BC)", "{}（前{}年起）", crate::lang::tx(t.name), t.from_bc) } else if t.to_bc > END_BC as i32 { tf!("{}（〜前{}年）", "{} (until {} BC)", "{}（至前{}年）", crate::lang::tx(t.name), t.to_bc) } else { crate::lang::tx(t.name).to_string() })
                    .collect();
                let s = tf!("特徴：{}", "Traits: {}", "特点：{}", names.join(sep()));
                self.gfx.text(&s, px + 32.0, oy + 492.0, 316.0, 22.0, Font::Normal, [0.70, 0.92, 1.0, 1.0]);
                let tip: Vec<String> = trs.iter().map(|t| format!("【{}】{}", crate::lang::tx(t.name), crate::lang::tx(t.desc))).collect();
                self.buttons.push(Button {
                    x0: px + 32.0,
                    y0: oy + 490.0,
                    x1: w - 30.0,
                    y1: oy + 514.0,
                    label: String::new(),
                    key: 0,
                    enabled: true,
                    act: Act::Noop,
                    tip: tip.join("\n"),
                });
            }
            let sy = if compact { (oy + 528.0).min(h - 138.0) } else { 530.0 };
            self.add_button(px + 50.0, sy, w - 50.0, sy + if compact { 40.0 } else { 46.0 }, t!("この国で覇業を開始", "Begin your conquest", "以此国开始霸业"), "", 0, true, Act::Start, "");
        }
        if compact {
            self.render_title_buttons_compact(px);
            return;
        }
        let ty = (h - 128.0).max(592.0);
        // 記録から続ける（空きがあれば独立した行、狭ければ英霊モードの行と並べる）
        let cy = ty - 108.0;
        let heroic_x0 = if self.title_sel.is_some() && cy < 590.0 {
            let mid = px + 24.0 + (w - 48.0 - px) * 0.5;
            self.add_button(mid + 4.0, ty - 54.0, w - 24.0, ty - 12.0, t!("続きから F5", "Continue F5", "继续 F5"), "", 0, true, Act::SaveMenu, t!("記録した対局を再開する", "Resume a saved game", "继续已保存的对局"));
            Some(mid - 4.0)
        } else {
            self.add_button(px + 24.0, cy, w - 24.0, cy + 42.0, t!("続きから（記録を読み込む） F5", "Continue (load a save) F5", "继续（读取存档） F5"), "", 0, true, Act::SaveMenu, t!("記録した対局を再開する。書き出したファイルからも読み込める", "Resume a saved game. Exported files can be loaded too", "继续已保存的对局。也可读取导出的文件"));
            None
        };
        let hl = if self.heroic_opt { t!("英霊モード：オン", "Heroic Spirits: On", "英灵模式：开") } else { t!("英霊モード：オフ", "Heroic Spirits: Off", "英灵模式：关") };
        let hx1 = heroic_x0.unwrap_or(w - 24.0);
        self.add_button(px + 24.0, ty - 54.0, hx1, ty - 12.0, hl, "", 0, true, Act::ToggleHeroic, t!("三皇五帝から春秋戦国の英霊を、民を生贄にして時代と国を超えて呼べる（一国に一体）。英霊は上帝の軍を傷つけられ、上帝降臨から10年後の版図が上帝の秦より広ければ勝利。他国も召喚する", "Sacrifice workers to summon heroic spirits from the Three Sovereigns to the Warring States, across eras and states (one per state). Spirits can harm the Deity's army; if your territory is larger than the Deity's Qin 10 years after the Descent, you win. Other states summon too", "以民为祭品，跨越时代与国界召唤自三皇五帝至春秋战国的英灵（每国一位）。英灵能伤及上帝之军，上帝降临10年后若版图大于上帝之秦即获胜。他国也会召唤"));
        if self.heroic_opt {
            self.gfx.stroke_round(px + 24.0, ty - 54.0, hx1, ty - 12.0, 3.0, 2.0, [1.0, 0.85, 0.4, 1.0]);
        }
        self.add_button(px + 24.0, ty, w - 24.0, ty + 48.0, t!("はじめての方は：チュートリアル", "New here? Tutorial", "初次游玩：教程"), "", 0, true, Act::Tutorial, t!("晋の君主として、操作と遊び方を1段ずつ学ぶ", "Learn the controls and the game step by step as ruler of Jin", "以晋国君主身份逐步学习操作与玩法"));
        let ty = ty + 60.0;
        self.add_button(px + 24.0, ty, px + 70.0, ty + 40.0, "A−", "", 0, true, Act::Scale(-1), t!("文字と画面部品を小さくする（Ctrl＋−）", "Make text and interface smaller (Ctrl+−)", "缩小文字与界面（Ctrl＋−）"));
        self.add_button(px + 74.0, ty, px + 120.0, ty + 40.0, "A＋", "", 0, true, Act::Scale(1), t!("文字と画面部品を大きくする（Ctrl＋＋）", "Make text and interface larger (Ctrl++)", "放大文字与界面（Ctrl＋＋）"));
        let fl = if self.gfx.is_fullscreen() { t!("ウィンドウ F11", "Window F11", "窗口 F11") } else { t!("全画面 F11", "Fullscreen F11", "全屏 F11") };
        // 表示言語（いまの言語の名前を出し、押すと次の言語へ）
        let ll = format!("{} ▸", crate::lang::lang().native());
        self.add_button(px + 124.0, ty, px + 222.0, ty + 40.0, &ll, "", 0, true, Act::Lang, "表示言語を切り替える / Change language / 切换语言（日本語 → English → 中文）");
        self.add_button(px + 226.0, ty, w - 24.0, ty + 40.0, fl, "", 0, true, Act::Fullscreen, t!("全画面表示を切り替える", "Toggle fullscreen", "切换全屏显示"));
        // タイトル画面のツールチップ
        let (mx, my) = self.mouse;
        let tip = self.buttons.iter().find(|b| mx >= b.x0 && mx <= b.x1 && my >= b.y0 && my <= b.y1 && !b.tip.is_empty()).map(|b| b.tip.clone());
        if let Some(t) = tip.filter(|_| !self.show_save && !(self.touch && self.clock > self.tip_until)) {
            // 説明の行数に合わせて上へ伸ばす
            let x0 = px + 24.0;
            let th = self.gfx.measure_h(&t, Font::Small, w - x0 - 40.0) + 10.0;
            // 英霊モードの切り替えボタンの上に出す
            let y1 = ty - 124.0;
            self.art.draw_inset_box(&self.gfx, x0, y1 - th - 4.0, w - 24.0, y1);
            self.gfx.text(&t, x0 + 8.0, y1 - th, w - x0 - 40.0, th, Font::Small, [0.95, 0.92, 0.85, 1.0]);
        }
    }

    fn render_hud(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pl = self.world.player;
        if self.gfx.compact {
            self.render_topbar_compact();
        } else {
        // ---- 上部バー（Age of Empires 風石彫パネル＋資源アイコン）
        self.art.draw_stone_panel(&self.gfx, 0.0, 0.0, w, self.top_h());
        let n = &self.world.nations[pl];
        let mut x = 8.0;
        let res = n.res;
        let inc = n.income;
        let upkeep = n.upkeep;
        let res_icons = [ResIcon::Food, ResIcon::Wood, ResIcon::Bronze];
        let (n_workers, w_cap, n_soldiers, m_cap) = (n.workers, n.worker_cap(), n.soldiers, n.mil_cap());
        for k in 0..3 {
            if let Some(ic) = &self.art.res_icons[res_icons[k] as usize] {
                self.gfx.draw_bitmap(ic, x, 4.0, x + 28.0, 32.0);
            }
            let rate = if k == 0 { inc[k] - upkeep } else { inc[k] };
            let s = format!("{}", res[k] as i32);
            self.gfx.text_shadow(&s, x + 31.0, 7.0, 72.0, 22.0, Font::Big, RES_COLORS[k]);
            let rs = tf!("{:+.1}/秒", "{:+.1}/s", "{:+.1}/秒", rate);
            let rc = if rate < 0.0 { [1.0, 0.42, 0.32, 1.0] } else { [0.80, 0.78, 0.70, 1.0] };
            self.gfx.text(&rs, x + 88.0, 11.0, 56.0, 18.0, Font::Small, rc);
            x += 148.0;
        }
        // 民全体・全軍をワンクリック選択できるトップバーアイコンボタン
        let (mx, my) = self.mouse;
        let w_hov = mx >= x && mx <= x + 88.0 && my >= 4.0 && my <= 32.0;
        self.gfx.fill_round(x, 4.0, x + 88.0, 32.0, 4.0, if w_hov { [0.35, 0.28, 0.16, 0.9] } else { [0.16, 0.13, 0.09, 0.75] });
        self.gfx.stroke_round(x, 4.0, x + 88.0, 32.0, 4.0, 1.0, [0.75, 0.62, 0.34, 0.85]);
        if let Some(ic) = &self.art.res_icons[ResIcon::Worker as usize] {
            self.gfx.draw_bitmap(ic, x + 3.0, 5.0, x + 29.0, 31.0);
        }
        let s = format!("{}/{}", n_workers, w_cap);
        self.gfx.text_shadow(&s, x + 32.0, 8.0, 54.0, 22.0, Font::Normal, [0.95, 0.92, 0.85, 1.0]);
        self.buttons.push(Button {
            x0: x,
            y0: 4.0,
            x1: x + 88.0,
            y1: 32.0,
            label: String::new(),
            key: 0,
            enabled: true,
            act: Act::SelectAllWorkers,
            tip: t!("民（生産ユニット）全体を選択する", "Select all workers", "选择全部民（生产单位）").to_string(),
        });
        x += 94.0;

        let s_hov = mx >= x && mx <= x + 88.0 && my >= 4.0 && my <= 32.0;
        self.gfx.fill_round(x, 4.0, x + 88.0, 32.0, 4.0, if s_hov { [0.35, 0.28, 0.16, 0.9] } else { [0.16, 0.13, 0.09, 0.75] });
        self.gfx.stroke_round(x, 4.0, x + 88.0, 32.0, 4.0, 1.0, [0.75, 0.62, 0.34, 0.85]);
        if let Some(ic) = &self.art.res_icons[ResIcon::Soldier as usize] {
            self.gfx.draw_bitmap(ic, x + 3.0, 5.0, x + 29.0, 31.0);
        }
        let s = format!("{}/{}", n_soldiers, m_cap);
        self.gfx.text_shadow(&s, x + 32.0, 8.0, 54.0, 22.0, Font::Normal, [0.95, 0.92, 0.85, 1.0]);
        self.buttons.push(Button {
            x0: x,
            y0: 4.0,
            x1: x + 88.0,
            y1: 32.0,
            label: String::new(),
            key: 0,
            enabled: true,
            act: Act::SelectAllMilitary,
            tip: t!("軍事ユニット全体（全軍）を選択する", "Select all soldiers", "选择全部军事单位（全军）").to_string(),
        });
        x += 96.0;

        if let Some(ic) = &self.art.res_icons[ResIcon::Upkeep as usize] {
            self.gfx.draw_bitmap(ic, x, 5.0, x + 26.0, 31.0);
        }
        let s = tf!("-{:.1}/秒", "-{:.1}/s", "-{:.1}/秒", upkeep);
        self.gfx.text_shadow(&s, x + 28.0, 8.0, 88.0, 22.0, Font::Normal, [0.98, 0.68, 0.52, 1.0]);
        x += 112.0;
        // 銭（商人からの税収）
        let (coin, coin_inc) = (self.world.nations[pl].coin, self.world.nations[pl].coin_income);
        self.gfx.fill_circle(x + 13.0, 18.0, 11.0, [0.86, 0.70, 0.30, 1.0]);
        self.gfx.stroke_circle(x + 13.0, 18.0, 11.0, 1.2, [0.45, 0.32, 0.12, 1.0]);
        self.gfx.fill_rect(x + 9.0, 14.0, x + 17.0, 22.0, [0.30, 0.22, 0.10, 1.0]);
        let s = format!("{}", coin as i32);
        self.gfx.text_shadow(&s, x + 28.0, 7.0, 64.0, 22.0, Font::Big, [1.0, 0.86, 0.45, 1.0]);
        let s = tf!("{:+.1}/秒", "{:+.1}/s", "{:+.1}/秒", coin_inc);
        self.gfx.text(&s, x + 80.0, 11.0, 56.0, 18.0, Font::Small, [0.80, 0.78, 0.70, 1.0]);
        self.buttons.push(Button {
            x0: x,
            y0: 4.0,
            x1: x + 136.0,
            y1: 32.0,
            label: String::new(),
            key: 0,
            enabled: true,
            act: Act::Market,
            tip: t!("銭：領内を通る商人・西域の商人・部族の隊商からの税収。交易画面（F4）で資源の購入や傭兵の雇用に使う", "Coin: taxes from merchants, Western merchants and tribal caravans passing through your land. Spend it on the Trade screen (F4) to buy resources or hire mercenaries", "钱：途经领内的商人、西域商人、部族商队所缴的税。可在贸易画面（F4）购买资源或雇佣佣兵").to_string(),
        });

        // 速度・メニュー
        let bx = w - 424.0;
        let ex = (bx - 248.0).max(x + 150.0);
        // 幅に収まるいちばん詳しい書き方を選ぶ（言語で文字幅が違うので測って決める）
        let yb = self.world.year_bc();
        let spring = self.world.era == Era::Spring;
        let long = tf!("紀元前 {}年　{}", "{} BC  {}", "公元前 {}年　{}", yb, if spring { t!("春秋時代", "Spring & Autumn", "春秋时代") } else { t!("戦国時代", "Warring States", "战国时代") });
        let mid = tf!("前{}年 {}", "{} BC {}", "前{}年 {}", yb, if spring { t!("春秋", "S&A", "春秋") } else { t!("戦国", "WS", "战国") });
        let short = tf!("前{}年", "{} BC", "前{}年", yb);
        let room = bx - ex - 6.0;
        let (s, ef) = [(long, Font::Big), (mid, Font::Big), (short.clone(), Font::Big), (short.clone(), Font::Normal), (short, Font::Small)]
            .into_iter()
            .find(|(s, f)| self.gfx.measure(s, *f) <= room)
            .unwrap_or((tf!("前{}", "{}BC", "前{}", yb), Font::Small));
        if let Some(ic) = &self.art.res_icons[ResIcon::Era as usize] {
            self.gfx.draw_bitmap(ic, ex - 22.0, 8.0, ex - 2.0, 28.0);
        }
        self.gfx.text_shadow(&s, ex, if ef == Font::Big { 7.0 } else { 9.0 }, (bx - ex - 4.0).max(60.0), 24.0, ef, [1.0, 0.9, 0.6, 1.0]);
        let paused = self.paused;
        let sp = self.speed;
        self.add_button(bx, 5.0, bx + 36.0, 31.0, if paused { "▶" } else { "❚❚" }, "", 0, true, Act::Speed(0), "");
        for (k, s) in [1u32, 2, 4, 8].iter().enumerate() {
            let x0 = bx + 39.0 + k as f32 * 35.0;
            let lbl = format!("×{}", s);
            self.add_button(x0, 5.0, x0 + 33.0, 31.0, &lbl, "", 0, true, Act::Speed(*s), "");
            if sp == *s && !paused {
                self.gfx.stroke_round(x0, 5.0, x0 + 33.0, 31.0, 3.0, 2.0, [1.0, 0.9, 0.4, 1.0]);
            }
        }
        self.add_button(w - 242.0, 5.0, w - 166.0, 31.0, t!("交易 F4", "Trade F4", "贸易 F4"), "", 0, true, Act::Market, t!("商人の税・市場・山岳民と遊牧民・傭兵・西域", "Merchant taxes, market, hill and steppe peoples, mercenaries, the Western Regions", "商人税、市场、山地与游牧民、佣兵、西域"));
        self.add_button(w - 162.0, 5.0, w - 84.0, 31.0, t!("外交 F2", "Diplo F2", "外交 F2"), "", 0, true, Act::Diplo, "");
        self.add_button(w - 80.0, 5.0, w - 4.0, 31.0, t!("ヘルプ F1", "Help F1", "帮助 F1"), "", 0, true, Act::Help, "");
        // 記録（上部バーに余地が無いので、その下の右端に置く）
        self.add_button(w - 80.0, self.top_h() + 4.0, w - 4.0, self.top_h() + 30.0, t!("記録 F5", "Save F5", "存档 F5"), "", 0, true, Act::SaveMenu, t!("記録（セーブ・ロード）。10年ごとに自動でも記録する", "Save and load. The game also saves automatically every 10 years", "保存与读取。每10年也会自动保存"));
        }
        let paused = self.paused;
        if paused {
            self.gfx.text_shadow(t!("一時停止中（Space）", "Paused (Space)", "已暂停（Space）"), w / 2.0 - 120.0, self.top_h() + 60.0, 240.0, 30.0, Font::BigCenter, [1.0, 1.0, 0.8, 1.0]);
        } else if let Some(left) = self.world.descent_left() {
            let s = tf!("上帝降臨：上帝の軍が天下を統一するまで あと{}年", "Descent of the Deity: {} years until the Deity's army unifies the realm", "上帝降临：距上帝之军统一天下还有{}年", left.ceil() as i32);
            self.gfx.text_shadow(&s, w / 2.0 - 320.0, self.top_h() + 60.0, 640.0, 30.0, Font::BigCenter, [1.0, 0.75, 0.35, 1.0]);
        } else if self.cmd_hold() {
            self.gfx.text_shadow(t!("指揮中（時は止まっている。命令を出すか選択を解くと進む）", "Commanding: time stopped (give an order or deselect)", "指挥中（时间已停止。下达命令或取消选择后继续）"), w / 2.0 - 320.0, self.top_h() + 60.0, 640.0, 30.0, Font::BigCenter, [0.8, 0.95, 1.0, 1.0]);
        }

        // ---- 連合情報
        let mut cy = self.top_h() + 6.0;
        let years = self.world.years;
        let coal: Vec<String> = self
            .world
            .coalitions
            .iter()
            .map(|c| {
                let k = if c.kind == CoalKind::Hegemon { t!("覇者の呼びかけ", "Hegemon's Call", "霸主号召") } else { t!("合従軍", "Grand Alliance", "合纵军") };
                let me = if c.target == pl {
                    t!("（標的：自国！）", " (target: YOU!)", "（目标：我国！）")
                } else if c.members.contains(&pl) {
                    t!("（参加中）", " (you joined)", "（参与中）")
                } else {
                    ""
                };
                tf!("【{}】{}討伐 盟主{} {}国 残り{}年{}", "[{}] vs {}, led by {}, {} states, {} yrs{}", "【{}】讨伐{} 盟主{} {}国 剩余{}年{}", k, crate::lang::tx(self.world.nations[c.target].name), crate::lang::tx(self.world.nations[c.leader].name), c.members.len(), (c.end - years).ceil() as i32, me)
            })
            .collect();
        for s in coal {
            self.gfx.fill_round(w - 472.0, cy, w - 6.0, cy + 23.0, 3.0, [0.38, 0.10, 0.08, 0.90]);
            self.gfx.stroke_round(w - 472.0, cy, w - 6.0, cy + 23.0, 3.0, 1.2, [0.92, 0.72, 0.34, 0.9]);
            self.gfx.text_shadow(&s, w - 464.0, cy + 2.0, 460.0, 20.0, Font::Normal, [1.0, 0.88, 0.52, 1.0]);
            cy += 25.0;
        }
        // 覇者の警戒度
        if self.world.era == Era::Spring {
            let ag = self.world.nations[pl].aggression;
            if ag > 20.0 {
                let s = tf!("諸侯の警戒 {:.0}/100", "Lords' alarm {:.0}/100", "诸侯警戒 {:.0}/100", ag);
                let col = if ag > 75.0 { [1.0, 0.4, 0.3, 1.0] } else { [0.95, 0.8, 0.5, 1.0] };
                self.gfx.text_shadow(&s, w - 220.0, cy + 2.0, 214.0, 20.0, Font::Normal, col);
            }
        }

        // ---- メッセージ
        let now = self.world.time;
        let mut y = self.top_h() + 8.0;
        let msgs: Vec<(String, [f32; 3], f32)> = self.world.msgs.iter().rev().take(if self.gfx.compact { 4 } else { 7 }).filter(|m| now - m.time < 25.0).map(|m| (m.text.clone(), m.color, now - m.time)).collect();
        for (t, c, age) in msgs.into_iter().rev() {
            let a = if age > 20.0 { (25.0 - age) / 5.0 } else { 1.0 };
            let mw = if self.gfx.compact { w * 0.5 } else { w * 0.55 };
            let mh = self.gfx.measure_h(&t, Font::Normal, mw).clamp(20.0, 40.0);
            self.gfx.text_shadow(&t, 10.0, y, mw, mh, Font::Normal, [c[0], c[1], c[2], a]);
            y += mh;
        }

        // ---- イベントバナー
        if let Some(b) = &self.world.banner {
            let a = b.t.min(1.0);
            let (title, text) = (b.title.clone(), b.text.clone());
            let bw = 760.0f32.min(w - 40.0);
            let x0 = (w - bw) / 2.0;
            let y0 = self.top_h() + 90.0;
            let th = self.gfx.measure_h(&text, Font::NormalCenter, bw - 40.0).max(24.0);
            let bh = 66.0 + th + 14.0;
            self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + bw, y0 + bh);
            self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + bw - 3.0, y0 + bh - 3.0, 14.0);
            self.gfx.text_shadow(&title, x0, y0 + 8.0, bw, 44.0, Font::Title, [1.0, 0.85, 0.4, a]);
            self.gfx.text_shadow(&text, x0 + 20.0, y0 + 58.0, bw - 40.0, th + 6.0, Font::NormalCenter, [1.0, 0.95, 0.85, a]);
        }

        // ---- 下部パネル（Age of Empires 風石彫レリーフ＆3区画フレーム）
        let py = h - BOT_H;
        self.art.draw_stone_panel(&self.gfx, 0.0, py, w, h);

        // ミニマップ枠
        let (mx0, my0, mx1, my1) = self.mm_rect();
        self.gfx.fill_rect(mx0 - 2.0, my0 - 2.0, mx1 + 2.0, my1 + 2.0, [0.05, 0.04, 0.03, 1.0]);
        if let Some(b) = &self.minimap_bmp {
            self.gfx.draw_bitmap(b, mx0, my0, mx1, my1);
        }
        let sxs = MM_W / MAP_W as f32;
        let sys = MM_H / MAP_H as f32;
        let dots: Vec<(f32, f32, [f32; 3], bool)> = self
            .world
            .units
            .iter()
            .filter(|u| u.alive && u.kind.military() && (u.nation as usize == pl || self.seen(u.pos)))
            .map(|u| (mx0 + u.pos.x * sxs, my0 + u.pos.y * sys, self.world.nations[u.nation as usize].color, u.nation as usize == pl))
            .collect();
        let cdots: Vec<(f32, f32, [f32; 4])> = self
            .world
            .trade
            .caravans
            .iter()
            .filter(|c| self.seen(c.pos))
            .map(|c| (mx0 + c.pos.x * sxs, my0 + c.pos.y * sys, crate::game::market::caravan_color(c.kind)))
            .collect();
        for (x, y, c) in cdots {
            self.gfx.fill_rect(x - 1.0, y - 1.0, x + 1.0, y + 1.0, c);
        }
        for (x, y, c, mine) in dots {
            let s = if mine { 1.6 } else { 1.0 };
            self.gfx.fill_rect(x - s, y - s, x + s, y + s, if mine { [1.0, 1.0, 1.0, 1.0] } else { c4(darker(c, 0.6), 1.0) });
        }
        let a = self.screen_to_world(0.0, self.top_h());
        let b = self.screen_to_world(w, h - BOT_H);
        self.gfx.stroke_rect(
            (mx0 + a.x * sxs).max(mx0),
            (my0 + a.y * sys).max(my0),
            (mx0 + b.x * sxs).min(mx1),
            (my0 + b.y * sys).min(my1),
            1.2,
            [1.0, 0.96, 0.80, 0.92],
        );
        self.gfx.stroke_rect(mx0, my0, mx1, my1, 2.0, [0.78, 0.64, 0.34, 1.0]);
        self.art.draw_corner_ornaments(&self.gfx, mx0, my0, mx1, my1, 10.0);

        // 中央の情報欄ボックス
        let ix = MM_W + 12.0;
        let iw = w - ix - 394.0;
        self.art.draw_inset_box(&self.gfx, ix, py + 6.0, ix + iw, h - 4.0);
        self.art.draw_corner_ornaments(&self.gfx, ix + 2.0, py + 8.0, ix + iw - 2.0, h - 6.0, 11.0);
        self.render_info(ix + 12.0, py + 12.0, iw - 24.0);
        // 英霊モード：民を選んでいれば英霊召喚ボタン
        if self.world.heroic && self.own_sel().iter().any(|&h| self.world.unit(h).map_or(false, |u| u.kind == UnitKind::Worker)) {
            let tip = if self.world.hero_of(pl).is_some() { t!("英霊は一国に一体だけ。いまの英霊が倒れれば、また呼べる", "Only one spirit per state. If yours falls, you can summon again", "每国只能有一位英灵。现有英灵倒下后可再次召唤") } else { t!("選んだ民を生贄に捧げ、三皇五帝から春秋戦国の英霊を呼ぶ（英霊の能力＝生贄の能力の合計×1.1〜3.0）", "Sacrifice the selected workers to summon a spirit from the Three Sovereigns to the Warring States (spirit power = total sacrificed power × 1.1–3.0)", "献祭所选的民，召唤自三皇五帝至春秋战国的英灵（英灵能力＝祭品能力总和×1.1〜3.0）") };
            self.add_button(ix + iw - 176.0, h - 58.0, ix + iw - 14.0, h - 14.0, t!("英霊召喚", "Summon Spirit", "召唤英灵"), t!("民を生贄に", "sacrifice workers", "献祭民"), 'H' as u32, self.world.hero_of(pl).is_none(), Act::Summon, tip);
        }

        // スマートフォン：選択や建設場所選びを指で解く ✕
        if self.gfx.compact && (self.mode != Mode::Normal || !self.sel.is_empty() || self.sel_b.is_some()) {
            let tip = if self.mode != Mode::Normal { t!("取り消す", "Cancel", "取消") } else { t!("選択を解く", "Deselect", "取消选择") };
            self.add_button(ix + iw - 46.0, py + 10.0, ix + iw - 8.0, py + 48.0, "✕", "", 0, true, Act::Cancel, tip);
        }

        // 右側のコマンド欄ボックス
        let cx0 = w - 388.0;
        self.art.draw_inset_box(&self.gfx, cx0, py + 6.0, w - 4.0, h - 4.0);
        self.art.draw_corner_ornaments(&self.gfx, cx0 + 2.0, py + 8.0, w - 6.0, h - 6.0, 11.0);
        if !self.show_diplo && !self.show_help && !self.show_chron && !self.show_market && !self.show_summon && !self.show_save {
            self.render_commands(w - 382.0, py + 12.0);
        }

        if let Some((t, life)) = &self.notice {
            let a = life.min(1.0);
            let t = t.clone();
            self.gfx.text_shadow(&t, w / 2.0 - 300.0, py - 34.0, 600.0, 26.0, Font::BigCenter, [1.0, 0.85, 0.5, a]);
        }
    }

    /// ボタンのツールチップ（すべての画面部品の上に描く。上端で切れるならカーソルの下へ）
    fn render_tooltip(&mut self) {
        if self.touch && self.clock > self.tip_until {
            return;
        }
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let (mx, my) = self.mouse;
        // 後から置いたボタン（上に重なる画面）を優先
        let mut tip = None;
        for b in self.buttons.iter().rev() {
            if mx >= b.x0 && mx <= b.x1 && my >= b.y0 && my <= b.y1 {
                if b.label == BLOCK_LABEL {
                    break;
                }
                if !b.tip.is_empty() {
                    tip = Some(b.tip.clone());
                    break;
                }
            }
        }
        let Some(t) = tip else { return };
        let tw = 340.0f32.min(w - 8.0);
        let th = self.gfx.measure_h(&t, Font::Small, tw - 16.0) + 14.0;
        let x0 = (mx - tw * 0.5).clamp(4.0, (w - tw - 4.0).max(4.0));
        let y0 = if my - th - 10.0 >= 2.0 { my - th - 10.0 } else { (my + 24.0).min(h - th - 2.0) };
        self.art.draw_inset_box(&self.gfx, x0, y0, x0 + tw, y0 + th);
        self.gfx.text(&t, x0 + 8.0, y0 + 6.0, tw - 16.0, th - 8.0, Font::Small, [0.95, 0.92, 0.85, 1.0]);
    }

    fn order_text(&self, u: &Unit) -> &'static str {
        match u.order {
            Order::Idle => t!("待機中", "Idle", "待命"),
            Order::Move(_) => t!("移動中", "Moving", "移动中"),
            Order::AttackMove(_) => t!("進軍中", "Advancing", "进军中"),
            Order::Attack(_) => t!("交戦中", "Fighting", "交战中"),
            Order::Gather { src, phase } => match (src.res(), phase) {
                (_, 2) => t!("運搬中", "Carrying", "搬运中"),
                (0, _) => t!("農耕中", "Farming", "耕作中"),
                (1, _) => t!("伐採中", "Cutting wood", "伐木中"),
                _ => t!("採掘中", "Mining", "采矿中"),
            },
            Order::Build(_) => t!("建設中", "Building", "建造中"),
        }
    }

    fn render_info(&mut self, x: f32, y: f32, wd: f32) {
        let pl = self.world.player;
        let era = self.world.era;
        let cap = self.world.power_cap();
        let tc = [0.95, 0.92, 0.85, 1.0];
        if self.sel.len() == 1 {
            let u = self.world.unit(self.sel[0]).unwrap();
            let n = &self.world.nations[u.nation as usize];
            let nc = n.color;
            // Age of Empires 風の大型全身ユニット肖像カード＋体力バー
            let px0 = x + 2.0;
            let py0 = y + 2.0;
            let psz = 88.0;
            self.art.draw_inset_box(&self.gfx, px0 - 2.0, py0 - 2.0, px0 + psz + 2.0, py0 + psz + 2.0);
            self.gfx.fill_circle(px0 + psz * 0.5, py0 + psz * 0.74, psz * 0.34, c4(nc, 0.42));
            self.gfx.stroke_circle(px0 + psz * 0.5, py0 + psz * 0.74, psz * 0.34, 1.5, c4(nc, 0.9));
            let sprite_bmp = if (u.hero as usize) < HEROES.len() {
                self.art.hero_sprite(u.hero as usize, false)
            } else if u.divine {
                self.art.divine_unit_sprite(u.kind, false).or_else(|| self.art.unit_sprite(u.kind, era, false))
            } else {
                self.art.unit_sprite(u.kind, era, false)
            };
            if let Some(bmp) = sprite_bmp {
                self.gfx.draw_bitmap(bmp, px0 + 2.0, py0 + 2.0, px0 + psz - 2.0, py0 + psz - 2.0);
            }
            // 肖像下のHPバー
            let f = (u.hp / u.max_hp()).clamp(0.0, 1.0);
            self.gfx.fill_rect(px0, py0 + psz + 4.0, px0 + psz, py0 + psz + 11.0, [0.08, 0.06, 0.05, 0.95]);
            self.gfx.fill_rect(px0 + 1.0, py0 + psz + 5.0, px0 + 1.0 + (psz - 2.0) * f, py0 + psz + 10.0, [1.0 - f, 0.25 + 0.7 * f, 0.15, 1.0]);
            self.gfx.stroke_rect(px0, py0 + psz + 4.0, px0 + psz, py0 + psz + 11.0, 1.0, [0.75, 0.62, 0.34, 0.9]);

            let tx = x + 102.0;
            let tw = (wd - 102.0).max(100.0);
            let title = if (u.hero as usize) < HEROES.len() {
                let d = &HEROES[u.hero as usize];
                tf!("英霊・{}（{}）【{}】", "Spirit: {} ({}) [{}]", "英灵・{}（{}）【{}】", crate::lang::tx(d.name), crate::lang::tx(d.era), crate::lang::tx(n.name))
            } else if u.divine {
                tf!("上帝の軍・{}　【{}】", "Deity's {}  [{}]", "上帝之军・{}　【{}】", u.kind.name(era), crate::lang::tx(n.name))
            } else if (u.tribe as usize) < TRIBES.len() {
                let td = &TRIBES[u.tribe as usize];
                let ue = if td.nomad { Era::Warring } else { era };
                if u.merc_until > 0.0 {
                    tf!("{}の傭兵（{}）【{}】残り{}年", "{} mercenary ({}) [{}] {} yrs left", "{}佣兵（{}）【{}】剩余{}年", crate::lang::tx(td.name), u.kind.name(ue), crate::lang::tx(n.name), (u.merc_until - self.world.years).ceil() as i32)
                } else {
                    tf!("{}の{}", "{} {}", "{}{}", crate::lang::tx(td.name), u.kind.name(ue))
                }
            } else {
                format!("{}　【{}】", u.kind.name(era), crate::lang::tx(n.name))
            };
            let lines = tf!(
                "兵力 {:.0}/{:.0}　　能力 {:.1}（時代上限 {:.1}）\n状態：{}{}",
                "HP {:.0}/{:.0}    Power {:.1} (era limit {:.1})\nStatus: {}{}",
                "兵力 {:.0}/{:.0}　　能力 {:.1}（时代上限 {:.1}）\n状态：{}{}",
                u.hp.max(0.0),
                u.max_hp(),
                u.power,
                cap,
                self.order_text(u),
                if u.carry > 0.5 { tf!("（{} {:.0}）", " ({} {:.0})", "（{} {:.0}）", res_name(u.carry_kind), u.carry) } else { String::new() }
            );
            let desc = if (u.hero as usize) < HEROES.len() {
                crate::lang::tx(HEROES[u.hero as usize].desc)
            } else if u.divine {
                t!("天より降りた上帝の不死身の神兵。通常の兵では傷つけられず、英霊のみがこれを討てる。", "An immortal divine soldier sent down by the Supreme Deity. Ordinary troops cannot hurt it; only heroic spirits can.", "从天而降的上帝不死神兵。普通士兵无法伤其分毫，唯有英灵能将其击败。")
            } else {
                match u.kind {
                    UnitKind::Worker => t!("生産ユニット。田・森・銅山で資源を集め、建物を建てる。", "Worker. Gathers resources at fields, forests and mines, and builds.", "生产单位。在田、森林、铜矿采集资源并建造建筑。"),
                    UnitKind::Infantry => t!("戈を持つ歩兵。戦車・騎兵に強い。建物攻撃に優れる。", "Infantry with dagger-axes. Strong against chariots and cavalry, good at attacking buildings.", "手持戈的步兵。克制战车与骑兵，擅长攻击建筑。"),
                    UnitKind::Archer => t!("遠距離から射撃する。歩兵に強いが戦車・騎兵に弱い。", "Shoots from range. Strong against infantry, weak against chariots and cavalry.", "远程射击。克制步兵，惧战车与骑兵。"),
                    UnitKind::Chariot => t!("高速の突撃兵。弩兵と民に強いが、戈兵に弱い。", "Fast shock troops. Strong against crossbowmen and workers, weak against halberdiers.", "高速突击兵。克制弩兵与民，惧戈兵。"),
                }
            };
            self.gfx.text_shadow(&title, tx, y + 2.0, tw, 28.0, Font::Big, tc);
            self.gfx.text(&lines, tx, y + 34.0, tw, 52.0, Font::Normal, tc);
            self.gfx.text(desc, x + 2.0, y + 106.0, wd, 38.0, Font::Small, [0.80, 0.76, 0.68, 1.0]);
        } else if self.sel.len() > 1 {
            let mut cnt = [0usize; 4];
            let mut pw = 0.0;
            let mut hp = 0.0;
            let mut mhp = 0.0;
            for &h in self.sel.iter() {
                let u = self.world.unit(h).unwrap();
                cnt[u.kind as usize] += 1;
                pw += u.power;
                hp += u.hp;
                mhp += u.max_hp();
            }
            let title = tf!("{} 部隊を編制選択中", "{} units selected", "已选择{}个单位", self.sel.len());
            self.gfx.text_shadow(&title, x, y, wd, 26.0, Font::Big, tc);
            let kinds = [UnitKind::Worker, UnitKind::Infantry, UnitKind::Archer, UnitKind::Chariot];
            let (mx, my) = self.mouse;
            let mut ix = x;
            for k in 0..4 {
                if cnt[k] > 0 {
                    let bx0 = ix;
                    let by0 = y + 28.0;
                    let bx1 = ix + 86.0;
                    let by1 = y + 72.0;
                    let hov = mx >= bx0 && mx <= bx1 && my >= by0 && my <= by1;
                    self.gfx.fill_round(bx0, by0, bx1, by1, 4.0, if hov { [0.35, 0.28, 0.16, 0.95] } else { [0.16, 0.12, 0.08, 0.9] });
                    self.gfx.stroke_round(bx0, by0, bx1, by1, 4.0, 1.2, [0.78, 0.64, 0.34, 0.9]);
                    if let Some(bmp) = self.art.unit_icon(kinds[k], era) {
                        self.gfx.draw_bitmap(bmp, bx0 + 3.0, by0 + 3.0, bx0 + 41.0, by1 - 3.0);
                    }
                    let ks = format!("×{}", cnt[k]);
                    self.gfx.text_shadow(&ks, bx0 + 44.0, by0 + 10.0, 40.0, 24.0, Font::Big, tc);
                    self.buttons.push(Button {
                        x0: bx0,
                        y0: by0,
                        x1: bx1,
                        y1: by1,
                        label: String::new(),
                        key: 0,
                        enabled: true,
                        act: Act::FilterKind(kinds[k]),
                        tip: tf!("{}（{}体）：クリックでこの兵種のみに絞り込む", "{} ({}): click to keep only this type", "{}（{}个）：点击只保留该兵种", kinds[k].name(era), cnt[k]),
                    });
                    ix += 94.0;
                }
            }
            let s = tf!("総能力 {:.1}（平均 {:.2}）　　総兵力 {:.0}/{:.0}", "Total power {:.1} (avg {:.2})    Total HP {:.0}/{:.0}", "总能力 {:.1}（平均 {:.2}）　　总兵力 {:.0}/{:.0}", pw, pw / self.sel.len() as f32, hp, mhp);
            self.gfx.text(&s, x, y + 76.0, wd, 30.0, Font::Normal, tc);
            let f = (hp / mhp.max(1.0)).clamp(0.0, 1.0);
            let bw = wd.min(320.0);
            self.gfx.fill_rect(x, y + 106.0, x + bw, y + 114.0, [0.08, 0.06, 0.05, 0.95]);
            self.gfx.fill_rect(x + 1.0, y + 107.0, x + 1.0 + (bw - 2.0) * f, y + 113.0, [1.0 - f, 0.25 + 0.7 * f, 0.15, 1.0]);
            self.gfx.stroke_rect(x, y + 106.0, x + bw, y + 114.0, 1.0, [0.75, 0.62, 0.34, 0.9]);
        } else if let Some(bh) = self.sel_b {
            let b = self.world.bld(bh).unwrap();
            let n = &self.world.nations[b.nation as usize];
            let nc = n.color;
            let is_divine_city = self.world.descent.is_some() && b.nation as usize == 2 && b.kind.city();
            let px0 = x + 2.0;
            let py0 = y + 4.0;
            let psz = 68.0;
            self.gfx.fill_rect(px0 - 2.0, py0 - 2.0, px0 + psz + 2.0, py0 + psz + 2.0, if is_divine_city { [0.95, 0.85, 0.45, 0.95] } else { c4(nc, 0.9) });
            let bld_bmp = if is_divine_city {
                self.art.divine_bld_sprite(b.kind, false).or_else(|| self.art.bld_icon(b.kind))
            } else {
                self.art.bld_icon(b.kind)
            };
            if let Some(bmp) = bld_bmp {
                self.gfx.draw_bitmap(bmp, px0, py0, px0 + psz, py0 + psz);
            }
            let f = (b.hp / b.kind.max_hp()).clamp(0.0, 1.0);
            self.gfx.fill_rect(px0, py0 + psz + 4.0, px0 + psz, py0 + psz + 11.0, [0.08, 0.06, 0.05, 0.95]);
            self.gfx.fill_rect(px0 + 1.0, py0 + psz + 5.0, px0 + 1.0 + (psz - 2.0) * f, py0 + psz + 10.0, [1.0 - f, 0.25 + 0.7 * f, 0.15, 1.0]);
            self.gfx.stroke_rect(px0, py0 + psz + 4.0, px0 + psz, py0 + psz + 11.0, 1.0, [0.75, 0.62, 0.34, 0.9]);

            let tx = x + 84.0;
            let tw = (wd - 84.0).max(100.0);
            let title = if is_divine_city {
                tf!("上帝の{}　【{}】", "Deity's {}  [{}]", "上帝之{}　【{}】", b.kind.name(), crate::lang::tx(n.name))
            } else {
                format!("{}　【{}】", b.kind.name(), crate::lang::tx(n.name))
            };
            let mut s = tf!("耐久 {:.0}/{:.0}", "HP {:.0}/{:.0}", "耐久 {:.0}/{:.0}", b.hp.max(0.0), b.kind.max_hp());
            if !b.done() {
                let builders = self.world.bld_builders(bh);
                if builders > 0 {
                    s.push_str(&tf!("　（建設中 {:.0}%・作業民{}体）", "  (building {:.0}%, {} workers)", "　（建造中 {:.0}%・施工民{}名）", b.progress * 100.0, builders));
                } else {
                    s.push_str(&tf!("　（建設中断 {:.0}%・右の「建設を再開」で民を派遣）", "  (paused at {:.0}%: press Resume building to send workers)", "　（建造中断 {:.0}%・点右侧“继续建造”派民）", b.progress * 100.0));
                }
            }
            let desc = if is_divine_city {
                t!("天より降臨した上帝の神威が宿る神聖な城邑。上帝の軍が湧き出でる拠点であり、全ての邑を落とせば上帝は退く。", "A holy city filled with the might of the Supreme Deity. The Deity's army pours out from here; take all of them and the Deity withdraws.", "上帝神威所寄的神圣城邑，上帝之军由此涌出。攻下全部城邑，上帝便会退去。")
            } else {
                match b.kind {
                    BKind::Capital => t!("国の中心。民を訓練し、資源の集積地となる。失うと遷都し、全ての城邑を失うと滅亡する。", "Heart of the state. Trains workers and stores resources. If it falls the capital moves; lose every city and the state falls.", "国家中心。训练民，存放资源。失去则迁都，失去全部城邑则灭亡。"),
                    BKind::Town => t!("領土を広げ、民を訓練し、資源の集積地となる。城邑が増えるほど人口上限が上がる。", "Expands territory, trains workers and stores resources. More cities raise your population limits.", "扩张领土，训练民，存放资源。城邑越多人口上限越高。"),
                    BKind::Barracks => t!("戈兵・弩兵・戦車（騎兵）を訓練する。", "Trains halberdiers, crossbowmen and chariots (cavalry).", "训练戈兵、弩兵、战车（骑兵）。"),
                    BKind::Farm => t!("民が耕すと糧を産む。枯れることはない。", "Produces food when a worker farms it. Never runs out.", "民耕作后产粮，永不枯竭。"),
                    BKind::Fort => t!("近づく敵兵を射る（射程9）。数は城邑の数で決まる上限まで。", "Shoots enemy soldiers that come near (range 9). How many you may build depends on your number of cities.", "射击靠近的敌兵（射程9）。数量上限取决于城邑数。"),
                }
            };
            let queue: Vec<(UnitKind, f32)> = b.queue.iter().enumerate().map(|(i, k)| (*k, if i == 0 { b.queue_t / k.train_time() } else { 0.0 })).collect();
            self.gfx.text_shadow(&title, tx, y, tw, 28.0, Font::Big, tc);
            self.gfx.text(&s, tx, y + 30.0, tw, 24.0, Font::Normal, tc);
            self.gfx.text(desc, tx, y + 56.0, tw, 40.0, Font::Small, [0.80, 0.76, 0.68, 1.0]);
            for (i, (k, f)) in queue.iter().enumerate() {
                let qx = tx + i as f32 * 44.0;
                let qy = y + 98.0;
                self.gfx.fill_round(qx, qy, qx + 40.0, qy + 40.0, 3.0, [0.18, 0.14, 0.10, 1.0]);
                if let Some(ubmp) = self.art.unit_icon(*k, era) {
                    self.gfx.draw_bitmap(ubmp, qx + 2.0, qy + 2.0, qx + 38.0, qy + 38.0);
                }
                if *f > 0.0 {
                    self.gfx.fill_rect(qx + 2.0, qy + 34.0, qx + 2.0 + 36.0 * f, qy + 38.0, [0.35, 0.9, 0.4, 1.0]);
                }
            }
        } else {
            let n = &self.world.nations[pl];
            let nc = n.color;
            let is_divine = self.world.descent.is_some() && pl == 2;
            let px0 = x + 2.0;
            let py0 = y + 4.0;
            let psz = 68.0;
            self.gfx.fill_rect(px0 - 2.0, py0 - 2.0, px0 + psz + 2.0, py0 + psz + 2.0, if is_divine { [0.95, 0.85, 0.45, 0.95] } else { c4(nc, 0.9) });
            let cap_bmp = if is_divine {
                self.art.divine_bld_sprite(BKind::Capital, false).or_else(|| self.art.bld_icon(BKind::Capital))
            } else {
                self.art.bld_icon(BKind::Capital)
            };
            if let Some(bmp) = cap_bmp {
                self.gfx.draw_bitmap(bmp, px0, py0, px0 + psz, py0 + psz);
            }
            let tx = x + 84.0;
            let tw = (wd - 84.0).max(100.0);
            let title = if self.gfx.compact {
                tf!("{} の国勢", "State of {}", "{}国势", crate::lang::tx(n.name))
            } else {
                tf!("{} の国勢（右のボタンで民全体・全軍へ命令可）", "State of {} (the buttons on the right command all workers or all soldiers)", "{}国势（右侧按钮可命令全体民或全军）", crate::lang::tx(n.name))
            };
            let wars: Vec<&str> = (0..NATIONS.len()).filter(|&o| self.world.at_war(pl, o)).map(|o| crate::lang::tx(self.world.nations[o].name)).collect();
            let s = tf!(
                "城邑 {}　領土 {}　国力 {:.0}　生産 ×{:.2}　時代 ×{:.2}　上限 {:.1}\n交戦中：{}",
                "Cities {}  Land {}  Strength {:.0}  Production ×{:.2}  Era ×{:.2}  Limit {:.1}\nAt war with: {}",
                "城邑 {}　领土 {}　国力 {:.0}　生产 ×{:.2}　时代 ×{:.2}　上限 {:.1}\n交战中：{}",
                n.cities,
                n.tiles,
                n.strength,
                n.bias,
                self.world.era_mult(),
                cap,
                if wars.is_empty() {
                    t!("なし", "none", "无").to_string()
                } else if wars.len() > 8 {
                    format!("{}{}", wars[..8].join(sep()), tf!("ほか{}国", " +{} more", "等{}国", wars.len() - 8))
                } else {
                    wars.join(sep())
                }
            );
            self.gfx.text_shadow(&title, tx, y, tw, 26.0, Font::Big, tc);
            self.gfx.text(&s, tx, y + 28.0, tw, 48.0, Font::Normal, tc);

            // 自国の各兵種アイコンボタン（クリックでその兵種全体を選択）
            let mut cnt = [0usize; 4];
            for u in self.world.units.iter() {
                if u.alive && u.nation as usize == pl {
                    cnt[u.kind as usize] += 1;
                }
            }
            let kinds = [UnitKind::Worker, UnitKind::Infantry, UnitKind::Archer, UnitKind::Chariot];
            let (mx, my) = self.mouse;
            let mut ix = tx;
            for k in 0..4 {
                let bx0 = ix;
                let by0 = y + 82.0;
                let bx1 = ix + 78.0;
                let by1 = y + 122.0;
                let hov = mx >= bx0 && mx <= bx1 && my >= by0 && my <= by1;
                self.gfx.fill_round(bx0, by0, bx1, by1, 4.0, if hov { [0.35, 0.28, 0.16, 0.95] } else { [0.16, 0.12, 0.08, 0.9] });
                self.gfx.stroke_round(bx0, by0, bx1, by1, 4.0, 1.1, [0.78, 0.64, 0.34, 0.85]);
                if let Some(bmp) = self.art.unit_icon(kinds[k], era) {
                    self.gfx.draw_bitmap(bmp, bx0 + 3.0, by0 + 3.0, bx0 + 37.0, by1 - 3.0);
                }
                let ks = format!("×{}", cnt[k]);
                self.gfx.text_shadow(&ks, bx0 + 40.0, by0 + 8.0, 36.0, 22.0, Font::Normal, tc);
                self.buttons.push(Button {
                    x0: bx0,
                    y0: by0,
                    x1: bx1,
                    y1: by1,
                    label: String::new(),
                    key: 0,
                    enabled: cnt[k] > 0,
                    act: Act::SelectKind(kinds[k]),
                    tip: tf!("{}全体（{}体）を選択する", "Select all {} ({})", "选择全部{}（{}个）", kinds[k].name(era), cnt[k]),
                });
                ix += 84.0;
            }
        }
    }

    fn render_commands(&mut self, x: f32, y: f32) {
        let pl = self.world.player;
        let era = self.world.era;
        let (bw, bh, gap) = (122.0, 46.0, 4.0);
        let pos = |i: usize| -> (f32, f32, f32, f32) {
            let c = (i % 3) as f32;
            let r = (i / 3) as f32;
            let x0 = x + c * (bw + gap);
            let y0 = y + r * (bh + gap);
            (x0, y0, x0 + bw, y0 + bh)
        };
        let res = self.world.nations[pl].res;
        let afford = |c: [f32; 3]| res[0] >= c[0] && res[1] >= c[1] && res[2] >= c[2];
        let own = self.own_sel();
        let (n_workers, n_soldiers) = (self.world.nations[pl].workers, self.world.nations[pl].soldiers);
        if !own.is_empty() {
            let any_worker = own.iter().any(|&h| self.world.unit(h).unwrap().kind == UnitKind::Worker);
            let any_mil = own.iter().any(|&h| self.world.unit(h).unwrap().kind.military());
            let mut i = 0;
            if any_worker {
                for (k, key, tip) in [
                    (BKind::Farm, 'Q', t!("田：民が耕すと糧を産む。自国領内に建設。", "Field: produces food when farmed. Build inside your territory.", "田：民耕作后产粮。建于己方领土内。")),
                    (BKind::Barracks, 'W', t!("兵営：軍事ユニットを訓練する。自国領内に建設。", "Barracks: trains soldiers. Build inside your territory.", "兵营：训练军事单位。建于己方领土内。")),
                    (BKind::Town, 'E', t!("邑：領土を広げ人口上限を上げる。他の城邑から9以上離し、自国城邑から26以内。", "Town: expands territory and raises population limits. At least 9 from any city and within 26 of one of yours.", "邑：扩张领土、提高人口上限。须距其他城邑9以上，距己方城邑26以内。")),
                ] {
                    let (x0, y0, x1, y1) = pos(i);
                    let c = k.cost();
                    self.add_button(x0, y0, x1, y1, k.name(), &Self::cost_str(c), key as u32, afford(c), Act::Place(k), tip);
                    i += 1;
                }
            }
            // 鍛錬
            let u0 = self.world.unit(own[0]).unwrap();
            let u0_kind = u0.kind;
            let up_cost = self.world.upgrade_cost(u0);
            let label = if any_mil && !any_worker { t!("訓練", "Train", "训练") } else if any_worker && !any_mil { t!("教化", "Educate", "教化") } else { t!("鍛錬", "Train", "锻炼") };
            let (x0, y0, x1, y1) = pos(i);
            self.add_button(x0, y0, x1, y1, label, &Self::cost_str(up_cost), 'T' as u32, true, Act::Upgrade, t!("選択部隊の能力を +0.5 高める（自国領内のみ）。兵は戦闘力と兵力、民は採集量が上がる。費用は現在の能力に比例し、糧・木・銅のうち蓄えの多いものから優先して払う。", "Raises the selected units' power by +0.5 (inside your territory only). Soldiers gain strength and HP, workers gather more. The cost grows with power and is paid mostly from whichever of food, wood and bronze you have most of.", "使所选单位能力+0.5（仅限己方领土内）。士兵提升战斗力与兵力，民提升采集量。费用与当前能力成正比，优先从粮、木、铜中储量较多者支付。"));
            i += 1;
            let (x0, y0, x1, y1) = pos(i);
            let (cl, ct) = if any_mil && !any_worker {
                (t!("解散→民", "Disband", "解散→民"), t!("兵を解散して民にする（能力は2/3に目減り）。", "Disband soldiers into workers (power drops to 2/3).", "将士兵解散为民（能力降为2/3）。"))
            } else if any_worker && !any_mil {
                (t!("徴兵→戈兵", "Draft", "征兵→戈兵"), t!("民を徴兵して戈兵にする（能力は2/3に目減り）。", "Draft workers as halberdiers (power drops to 2/3).", "征民为戈兵（能力降为2/3）。"))
            } else {
                (t!("兵⇔民", "Convert", "兵⇔民"), t!("兵は民に、民は戈兵に転換する（能力は2/3に目減り）。", "Soldiers become workers and workers become halberdiers (power drops to 2/3).", "士兵转为民，民转为戈兵（能力降为2/3）。"))
            };
            self.add_button(x0, y0, x1, y1, cl, t!("能力×2/3", "power ×2/3", "能力×2/3"), 'C' as u32, true, Act::Convert, ct);
            i += 1;
            let (x0, y0, x1, y1) = pos(i);
            if any_worker && !any_mil {
                // 民だけを選んでいるときは、停止の代わりに要塞
                let n = &self.world.nations[pl];
                let (nf, cap) = (self.world.fort_count(pl), n.fort_cap());
                let c = BKind::Fort.cost();
                let tip = tf!(
                    "要塞：近づく敵兵を射る（射程{:.0}）。自国領内に建設。数は城邑2つごとに1つ、最大6まで（いま{}／{}）。余った資源の使い道に。",
                    "Fort: shoots enemy soldiers that come near (range {:.0}). Build inside your territory. One per two cities, six at most (now {}/{}). A good use for surplus resources.",
                    "要塞：射击靠近的敌兵（射程{:.0}）。建于己方领土内。每两座城邑可建一座，最多6座（现有{}／{}）。可消耗多余资源。",
                    FORT_RANGE, nf, cap
                );
                let lbl = tf!("要塞 {}/{}", "Fort {}/{}", "要塞 {}/{}", nf, cap);
                self.add_button(x0, y0, x1, y1, &lbl, &Self::cost_str(c), 'R' as u32, afford(c) && nf < cap, Act::Place(BKind::Fort), &tip);
            } else {
                self.add_button(x0, y0, x1, y1, t!("停止", "Stop", "停止"), "", 'S' as u32, true, Act::Stop, t!("選択中の部隊を停止して待機させる。", "Stop the selected units and make them wait.", "让所选单位停止待命。"));
            }
            i += 1;

            if any_worker && !any_mil {
                // 下段：選択民および民全体への一括命令
                let (x0, y0, x1, y1) = pos(6);
                self.add_button(x0, y0, x1, y1, t!("自動採集", "Auto gather", "自动采集"), t!("田・森・銅へ", "auto-assign", "田・森・铜"), 'F' as u32, true, Act::WorkersAuto, t!("選択中の民を、空いている田・森・銅山へバランスよく自動配分する。", "Spread the selected workers evenly over free fields, forests and mines.", "将所选的民均衡分配到空闲的田、森林、铜矿。"));
                let all_sel = own.len() >= n_workers as usize;
                let (x0, y0, x1, y1) = pos(7);
                if all_sel {
                    self.add_button(x0, y0, x1, y1, t!("全民・伐採", "All: wood", "全民・伐木"), t!("最寄りの森へ", "nearest forest", "去最近的森林"), 'G' as u32, true, Act::AllWorkersWood, t!("すべての民（建設中を除く）に最寄りの森の伐採を命じる。", "Order every worker (except builders) to cut the nearest forest.", "命令全部民（建造中除外）砍伐最近的森林。"));
                } else {
                    self.add_button(x0, y0, x1, y1, t!("全民を選択", "All workers", "选择全民"), &tf!("全{}体", "all {}", "全{}名", n_workers), 'G' as u32, n_workers > 0, Act::SelectAllWorkers, t!("自国のすべての民を選択する。", "Select all your workers.", "选择我国全部民。"));
                }
                let (x0, y0, x1, y1) = pos(8);
                if all_sel {
                    self.add_button(x0, y0, x1, y1, t!("全民・採掘", "All: bronze", "全民・采矿"), t!("最寄りの銅山", "nearest mine", "最近的铜矿"), 'V' as u32, true, Act::AllWorkersMine, t!("すべての民（建設中を除く）に最寄りの銅山の採掘を命じる。", "Order every worker (except builders) to mine the nearest mine.", "命令全部民（建造中除外）开采最近的铜矿。"));
                } else {
                    self.add_button(x0, y0, x1, y1, t!("全民・自動", "All: auto", "全民・自动"), t!("民全体を配分", "assign everyone", "分配全部民"), 'V' as u32, n_workers > 0, Act::AllWorkersAuto, t!("自国のすべての民（建設中を除く）を田・森・銅山へ自動でバランス配分する。", "Spread all your workers (except builders) evenly over fields, forests and mines.", "将我国全部民（建造中除外）均衡分配到田、森林、铜矿。"));
                }
            } else if any_mil && !any_worker {
                let (x0, y0, x1, y1) = pos(i);
                self.add_button(x0, y0, x1, y1, t!("進軍", "Attack-move", "进军"), t!("攻撃移動", "fight on the way", "攻击移动"), 'A' as u32, true, Act::AttackMove, t!("クリックした地点へ、途中の敵と戦いながら進む。", "March to the clicked point, fighting enemies on the way.", "向点击的地点进军，沿途与敌交战。"));
                i += 1;
                let (x0, y0, x1, y1) = pos(i);
                self.add_button(x0, y0, x1, y1, t!("防衛・帰還", "Defend", "防卫・归还"), t!("領内迎撃・集結", "guard cities", "领内迎击・集结"), 'D' as u32, true, Act::ArmyDefend, t!("領内の侵入者を迎え撃ち、平時は最寄りの自国城邑へ帰還・防衛配置する。", "Intercept intruders in your land; in peacetime return to the nearest city and guard it.", "迎击领内入侵者，平时返回最近的己方城邑防守。"));
                i += 1;
                let (x0, y0, x1, y1) = pos(i);
                self.add_button(x0, y0, x1, y1, t!("同兵種選択", "Same type", "选择同兵种"), u0_kind.name(era), 'F' as u32, true, Act::SelectKind(u0_kind), t!("自国の同じ兵種のユニットをすべて選択する。", "Select all your units of the same type.", "选择我国所有同兵种单位。"));
                // 下段：全軍（軍事ユニット全体）への命令
                let (x0, y0, x1, y1) = pos(6);
                self.add_button(x0, y0, x1, y1, t!("全軍を選択", "All soldiers", "选择全军"), &tf!("全{}体", "all {}", "全{}个", n_soldiers), 'G' as u32, n_soldiers > 0, Act::SelectAllMilitary, t!("自国のすべての軍事ユニットを選択する。", "Select all your soldiers.", "选择我国全部军事单位。"));
                let (x0, y0, x1, y1) = pos(7);
                self.add_button(x0, y0, x1, y1, t!("全軍・進撃", "All: attack", "全军・进击"), t!("全軍で敵陣へ", "whole army", "全军攻敌"), 'Q' as u32, n_soldiers > 0, Act::AllArmyAttack, t!("全軍事ユニットを選択し、交戦国の城邑・敵部隊へ一斉進撃（または地図クリックで進軍先指定）する。", "Select every soldier and advance on enemy cities and armies (or click the map to choose where).", "选择全部军事单位，向交战国城邑、敌军全面进击（或点击地图指定目标）。"));
                let (x0, y0, x1, y1) = pos(8);
                self.add_button(x0, y0, x1, y1, t!("全軍・防衛", "All: defend", "全军・防卫"), t!("全軍で防衛", "whole army", "全军防守"), 'W' as u32, n_soldiers > 0, Act::AllArmyDefend, t!("全軍事ユニットに領内侵入者の迎撃および城邑への防衛帰還を命じる。", "Order every soldier to intercept intruders and return to defend the cities.", "命令全部军事单位迎击入侵者并返回城邑防守。"));
            } else {
                let (x0, y0, x1, y1) = pos(i);
                self.add_button(x0, y0, x1, y1, t!("進軍", "Attack-move", "进军"), t!("攻撃移動", "fight on the way", "攻击移动"), 'A' as u32, true, Act::AttackMove, t!("クリックした地点へ、途中の敵と戦いながら進む。", "March to the clicked point, fighting enemies on the way.", "向点击的地点进军，沿途与敌交战。"));
                let (x0, y0, x1, y1) = pos(7);
                self.add_button(x0, y0, x1, y1, t!("全民を選択", "All workers", "选择全民"), &tf!("全{}体", "all {}", "全{}名", n_workers), 'G' as u32, n_workers > 0, Act::SelectAllWorkers, t!("自国のすべての民を選択する。", "Select all your workers.", "选择我国全部民。"));
                let (x0, y0, x1, y1) = pos(8);
                self.add_button(x0, y0, x1, y1, t!("全軍を選択", "All soldiers", "选择全军"), &tf!("全{}体", "all {}", "全{}个", n_soldiers), 'V' as u32, n_soldiers > 0, Act::SelectAllMilitary, t!("自国のすべての軍事ユニットを選択する。", "Select all your soldiers.", "选择我国全部军事单位。"));
            }
        } else if let Some(bh) = self.sel_b {
            let b = self.world.bld(bh).unwrap();
            if b.nation as usize != pl {
                return;
            }
            let (kind, done) = (b.kind, b.done());
            let mut i = 0;
            if done {
                let list: Vec<(UnitKind, char, &str)> = match kind {
                    BKind::Capital | BKind::Town => vec![(UnitKind::Worker, 'Q', t!("民：資源を集め、建物を建てる。", "Worker: gathers resources and builds.", "民：采集资源、建造建筑。"))],
                    BKind::Barracks => vec![
                        (UnitKind::Infantry, 'Q', t!("戈兵：戦車・騎兵に強い歩兵。", "Halberdier: infantry strong against chariots and cavalry.", "戈兵：克制战车、骑兵的步兵。")),
                        (UnitKind::Archer, 'W', t!("弩兵：遠距離攻撃。歩兵に強い。", "Crossbowman: ranged attack, strong against infantry.", "弩兵：远程攻击，克制步兵。")),
                        (UnitKind::Chariot, 'E', if era == Era::Spring { t!("戦車：高速。弩兵に強い。", "Chariot: fast, strong against crossbowmen.", "战车：高速，克制弩兵。") } else { t!("騎兵：高速。弩兵に強い。", "Cavalry: fast, strong against crossbowmen.", "骑兵：高速，克制弩兵。") }),
                    ],
                    BKind::Farm | BKind::Fort => vec![],
                };
                for (k, key, tip) in list {
                    let (x0, y0, x1, y1) = pos(i);
                    let c = self.world.train_cost(pl, k);
                    let tip = tf!("{}\n訓練時間 {:.0}秒　維持費 {}", "{}\nTraining time {:.0}s   Upkeep {}", "{}\n训练时间 {:.0}秒　维持费 {}", tip, k.train_time(), if k.military() { t!("糧0.07/秒×能力", "0.07 food/s × power", "粮0.07/秒×能力") } else { t!("なし", "none", "无") });
                    self.add_button(x0, y0, x1, y1, k.name(era), &Self::cost_str(c), key as u32, afford(c), Act::Train(k), &tip);
                    i += 1;
                }
            } else {
                let (x0, y0, x1, y1) = pos(0);
                self.add_button(
                    x0,
                    y0,
                    x1,
                    y1,
                    t!("建設を再開", "Resume building", "继续建造"),
                    t!("民を派遣", "send workers", "派遣民"),
                    'Q' as u32,
                    n_workers > 0,
                    Act::ResumeBuild,
                    t!("近くの民をこの建物の建設へ派遣し、中断した建設を再開（または作業人数を追加）する（Q）", "Send nearby workers to build this, resuming a paused build or adding hands (Q)", "派附近的民来建造此建筑，继续中断的工程或增派人手（Q）"),
                );
            }
            if !kind.city() || !done {
                let (x0, y0, x1, y1) = pos(8);
                self.add_button(x0, y0, x1, y1, t!("取り壊す", "Demolish", "拆除"), "", 0, true, Act::Demolish, t!("この建物を取り壊す（Delete）", "Demolish this building (Delete)", "拆除此建筑（Delete）"));
            }
        } else {
            // 未選択時：民全体・軍事ユニット全体に対する全体命令パネル
            let (x0, y0, x1, y1) = pos(0);
            self.add_button(x0, y0, x1, y1, t!("全民・自動", "All: auto", "全民・自动"), t!("田・森・銅へ", "auto-assign", "田・森・铜"), 'Q' as u32, n_workers > 0, Act::AllWorkersAuto, t!("自国のすべての民（建設中を除く）を、空いている田・近くの森・銅山へバランスよく自動配分する。", "Spread all your workers (except builders) evenly over free fields and nearby forests and mines.", "将我国全部民（建造中除外）均衡分配到空闲的田、附近的森林与铜矿。"));
            let (x0, y0, x1, y1) = pos(1);
            self.add_button(x0, y0, x1, y1, t!("全民・伐採", "All: wood", "全民・伐木"), t!("最寄りの森へ", "nearest forest", "去最近的森林"), 'W' as u32, n_workers > 0, Act::AllWorkersWood, t!("自国のすべての民（建設中を除く）に、最寄りの森の伐採を命じる。", "Order all your workers (except builders) to cut the nearest forest.", "命令我国全部民（建造中除外）砍伐最近的森林。"));
            let (x0, y0, x1, y1) = pos(2);
            self.add_button(x0, y0, x1, y1, t!("全民・採掘", "All: bronze", "全民・采矿"), t!("最寄りの銅山", "nearest mine", "最近的铜矿"), 'E' as u32, n_workers > 0, Act::AllWorkersMine, t!("自国のすべての民（建設中を除く）に、最寄りの銅山の採掘を命じる。", "Order all your workers (except builders) to mine the nearest mine.", "命令我国全部民（建造中除外）开采最近的铜矿。"));

            let (x0, y0, x1, y1) = pos(3);
            self.add_button(x0, y0, x1, y1, t!("全民・教化", "All: educate", "全民・教化"), t!("能力+0.5", "power +0.5", "能力+0.5"), 'T' as u32, n_workers > 0, Act::AllWorkersUpgrade, t!("自国領内の民を、能力の低い順に資源が続く限り一斉に教化する。", "Educate the workers in your territory, weakest first, as far as resources last.", "按能力由低到高，在资源允许范围内一并教化己方领土内的民。"));
            let (x0, y0, x1, y1) = pos(4);
            self.add_button(x0, y0, x1, y1, t!("全民を選択", "All workers", "选择全民"), &tf!("全{}体", "all {}", "全{}名", n_workers), 'G' as u32, n_workers > 0, Act::SelectAllWorkers, t!("自国のすべての民を選択する（建設や一斉移動を行えます）。", "Select all your workers (to build or move them together).", "选择我国全部民（可进行建造或统一移动）。"));
            let (x0, y0, x1, y1) = pos(5);
            self.add_button(x0, y0, x1, y1, t!("全軍を選択", "All soldiers", "选择全军"), &tf!("全{}体", "all {}", "全{}个", n_soldiers), 'V' as u32, n_soldiers > 0, Act::SelectAllMilitary, t!("自国のすべての軍事ユニットを選択する。", "Select all your soldiers.", "选择我国全部军事单位。"));

            let (x0, y0, x1, y1) = pos(6);
            self.add_button(x0, y0, x1, y1, t!("全軍・進撃", "All: attack", "全军・进击"), t!("敵城・目標へ", "enemy cities", "敌城・目标"), 'A' as u32, n_soldiers > 0, Act::AllArmyAttack, t!("全軍事ユニットを選択し、交戦国の最寄り城邑・敵部隊へ総進撃（または地図クリックで進軍先指定）する。", "Select every soldier and advance on the nearest enemy city and armies (or click the map to choose where).", "选择全部军事单位，向交战国最近的城邑、敌军总攻（或点击地图指定目标）。"));
            let (x0, y0, x1, y1) = pos(7);
            self.add_button(x0, y0, x1, y1, t!("全軍・防衛", "All: defend", "全军・防卫"), t!("領内迎撃・帰還", "guard cities", "领内迎击・归还"), 'D' as u32, n_soldiers > 0, Act::AllArmyDefend, t!("全軍事ユニットに、領内の侵入者の迎撃または自国城邑への防衛帰還を命じる。", "Order every soldier to intercept intruders or return to defend your cities.", "命令全部军事单位迎击入侵者或返回己方城邑防守。"));
            let (x0, y0, x1, y1) = pos(8);
            self.add_button(x0, y0, x1, y1, t!("全軍・訓練", "All: train", "全军・训练"), t!("能力+0.5", "power +0.5", "能力+0.5"), 'R' as u32, n_soldiers > 0, Act::AllArmyUpgrade, t!("自国領内の軍事ユニットを、能力の低い順に資源が続く限り一斉に訓練する。", "Train the soldiers in your territory, weakest first, as far as resources last.", "按能力由低到高，在资源允许范围内一并训练己方领土内的军事单位。"));
        }
    }

    fn render_diplo(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pl = self.world.player;
        let pw = 960.0f32.min(w - 20.0);
        let ph = 620.0f32.min(h - 60.0);
        let x0 = (w - pw) / 2.0;
        let y0 = (h - ph) / 2.0;
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.45]);
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + ph);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + ph - 3.0, 16.0);
        if let Some(ic) = &self.art.cmd_icons[CmdIcon::Diplo as usize] {
            self.gfx.draw_bitmap(ic, x0 + 16.0, y0 + 8.0, x0 + 48.0, y0 + 40.0);
        }
        self.gfx.text_shadow(t!("諸侯外交", "Diplomacy", "诸侯外交"), x0 + 56.0, y0 + 10.0, 200.0, 34.0, Font::Big, [1.0, 0.85, 0.5, 1.0]);
        let my = self.world.nations[pl].strength;
        let s = tf!("自国の国力 {:.0}　（国力は兵の能力・城邑・民・領土から算出）", "Our strength {:.0}  (from soldiers' power, cities, workers and land)", "我国国力 {:.0}　（国力由兵力、城邑、民、领土计算）", my);
        self.gfx.text(&s, x0 + 160.0, y0 + 16.0, pw - 370.0, 24.0, Font::Normal, [0.85, 0.82, 0.75, 1.0]);
        self.add_button(x0 + pw - 90.0, y0 + 10.0, x0 + pw - 14.0, y0 + 40.0, t!("閉じる", "Close", "关闭"), "", 0, true, Act::CloseOverlay, "");
        self.add_button(x0 + pw - 196.0, y0 + 10.0, x0 + pw - 98.0, y0 + 40.0, t!("史記 F3", "Annals F3", "史记 F3"), "", 0, true, Act::Chron, t!("英雄・諸子百家・政治指導者、反乱・簒奪の記録と各国の君主家", "Records of heroes, philosophers, statesmen, revolts and usurpations, and each state's ruling house", "英雄、诸子百家、政治家、叛乱与篡位的记录及各国君主家"));
        self.art.draw_inset_box(&self.gfx, x0 + 10.0, y0 + 48.0, x0 + pw - 10.0, y0 + ph - 10.0);
        let home = self.world.nations[pl].home;
        let mut list: Vec<usize> = (0..NATIONS.len()).filter(|&i| i != pl && self.world.nations[i].alive).collect();
        list.sort_by(|&a, &b| self.world.nations[a].home.dist(home).partial_cmp(&self.world.nations[b].home.dist(home)).unwrap());
        let cols = 2;
        let colw = (pw - 30.0) / cols as f32;
        let rowh = 25.0;
        let rows_per = ((ph - 70.0) / rowh) as usize;
        for (k, &o) in list.iter().enumerate() {
            let c = k / rows_per;
            if c >= cols {
                break;
            }
            let r = k % rows_per;
            let rx = x0 + 15.0 + c as f32 * colw;
            let ry = y0 + 54.0 + r as f32 * rowh;
            let n = &self.world.nations[o];
            let war = self.world.at_war(pl, o);
            let truce = self.world.truce[pl][o] - self.world.years;
            let locked = self.world.coalition_locked(pl, o);
            let offer = self.world.peace_offer[o];
            if r % 2 == 0 {
                self.gfx.fill_rect(rx, ry, rx + colw - 8.0, ry + rowh, [1.0, 1.0, 1.0, 0.04]);
            }
            self.gfx.fill_rect(rx + 4.0, ry + 5.0, rx + 18.0, ry + 19.0, c4(n.color, 1.0));
            let name = format!("{}{}", crate::lang::tx(n.name), if NATIONS[o].seven { "★" } else { "" });
            self.gfx.text(&name, rx + 24.0, ry + 3.0, 100.0, 20.0, Font::Normal, [0.95, 0.92, 0.85, 1.0]);
            let st = tf!("国力{:.0} 城{} 距離{:.0}", "Str {:.0} Cities {} Dist {:.0}", "国力{:.0} 城{} 距离{:.0}", n.strength, n.cities, n.home.dist(home));
            self.gfx.text(&st, rx + 126.0, ry + 5.0, 146.0, 20.0, Font::Small, [0.75, 0.72, 0.66, 1.0]);
            let (rel, rc) = if war {
                if locked {
                    (t!("交戦中（連合）", "At war (coalition)", "交战中（联军）"), [1.0, 0.35, 0.3, 1.0])
                } else if offer {
                    (t!("和睦の申し入れ", "Peace offered", "议和请求"), [0.6, 0.9, 1.0, 1.0])
                } else {
                    (t!("交戦中", "Fighting", "交战中"), [1.0, 0.45, 0.35, 1.0])
                }
            } else if truce > 0.0 {
                (t!("休戦中", "Truce", "休战中"), [0.7, 0.9, 0.7, 1.0])
            } else {
                (t!("平和", "Peace", "和平"), [0.75, 0.75, 0.7, 1.0])
            };
            let rel = if !war && truce > 0.0 { tf!("休戦 残{}年", "Truce, {} yrs", "休战 剩{}年", truce.ceil() as i32) } else { rel.to_string() };
            self.gfx.text(&rel, rx + 274.0, ry + 4.0, colw - 364.0, 20.0, Font::Small, rc);
            let bx = rx + colw - 82.0;
            if war {
                let lbl = if offer { t!("受諾", "Accept", "接受") } else { t!("和睦", "Peace", "议和") };
                self.add_button(bx, ry + 2.0, bx + 70.0, ry + rowh - 2.0, lbl, "", 0, !locked, Act::Peace(o), "");
            } else {
                self.add_button(bx, ry + 2.0, bx + 70.0, ry + rowh - 2.0, t!("宣戦", "War", "宣战"), "", 0, truce <= 0.0, Act::War(o), "");
            }
        }
    }

    /// 和睦の申し入れ：受けるか退けるかを選ぶ（返答するまで時間は止まる）
    fn render_peace_ask(&mut self) {
        let Some(q) = self.world.peace_asks.front().cloned() else { return };
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pl = self.world.player;
        let o = q.from;
        let pw = 520.0f32.min(w - 20.0);
        let ph = 250.0;
        let x0 = (w - pw) / 2.0;
        let y0 = (h - ph) / 2.0 - 40.0;
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.35]);
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + ph);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + ph - 3.0, 16.0);
        if let Some(ic) = &self.art.cmd_icons[CmdIcon::Diplo as usize] {
            self.gfx.draw_bitmap(ic, x0 + 16.0, y0 + 8.0, x0 + 48.0, y0 + 40.0);
        }
        self.gfx.text_shadow(t!("和睦の申し入れ", "Peace offered", "议和请求"), x0 + 56.0, y0 + 10.0, 260.0, 34.0, Font::Big, [1.0, 0.85, 0.5, 1.0]);
        let rest = self.world.peace_asks.len();
        if rest > 1 {
            let s = tf!("ほか{}件", "{} more", "另有{}件", rest - 1);
            self.gfx.text(&s, x0 + pw - 110.0, y0 + 18.0, 96.0, 20.0, Font::Small, [0.8, 0.78, 0.7, 1.0]);
        }
        self.art.draw_inset_box(&self.gfx, x0 + 12.0, y0 + 48.0, x0 + pw - 12.0, y0 + ph - 62.0);
        let n = &self.world.nations[o];
        self.gfx.fill_rect(x0 + 24.0, y0 + 62.0, x0 + 40.0, y0 + 78.0, c4(n.color, 1.0));
        let s = tf!("{} が和睦を申し入れてきた（{}）", "{} offers peace ({})", "{}提出议和（{}）", crate::lang::tx(n.name), q.why);
        self.gfx.text(&s, x0 + 48.0, y0 + 58.0, pw - 70.0, 24.0, Font::Normal, [0.95, 0.92, 0.85, 1.0]);
        let dur = (self.world.years - self.world.war_start[pl][o]).max(0.0);
        let s = tf!(
            "交戦{}年　国力：自国 {:.0}　{} {:.0}\n受諾すると{}年間休戦し、その間は互いに宣戦できない。\n退けると交戦を続ける。",
            "At war for {} years.  Strength: us {:.0}, {} {:.0}\nAccepting brings a {}-year truce in which neither side may declare war.\nRefusing continues the war.",
            "交战{}年　国力：我国 {:.0}　{} {:.0}\n接受则休战{}年，其间双方均不能宣战。\n拒绝则继续交战。",
            dur.floor() as i32,
            self.world.nations[pl].strength,
            crate::lang::tx(n.name),
            n.strength,
            q.years as i32
        );
        self.gfx.text(&s, x0 + 24.0, y0 + 88.0, pw - 48.0, 96.0, Font::Small, [0.82, 0.8, 0.74, 1.0]);
        let bw = (pw - 60.0) / 2.0;
        let by = y0 + ph - 52.0;
        let lbl = tf!("受諾（休戦{}年）", "Accept ({}-year truce)", "接受（休战{}年）", q.years as i32);
        self.add_button(x0 + 20.0, by, x0 + 20.0 + bw, by + 38.0, &lbl, "", 'Y' as u32, true, Act::PeaceAnswer(true), t!("和睦して休戦する（Y）", "Make peace and agree a truce (Y)", "议和并休战（Y）"));
        self.add_button(x0 + 40.0 + bw, by, x0 + 40.0 + 2.0 * bw, by + 38.0, t!("拒否（交戦を続ける）", "Refuse (keep fighting)", "拒绝（继续交战）"), "", 'N' as u32, true, Act::PeaceAnswer(false), t!("申し入れを退けて戦を続ける（N）", "Refuse the offer and keep fighting (N)", "拒绝请求，继续作战（N）"));
    }

    fn render_help(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pw = 1180.0f32.min(w - 20.0);
        let ph = (h - 20.0).max(300.0);
        let x0 = (w - pw) / 2.0;
        let y0 = (h - ph) / 2.0;
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.45]);
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + ph);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + ph - 3.0, 16.0);
        if let Some(ic) = &self.art.cmd_icons[CmdIcon::Help as usize] {
            self.gfx.draw_bitmap(ic, x0 + 16.0, y0 + 8.0, x0 + 48.0, y0 + 40.0);
        }
        self.gfx.text_shadow(t!("兵法・遊び方", "How to Play", "兵法・玩法"), x0 + 56.0, y0 + 10.0, 220.0, 34.0, Font::Big, [1.0, 0.85, 0.5, 1.0]);
        self.add_button(x0 + pw - 90.0, y0 + 10.0, x0 + pw - 14.0, y0 + 40.0, t!("閉じる", "Close", "关闭"), "", 0, true, Act::CloseOverlay, "");
        self.art.draw_inset_box(&self.gfx, x0 + 14.0, y0 + 46.0, x0 + pw - 14.0, y0 + ph - 14.0);
        let text = t!(
            "■ 目的　紀元前221年までに天下を統一せよ。時間は全ての国で共通に流れ（春秋は1年＝8秒、戦国は時間がゆっくり流れ1年＝16秒）、春秋時代（〜前453年）から戦国時代へ移る。どの国も同じ時代補正を受け、一部の国だけ文明が進むことはない。

■ 資源　糧（田を耕す）・木（森を伐採）・銅（銅山を採掘）。民が集めて都・邑へ運ぶ。城邑からも少しずつ税収がある。

■ ユニット　動かせる数には上限がある（城邑の数で増える）。その代わり「訓練／教化」で一体ごとの能力を高められる（時代ごとに上限あり。費用は糧・木・銅のうち蓄えの多いものから優先して払うので、どの資源も無駄にならない）。軍事ユニットは存在するだけで糧を消費する（維持費）。糧が尽きると兵は弱っていく。兵を解散して民に、民を徴兵して戈兵にできるが、能力は2/3に目減りする。
　戈兵＞戦車（騎兵）＞弩兵＞戈兵 の相性がある。

■ 外交　F2で外交画面。宣戦しないと他国を攻撃できない。城邑を攻め落とすと自国のものになる。全ての城邑を失った国は滅亡する。
■ 和睦の申し入れ　交戦国からの和睦の申し入れ（連合の解散・史実の休戦を含む）には「受諾（Y）」か「拒否（N）」で答える。答えるまで時間は止まる。拒否すれば交戦を続けられる。

■ 覇者の呼びかけ（春秋）　宣戦や攻略を繰り返すと「諸侯の警戒」が高まり、100に達すると最強国が諸侯に呼びかけ、周辺国が15年間一斉に攻めてくる。
■ 合従軍（戦国）　突出して強い国が現れると、周辺国が合従して15年間一斉に攻撃する。
■ 三家分晋　戦国時代の到来とともに晋は韓・趙・魏に分かれる。
■ 滅亡と人口上限　国が滅ぶと、その国の民・兵の上限（4と、それまでに分けられた分）が生き残った国に分けられる（攻め滅ぼした国が優先）。分けられた分は、城邑の数で決まる上限（最大25）に上乗せされる。
■ 要塞　民だけを選ぶと「要塞（R）」で築ける（糧300・木400・銅300）。近づく敵兵を射る（射程9）。数は「1＋城邑の数÷2」まで（最大6）。余った資源の使い道に。
■ 史実の出来事　史実の年になると、英雄・諸子百家・政治指導者が現れて国を強くし（一定年数、生産や武が上がる）、反乱や簒奪で兵や蓄えを失い君主家（国姓）が変わる。その国が滅んでいれば起こらない。記録と各国の君主家・人物は「史記」（F3、外交画面から）で見られる。
■ 上帝降臨　史実で中華が統一された紀元前221年までに天下を統一できなければ、上帝が秦に降臨する（秦が滅んでいても蘇る）。上帝の軍には抗えず、10年のうちに中華は秦に統一され、ゲームは終わる。
■ 英霊モード（タイトル画面で選ぶ。既定はオフ）　民を選んで「英霊召喚」（H）を押すと、選んだ民を生贄に捧げて三皇五帝から春秋戦国までの英霊を時代と国を超えて呼べる。英霊の能力は生贄の能力の合計の1.1〜3.0倍（英霊ごとに異なる）。一国が同時に従えられる英霊は一体だけで、同じ英霊は二国に現れない。他国も召喚する。英霊だけが上帝の兵を傷つけられ、英霊の近くの城邑は上帝の軍にも奪われない。上帝降臨から10年後、自国の版図が上帝の秦より広ければ勝利。
■ 国の特徴　国ごとに史実に基づく特徴がある（タイトル画面と史記で確認できる）。周は春秋時代には滅びない（最後の城邑を落とされても攻め手は兵を引く）。秦は関中の険で城が堅く、趙は胡服騎射（前307年〜）で騎兵が強く、韓は強弩、魏は武卒、斉・宋・鄭は商人が集まる、など。
■ 交易（F4）　国に属さない商人が諸国を巡り、生産力の高い国ほど多く集まる。商人は領内を通るだけで税（銭）を納め、中原全体の生産量に比例して数が増える。税率は軽いほど商人が集まる。銭は市場で糧・木・銅に換えられる。西域の商人は河西回廊・草原の道・蜀身毒道の先の地域（関中・河東・洛邑、北辺、蜀・巴）にだけ来て、多くの税を落とす。
■ 山岳民・遊牧民　犬戎・山戎・林胡・東胡・匈奴・百越などは国ではない。時代に合わせて現れ、守りの薄い国へ略奪に来たり（城邑を落とすと蓄えを奪って引き上げる）、馬や毛皮を商いに来たりする。交易画面から銭で傭兵に雇え、雇った部族はその間その国を襲わない。
■ 指揮中は時が止まる　自国の部隊を選ぶと時間が止まり、移動・攻撃・採集・建設などの命令を出すと（または選択を解くと）再び進む。範囲選択のドラッグ中も止まる。
■ 戦場の霧　自国の民・兵・建物の周囲しか見えない。一度見た土地は暗く残り、地形と城は見えるが敵兵は見えない。諸侯の都の所在だけは初めから分かっている。

■ 操作
　左クリック：選択　ドラッグ：範囲選択　ダブルクリック：同種を全選択　Shift：追加選択
　右クリック：移動／攻撃／採集（森・銅山・田）／建設の手伝い　建物選択中の右クリック：集合地点
　A：進軍（攻撃移動）　S：停止　T：訓練・教化　C：兵⇔民の転換　Q/W/E：建設・訓練
　Ctrl を押しながら建物を置く：建設の予約（続けて置ける）。選んだ民は今の建設を終えてから、置いた順に建てていく（建設地に順番を表示）
　Ctrl+数字：部隊登録　数字：部隊呼び出し　.：待機中の民　Home：国都へ
　ホイール：拡大縮小　中ボタンドラッグ・矢印キー・画面端：スクロール　ミニマップクリック：移動
　Space：一時停止　＋／－：速度　Esc：取り消し　F5：記録（セーブ・ロード）　F11：全画面　Ctrl＋＋／Ctrl＋−：文字の大きさ",
            "■ Goal  Unify the realm by 221 BC. Time flows the same for every state (in the Spring and Autumn period 1 year = 8 s; in the Warring States time slows to 1 year = 16 s), moving from the Spring and Autumn period (until 453 BC) into the Warring States. Every state gets the same era bonus; no state advances alone.\n\n■ Resources  Food (farm fields), wood (cut forests), bronze (mine). Workers carry them to the capital or a town. Cities also pay a little tax.\n\n■ Units  The number you can field is limited (more cities raise it). Instead, Train / Educate raises each unit's power (capped per era; the cost is paid mostly from whichever of food, wood and bronze you have most of, so nothing goes to waste). Soldiers eat food just by existing (upkeep); without food they weaken. Soldiers can be disbanded into workers and workers drafted as halberdiers, but power drops to 2/3.\n  Counters: halberdier > chariot (cavalry) > crossbowman > halberdier.\n\n■ Diplomacy  F2 opens Diplomacy. You must declare war before attacking. Capture a city and it becomes yours. A state that loses all its cities falls.\n■ Peace offers  When an enemy offers peace (including when a coalition disbands or a historical truce occurs), answer Accept (Y) or Refuse (N). Time stops until you answer. Refuse to keep fighting.\n\n■ The Hegemon's Call (Spring and Autumn)  Repeated wars and conquests raise the lords' alarm. At 100 the strongest state calls on the lords and the neighbors attack together for 15 years.\n■ Grand Alliance (Warring States)  When one state grows far stronger than the rest, its neighbors ally and attack together for 15 years.\n■ Partition of Jin  At the start of the Warring States, Jin splits into Han, Zhao and Wei.\n■ Fallen states  When a state falls, its population allowance (4, plus whatever it had received) is shared among the surviving states, the conqueror first. It is added on top of the worker and soldier limits set by your cities (normally 25 at most).\n■ Forts  With only workers selected, press Fort (R) to build one (food 300, wood 400, bronze 300). It shoots enemy soldiers that come near (range 9). You may have 1 + cities ÷ 2 of them (6 at most). A good use for surplus resources.\n■ Historical events  In the right year, heroes, philosophers and statesmen appear and strengthen their state (production or military up for some years), while revolts and usurpations cost soldiers and stores and change the ruling house. Events of fallen states do not happen. See the records and each state's house and figures in the Annals (F3, or from Diplomacy).\n■ Descent of the Supreme Deity  If the realm is not unified by 221 BC, the year it was unified in history, the Supreme Deity descends upon Qin (Qin returns even if destroyed). None can resist the Deity's army; within 10 years China is unified under Qin and the game ends.\n■ Heroic Spirits mode (chosen on the title screen, off by default)  Select workers and press Summon Spirit (H) to sacrifice them and summon a spirit from the Three Sovereigns to the Warring States, across eras and states. A spirit has 1.1–3.0 times the total power of the sacrifice (it depends on the spirit). Each state may have one spirit at a time, and a spirit never appears for two states. Other states summon too. Only spirits can harm the Deity's soldiers, and cities near a spirit cannot be taken by the Deity's army. If your territory is larger than the Deity's Qin 10 years after the Descent, you win.\n■ National traits  Each state has historical traits (see the title screen and the Annals). Zhou cannot fall in the Spring and Autumn period (an attacker withdraws from its last city). Qin's cities are hard to take behind its passes, Zhao's cavalry is strong after adopting Hu dress (from 307 BC), Han has fine crossbows, Wei its armored infantry, and merchants flock to Qi, Song and Zheng.\n■ Trade (F4)  Merchants who belong to no state travel the land and gather in states with high production. They pay tax (coin) just by passing through your land, and their number grows with the total production of the Central Plains. Lighter taxes draw more merchants. Coin buys food, wood and bronze at the market. Western merchants come only to the lands at the end of the Hexi Corridor, the Steppe Road and the Shu–India Road (Guanzhong, Hedong, Luoyi; the northern frontier; Shu and Ba) and pay a lot of tax.\n■ Hill and steppe peoples  The Quanrong, Shanrong, Linhu, Donghu, Xiongnu, Baiyue and others are not states. They appear in their historical eras, raid weakly defended states (taking a city, they seize its stores and leave) or come to trade horses and furs. Hire them as mercenaries with coin on the Trade screen; while hired they will not attack you.\n■ Time stops while commanding  Selecting your own units stops time; it resumes when you give an order (move, attack, gather, build...) or clear the selection. It also stops while drag-selecting.\n■ Fog of war  You see only around your own workers, soldiers and buildings. Land you have seen stays dimly visible with its terrain and cities, but not enemy soldiers. The capitals of all lords are known from the start.\n\n■ Controls\n  Left click: select   Drag: box select   Double click: select all of a type   Shift: add to selection\n  Right click: move / attack / gather (forest, mine, field) / help build   Right click with a building selected: rally point\n  A: attack-move   S: stop   T: train / educate   C: convert soldier ⇔ worker   Q/W/E: build / train\n  Hold Ctrl while placing buildings: reserve builds (keep placing). The selected workers finish their current build, then build in the order placed (numbers show the order)\n  Ctrl+number: save group   Number: recall group   .: idle worker   Home: go to capital\n  Wheel: zoom   Middle drag / arrow keys / screen edge: scroll   Minimap click: jump\n  Space: pause   + / -: speed   Esc: cancel   F5: save / load   F11: fullscreen   Ctrl++ / Ctrl+−: text size",
            "■ 目标　在公元前221年之前统一天下。时间对所有国家同样流逝（春秋时1年＝8秒，战国时时间放慢为1年＝16秒），由春秋时代（至前453年）进入战国时代。各国享有相同的时代加成，不会只有部分国家文明领先。\n\n■ 资源　粮（耕田）、木（伐木）、铜（采铜矿）。由民采集并运往都、邑。城邑也会缴纳少量税收。\n\n■ 单位　可指挥的数量有上限（随城邑数增加）。作为补偿，可通过“训练／教化”提升每个单位的能力（各时代有上限。费用优先从粮、木、铜中储量较多者支付，任何资源都不会浪费）。军事单位只要存在就会消耗粮（维持费），粮尽则士兵逐渐衰弱。士兵可解散为民，民可征为戈兵，但能力降为2/3。\n　相克关系：戈兵＞战车（骑兵）＞弩兵＞戈兵。\n\n■ 外交　按 F2 打开外交画面。不宣战就不能攻击他国。攻下城邑即归我方所有。失去全部城邑的国家灭亡。\n■ 议和请求　交战国提出议和（包括联军解散、史实休战）时，以“接受（Y）”或“拒绝（N）”回应。回应之前时间停止。拒绝即可继续交战。\n\n■ 霸主号召（春秋）　频繁宣战或攻城会提高“诸侯警戒”，达到100时最强之国号召诸侯，周边各国在15年内一齐来攻。\n■ 合纵军（战国）　出现一枝独秀的强国时，周边各国合纵，在15年内一齐进攻。\n■ 三家分晋　战国时代到来时，晋分裂为韩、赵、魏。\n■ 灭亡与人口上限　国家灭亡时，其民与兵的上限（4，加上此前分得的部分）会分给存活的国家，攻灭者优先。分得的部分加在由城邑数决定的上限（最多25）之上。\n■ 要塞　只选中民时，可用“要塞（R）”修筑（粮300・木400・铜300）。射击靠近的敌兵（射程9）。数量上限为“1＋城邑数÷2”（最多6）。可消耗多余资源。\n■ 史实事件　到了史实年份，英雄、诸子百家、政治家会出现并增强国力（一定年数内生产或武力提升）；叛乱与篡位则会损失士兵与储备，君主家（国姓）随之更替。若该国已灭亡则不会发生。记录及各国君主家、人物可在“史记”（F3，或从外交画面）查看。\n■ 上帝降临　若未能在史实上中华统一的公元前221年之前统一天下，上帝将降临于秦（即使秦已灭亡也会复活）。上帝之军不可抗拒，10年之内中华将归于秦，游戏结束。\n■ 英灵模式（在标题画面选择，默认关闭）　选中民后按“召唤英灵”（H），即可献祭所选的民，跨越时代与国界召唤自三皇五帝至春秋战国的英灵。英灵能力为祭品能力总和的1.1〜3.0倍（因英灵而异）。每国同时只能拥有一位英灵，同一英灵不会出现在两国。他国也会召唤。只有英灵能伤及上帝之兵，英灵附近的城邑也不会被上帝之军夺走。上帝降临10年后，若我国版图大于上帝之秦即获胜。\n■ 国家特点　各国拥有基于史实的特点（可在标题画面与史记中查看）。周在春秋时代不会灭亡（即使最后的城邑被攻破，攻方也会撤兵）。秦有关中之险，城池坚固；赵推行胡服骑射（前307年起）后骑兵强大；韩有强弩，魏有武卒；齐、宋、郑商人云集，等等。\n■ 贸易（F4）　不属于任何国家的商人周游列国，生产力越高的国家聚集越多。商人只要途经领内就会缴税（钱），其数量与中原整体生产量成正比。税率越轻，商人越多。钱可在市场换成粮、木、铜。西域商人只会来到河西走廊、草原之路、蜀身毒道尽头的地区（关中、河东、洛邑，北方边地，蜀、巴），并缴纳大量税收。\n■ 山地与游牧民　犬戎、山戎、林胡、东胡、匈奴、百越等不是国家。他们按时代出现，或来劫掠防守薄弱的国家（攻下城邑后夺取储备撤走），或来贩卖马匹、毛皮。可在贸易画面用钱雇其为佣兵，受雇期间不会袭击雇主。\n■ 指挥中时间停止　选中己方单位时时间停止，下达移动、攻击、采集、建造等命令（或取消选择）后继续。框选拖动时也会停止。\n■ 战争迷雾　只能看到己方民、兵、建筑周围。看过的土地会暗淡地保留，可见地形与城池，但看不到敌兵。诸侯都城的位置从一开始就已知。\n\n■ 操作\n　左键：选择　拖动：框选　双击：选择全部同类　Shift：追加选择\n　右键：移动／攻击／采集（森林・铜矿・田）／协助建造　选中建筑时右键：集结点\n　A：进军（攻击移动）　S：停止　T：训练・教化　C：兵⇔民转换　Q/W/E：建造・训练\n　按住 Ctrl 放置建筑：预约建造（可连续放置）。所选的民完成当前建造后，按放置顺序依次建造（建造地显示顺序）\n　Ctrl+数字：编队　数字：调出编队　.：闲置的民　Home：回国都\n　滚轮：缩放　中键拖动・方向键・画面边缘：滚动　点击小地图：移动\n　Space：暂停　＋／－：速度　Esc：取消　F5：存档（保存・读取）　F11：全屏　Ctrl＋＋／Ctrl＋−：文字大小"
        );
        // 入りきらなければ小さい字にする
        let f = if self.gfx.measure_h(text, Font::Normal, pw - 48.0) > ph - 64.0 { Font::Small } else { Font::Normal };
        self.gfx.text(text, x0 + 24.0, y0 + 52.0, pw - 48.0, ph - 64.0, f, [0.92, 0.9, 0.84, 1.0]);
    }

    fn render_gameover(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let Some((win, text)) = self.world.game_over.clone() else { return };
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.55]);
        let pw = 640.0;
        let x0 = (w - pw) / 2.0;
        let y0 = h / 2.0 - 140.0;
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + 260.0);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + 257.0, 16.0);
        let title = if win {
            t!("勝利", "Victory", "胜利")
        } else if self.world.descent.is_some() {
            t!("天命は秦に", "Heaven's Mandate Passes to Qin", "天命归秦")
        } else {
            t!("敗北", "Defeat", "败北")
        };
        self.gfx.text_shadow(title, x0, y0 + 20.0, pw, 60.0, Font::Title, if win { [1.0, 0.85, 0.4, 1.0] } else { [0.9, 0.4, 0.35, 1.0] });
        self.gfx.text(&text, x0 + 20.0, y0 + 100.0, pw - 40.0, 60.0, Font::BigCenter, [0.95, 0.92, 0.85, 1.0]);
        self.add_button(x0 + pw / 2.0 - 210.0, y0 + 180.0, x0 + pw / 2.0 - 10.0, y0 + 228.0, t!("新しい天下へ", "New game", "新的天下"), "", 0, true, Act::Restart, "");
        self.add_button(x0 + pw / 2.0 + 10.0, y0 + 180.0, x0 + pw / 2.0 + 210.0, y0 + 228.0, t!("記録から再開 F5", "Load a save F5", "读取存档 F5"), "", 0, true, Act::SaveMenu, "");
    }
}

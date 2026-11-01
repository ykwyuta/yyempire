//! Age of Empires 風のUI背景テクスチャ・各種アイコン・肖像画の生成と描画

use crate::gfx::Gfx;
use crate::rng::{fbm, hash2};
use crate::world::{BKind, Era, UnitKind};
use crate::gfx::Bitmap as ID2D1Bitmap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResIcon {
    Food = 0,
    Wood = 1,
    Bronze = 2,
    Worker = 3,
    Soldier = 4,
    Upkeep = 5,
    Era = 6,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CmdIcon {
    Worker = 0,
    Infantry = 1,
    Archer = 2,
    Chariot = 3,
    Cavalry = 4,
    Farm = 5,
    Barracks = 6,
    Town = 7,
    Capital = 8,
    Upgrade = 9,
    Convert = 10,
    Stop = 11,
    AttackMove = 12,
    Demolish = 13,
    Diplo = 14,
    Help = 15,
    GatherAuto = 16,
    GatherWood = 17,
    GatherBronze = 18,
    Defend = 19,
    AllArmy = 20,
    Fort = 21,
}

const UNIT_SPRITE_SZ: usize = 160;
const RAW_UNIT_SPRITES: [&[u8]; 5] = [
    include_bytes!("../assets/units/worker.bgra"),
    include_bytes!("../assets/units/infantry.bgra"),
    include_bytes!("../assets/units/archer.bgra"),
    include_bytes!("../assets/units/chariot.bgra"),
    include_bytes!("../assets/units/cavalry.bgra"),
];

const BLD_SPRITE_SZ: usize = 192;
const RAW_BLD_SPRITES: [&[u8]; 5] = [
    include_bytes!("../assets/map/capital.bgra"),
    include_bytes!("../assets/map/town.bgra"),
    include_bytes!("../assets/map/barracks.bgra"),
    include_bytes!("../assets/map/farm.bgra"),
    include_bytes!("../assets/map/fort.bgra"),
];

const PROP_SPRITE_SZ: usize = 160;
const RAW_PROP_SPRITES: [&[u8]; 3] = [
    include_bytes!("../assets/map/mountain.bgra"),
    include_bytes!("../assets/map/forest.bgra"),
    include_bytes!("../assets/map/mine.bgra"),
];

const HERO_SPRITE_SZ: usize = 128;
const RAW_HERO_SPRITES: [&[u8]; 31] = [
    include_bytes!("../assets/heroes/hero_00.bgra"),
    include_bytes!("../assets/heroes/hero_01.bgra"),
    include_bytes!("../assets/heroes/hero_02.bgra"),
    include_bytes!("../assets/heroes/hero_03.bgra"),
    include_bytes!("../assets/heroes/hero_04.bgra"),
    include_bytes!("../assets/heroes/hero_05.bgra"),
    include_bytes!("../assets/heroes/hero_06.bgra"),
    include_bytes!("../assets/heroes/hero_07.bgra"),
    include_bytes!("../assets/heroes/hero_08.bgra"),
    include_bytes!("../assets/heroes/hero_09.bgra"),
    include_bytes!("../assets/heroes/hero_10.bgra"),
    include_bytes!("../assets/heroes/hero_11.bgra"),
    include_bytes!("../assets/heroes/hero_12.bgra"),
    include_bytes!("../assets/heroes/hero_13.bgra"),
    include_bytes!("../assets/heroes/hero_14.bgra"),
    include_bytes!("../assets/heroes/hero_15.bgra"),
    include_bytes!("../assets/heroes/hero_16.bgra"),
    include_bytes!("../assets/heroes/hero_17.bgra"),
    include_bytes!("../assets/heroes/hero_18.bgra"),
    include_bytes!("../assets/heroes/hero_19.bgra"),
    include_bytes!("../assets/heroes/hero_20.bgra"),
    include_bytes!("../assets/heroes/hero_21.bgra"),
    include_bytes!("../assets/heroes/hero_22.bgra"),
    include_bytes!("../assets/heroes/hero_23.bgra"),
    include_bytes!("../assets/heroes/hero_24.bgra"),
    include_bytes!("../assets/heroes/hero_25.bgra"),
    include_bytes!("../assets/heroes/hero_26.bgra"),
    include_bytes!("../assets/heroes/hero_27.bgra"),
    include_bytes!("../assets/heroes/hero_28.bgra"),
    include_bytes!("../assets/heroes/hero_29.bgra"),
    include_bytes!("../assets/heroes/hero_30.bgra"),
];

const RAW_DIVINE_UNIT_SPRITES: [&[u8]; 2] = [
    include_bytes!("../assets/divine/divine_infantry.bgra"),
    include_bytes!("../assets/divine/divine_chariot.bgra"),
];

const RAW_DIVINE_BLD_SPRITES: [&[u8]; 2] = [
    include_bytes!("../assets/divine/divine_capital.bgra"),
    include_bytes!("../assets/divine/divine_town.bgra"),
];

pub struct Art {
    gen: u32,
    pub hud_stone: Option<ID2D1Bitmap>,
    pub panel_wood: Option<ID2D1Bitmap>,
    pub title_crest: Option<ID2D1Bitmap>,
    pub btn_norm: Option<ID2D1Bitmap>,
    pub btn_hover: Option<ID2D1Bitmap>,
    pub btn_dis: Option<ID2D1Bitmap>,
    pub res_icons: [Option<ID2D1Bitmap>; 7],
    pub cmd_icons: [Option<ID2D1Bitmap>; 22],
    pub unit_badges: [Option<ID2D1Bitmap>; 5],
    pub unit_sprites: [Option<ID2D1Bitmap>; 5],
    pub unit_sprites_flip: [Option<ID2D1Bitmap>; 5],
    pub bld_sprites: [Option<ID2D1Bitmap>; 5],
    pub bld_sprites_fog: [Option<ID2D1Bitmap>; 5],
    pub prop_sprites: [Option<ID2D1Bitmap>; 3],
    pub prop_sprites_flip: [Option<ID2D1Bitmap>; 3],
    pub prop_sprites_fog: [Option<ID2D1Bitmap>; 3],
    pub prop_sprites_fog_flip: [Option<ID2D1Bitmap>; 3],
    pub hero_sprites: [Option<ID2D1Bitmap>; 31],
    pub hero_sprites_flip: [Option<ID2D1Bitmap>; 31],
    pub divine_unit_sprites: [Option<ID2D1Bitmap>; 2],
    pub divine_unit_sprites_flip: [Option<ID2D1Bitmap>; 2],
    pub divine_bld_sprites: [Option<ID2D1Bitmap>; 2],
    pub divine_bld_sprites_fog: [Option<ID2D1Bitmap>; 2],
}

struct Canvas {
    w: usize,
    h: usize,
    px: Vec<[f32; 4]>,
}

impl Canvas {
    fn new(w: usize, h: usize) -> Self {
        Self { w, h, px: vec![[0.0; 4]; w * h] }
    }

    #[inline]
    fn blend(&mut self, x: i32, y: i32, c: [f32; 4]) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let a = c[3].clamp(0.0, 1.0);
        if a <= 1e-4 {
            return;
        }
        let d = &mut self.px[y as usize * self.w + x as usize];
        let inv = 1.0 - a;
        let out_a = a + d[3] * inv;
        if out_a > 1e-5 {
            d[0] = (c[0] * a + d[0] * d[3] * inv) / out_a;
            d[1] = (c[1] * a + d[1] * d[3] * inv) / out_a;
            d[2] = (c[2] * a + d[2] * d[3] * inv) / out_a;
            d[3] = out_a;
        }
    }

    fn circle(&mut self, cx: f32, cy: f32, r: f32, c: [f32; 4]) {
        let x0 = (cx - r - 1.5).floor() as i32;
        let x1 = (cx + r + 1.5).ceil() as i32;
        let y0 = (cy - r - 1.5).floor() as i32;
        let y1 = (cy + r + 1.5).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                let cov = (r + 0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, [c[0], c[1], c[2], c[3] * cov]);
                }
            }
        }
    }

    fn ring(&mut self, cx: f32, cy: f32, r: f32, w: f32, c: [f32; 4]) {
        let x0 = (cx - r - w - 1.5).floor() as i32;
        let x1 = (cx + r + w + 1.5).ceil() as i32;
        let y0 = (cy - r - w - 1.5).floor() as i32;
        let y1 = (cy + r + w + 1.5).ceil() as i32;
        let hw = w * 0.5;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                let dist = (d - r).abs();
                let cov = (hw + 0.5 - dist).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, [c[0], c[1], c[2], c[3] * cov]);
                }
            }
        }
    }

    fn capsule(&mut self, ax: f32, ay: f32, bx: f32, by: f32, r: f32, c: [f32; 4]) {
        let x0 = (ax.min(bx) - r - 1.5).floor() as i32;
        let x1 = (ax.max(bx) + r + 1.5).ceil() as i32;
        let y0 = (ay.min(by) - r - 1.5).floor() as i32;
        let y1 = (ay.max(by) + r + 1.5).ceil() as i32;
        let dx = bx - ax;
        let dy = by - ay;
        let l2 = dx * dx + dy * dy;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let t = if l2 > 1e-6 { ((px - ax) * dx + (py - ay) * dy) / l2 } else { 0.0 }.clamp(0.0, 1.0);
                let qx = ax + dx * t;
                let qy = ay + dy * t;
                let d = ((px - qx).powi(2) + (py - qy).powi(2)).sqrt();
                let cov = (r + 0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, [c[0], c[1], c[2], c[3] * cov]);
                }
            }
        }
    }

    fn round_rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, c: [f32; 4]) {
        let ix0 = (x0 - 1.0).floor() as i32;
        let ix1 = (x1 + 1.0).ceil() as i32;
        let iy0 = (y0 - 1.0).floor() as i32;
        let iy1 = (y1 + 1.0).ceil() as i32;
        let cx = (x0 + x1) * 0.5;
        let cy = (y0 + y1) * 0.5;
        let hx = (x1 - x0) * 0.5 - r;
        let hy = (y1 - y0) * 0.5 - r;
        for y in iy0..=iy1 {
            for x in ix0..=ix1 {
                let px = (x as f32 + 0.5 - cx).abs() - hx;
                let py = (y as f32 + 0.5 - cy).abs() - hy;
                let d = (px.max(0.0).powi(2) + py.max(0.0).powi(2)).sqrt() + px.max(py).min(0.0) - r;
                let cov = (0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, [c[0], c[1], c[2], c[3] * cov]);
                }
            }
        }
    }

    fn poly(&mut self, pts: &[(f32, f32)], c: [f32; 4]) {
        if pts.len() < 3 {
            return;
        }
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for &(x, y) in pts {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let ix0 = (min_x - 1.0).floor() as i32;
        let ix1 = (max_x + 1.0).ceil() as i32;
        let iy0 = (min_y - 1.0).floor() as i32;
        let iy1 = (max_y + 1.0).ceil() as i32;
        // 2x2 オーバーサンプリング
        for y in iy0..=iy1 {
            for x in ix0..=ix1 {
                let mut hit = 0;
                for sy in 0..2 {
                    for sx in 0..2 {
                        let px = x as f32 + (sx as f32 + 0.5) * 0.5;
                        let py = y as f32 + (sy as f32 + 0.5) * 0.5;
                        let mut inside = false;
                        let mut j = pts.len() - 1;
                        for i in 0..pts.len() {
                            let (xi, yi) = pts[i];
                            let (xj, yj) = pts[j];
                            if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
                                inside = !inside;
                            }
                            j = i;
                        }
                        if inside {
                            hit += 1;
                        }
                    }
                }
                if hit > 0 {
                    self.blend(x, y, [c[0], c[1], c[2], c[3] * (hit as f32 * 0.25)]);
                }
            }
        }
    }

    fn to_bgra_premul(&self) -> Vec<u32> {
        self.px
            .iter()
            .map(|p| {
                let a = p[3].clamp(0.0, 1.0);
                let r = (p[0].clamp(0.0, 1.0) * a * 255.0).round() as u32;
                let g = (p[1].clamp(0.0, 1.0) * a * 255.0).round() as u32;
                let b = (p[2].clamp(0.0, 1.0) * a * 255.0).round() as u32;
                let ai = (a * 255.0).round() as u32;
                b | (g << 8) | (r << 16) | (ai << 24)
            })
            .collect()
    }

    fn blit_raw_sprite(
        &mut self,
        raw: &[u8],
        sz: usize,
        dst: (f32, f32, f32, f32),
        src_uv: (f32, f32, f32, f32),
        circle_clip: Option<(f32, f32, f32)>,
    ) {
        let sw = sz as f32;
        let sh = sz as f32;
        let (dx0, dy0, dx1, dy1) = dst;
        let (u0, v0, u1, v1) = src_uv;
        let ix0 = dx0.floor().max(0.0) as i32;
        let iy0 = dy0.floor().max(0.0) as i32;
        let ix1 = dx1.ceil().min(self.w as f32 - 1.0) as i32;
        let iy1 = dy1.ceil().min(self.h as f32 - 1.0) as i32;
        let dw = (dx1 - dx0).max(1.0);
        let dh = (dy1 - dy0).max(1.0);

        let sample_px = |sx: i32, sy: i32| -> [f32; 4] {
            let cx = sx.clamp(0, sz as i32 - 1) as usize;
            let cy = sy.clamp(0, sz as i32 - 1) as usize;
            let off = (cy * sz + cx) * 4;
            let b = raw[off] as f32 / 255.0;
            let g = raw[off + 1] as f32 / 255.0;
            let r = raw[off + 2] as f32 / 255.0;
            let a = raw[off + 3] as f32 / 255.0;
            if a > 1e-4 {
                [(r / a).clamp(0.0, 1.0), (g / a).clamp(0.0, 1.0), (b / a).clamp(0.0, 1.0), a]
            } else {
                [0.0; 4]
            }
        };

        for y in iy0..=iy1 {
            for x in ix0..=ix1 {
                let mut clip_a = 1.0;
                if let Some((ccx, ccy, cr)) = circle_clip {
                    let d = ((x as f32 + 0.5 - ccx).powi(2) + (y as f32 + 0.5 - ccy).powi(2)).sqrt();
                    clip_a = (cr + 0.5 - d).clamp(0.0, 1.0);
                    if clip_a <= 0.0 {
                        continue;
                    }
                }
                let tx = (x as f32 + 0.5 - dx0) / dw;
                let ty = (y as f32 + 0.5 - dy0) / dh;
                let su = (u0 + (u1 - u0) * tx) * sw - 0.5;
                let sv = (v0 + (v1 - v0) * ty) * sh - 0.5;
                let sx0 = su.floor() as i32;
                let sy0 = sv.floor() as i32;
                let fx = su - sx0 as f32;
                let fy = sv - sy0 as f32;
                let c00 = sample_px(sx0, sy0);
                let c10 = sample_px(sx0 + 1, sy0);
                let c01 = sample_px(sx0, sy0 + 1);
                let c11 = sample_px(sx0 + 1, sy0 + 1);
                let mut c = [0.0f32; 4];
                let w00 = (1.0 - fx) * (1.0 - fy);
                let w10 = fx * (1.0 - fy);
                let w01 = (1.0 - fx) * fy;
                let w11 = fx * fy;
                let a = c00[3] * w00 + c10[3] * w10 + c01[3] * w01 + c11[3] * w11;
                if a > 1e-3 {
                    for k in 0..3 {
                        c[k] = (c00[k] * c00[3] * w00 + c10[k] * c10[3] * w10 + c01[k] * c01[3] * w01 + c11[k] * c11[3] * w11) / a;
                    }
                    c[3] = a * clip_a;
                    self.blend(x, y, c);
                }
            }
        }
    }

    fn blit_unit_sprite(
        &mut self,
        unit_idx: usize,
        dst: (f32, f32, f32, f32),
        src_uv: (f32, f32, f32, f32),
        circle_clip: Option<(f32, f32, f32)>,
    ) {
        self.blit_raw_sprite(RAW_UNIT_SPRITES[unit_idx.min(4)], UNIT_SPRITE_SZ, dst, src_uv, circle_clip);
    }

    fn blit_bld_sprite(
        &mut self,
        bld_idx: usize,
        dst: (f32, f32, f32, f32),
        src_uv: (f32, f32, f32, f32),
    ) {
        self.blit_raw_sprite(RAW_BLD_SPRITES[bld_idx.min(4)], BLD_SPRITE_SZ, dst, src_uv, None);
    }

    fn blit_prop_sprite(
        &mut self,
        prop_idx: usize,
        dst: (f32, f32, f32, f32),
        src_uv: (f32, f32, f32, f32),
    ) {
        self.blit_raw_sprite(RAW_PROP_SPRITES[prop_idx.min(2)], PROP_SPRITE_SZ, dst, src_uv, None);
    }
}

fn load_raw_sprite_bgra(raw: &[u8], sz: usize, flip_x: bool, mul: f32) -> Vec<u32> {
    let mut out = vec![0u32; sz * sz];
    for y in 0..sz {
        for x in 0..sz {
            let sx = if flip_x { sz - 1 - x } else { x };
            let off = (y * sz + sx) * 4;
            let b = ((raw[off] as f32 * mul).round() as u32).min(255);
            let g = ((raw[off + 1] as f32 * mul).round() as u32).min(255);
            let r = ((raw[off + 2] as f32 * mul).round() as u32).min(255);
            let a = raw[off + 3] as u32;
            out[y * sz + x] = b | (g << 8) | (r << 16) | (a << 24);
        }
    }
    out
}

fn load_unit_sprite_bgra(idx: usize, flip_x: bool) -> Vec<u32> {
    load_raw_sprite_bgra(RAW_UNIT_SPRITES[idx.min(4)], UNIT_SPRITE_SZ, flip_x, 1.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}

/// Age of Empires 風の石彫・青銅器レリーフ背景テクスチャ
fn gen_hud_stone(w: usize, h: usize) -> Vec<u32> {
    let mut cv = Canvas::new(w, h);
    for y in 0..h {
        let fy = y as f32 / h as f32;
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let n1 = fbm(x as f32 * 0.06, y as f32 * 0.06, 101, 4);
            let n2 = fbm(x as f32 * 0.18, y as f32 * 0.18, 202, 3);
            let grain = (n1 - 0.5) * 0.09 + (n2 - 0.5) * 0.05;
            // 石材の目地
            let bx = (x + (y / 32) * 48) % 96;
            let by = y % 32;
            let seam = if bx == 0 || by == 0 {
                -0.045
            } else if bx == 1 || by == 1 {
                0.03
            } else {
                0.0
            };
            let vig = 1.0 - ((fx - 0.5).powi(2) * 0.25 + (fy - 0.5).powi(2) * 0.45);
            let mut base = [0.13 + grain + seam, 0.105 + grain * 0.85 + seam, 0.08 + grain * 0.7 + seam];
            // 上部と下部の青銅色の縁取りグラデーション
            if y < 10 {
                let t = 1.0 - y as f32 / 10.0;
                base = lerp3(base, [0.32, 0.25, 0.14], t * 0.65);
            }
            // 雷紋（古代中国の幾何学文様）の透かし彫り
            if (4..16).contains(&y) {
                let rx = x % 16;
                let ry = y - 4;
                let pat = (ry == 0 || ry == 11 || rx == 0 || rx == 15)
                    || ((2..=9).contains(&ry) && (rx == 2 || rx == 13))
                    || ((ry == 2 || ry == 9) && (2..=13).contains(&rx))
                    || ((4..=7).contains(&ry) && (rx == 4 || rx == 11));
                if pat {
                    base = [base[0] + 0.055, base[1] + 0.042, base[2] + 0.02];
                }
            }
            cv.px[y * w + x] = [(base[0] * vig).clamp(0.0, 1.0), (base[1] * vig).clamp(0.0, 1.0), (base[2] * vig).clamp(0.0, 1.0), 0.97];
        }
    }
    cv.to_bgra_premul()
}

/// 竹簡・黒漆木目調の内部パネル背景テクスチャ
fn gen_panel_wood(w: usize, h: usize) -> Vec<u32> {
    let mut cv = Canvas::new(w, h);
    for y in 0..h {
        let fy = y as f32 / h as f32;
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let wood = fbm(x as f32 * 0.03, y as f32 * 0.22, 303, 4);
            let fine = fbm(x as f32 * 0.25, y as f32 * 0.05, 404, 2);
            let strip = x % 24;
            let groove = if strip == 0 {
                -0.04
            } else if strip == 1 {
                0.025
            } else {
                0.0
            };
            let v = (wood - 0.5) * 0.08 + (fine - 0.5) * 0.03 + groove;
            let vig = 1.0 - ((fx - 0.5).powi(2) + (fy - 0.5).powi(2)) * 0.35;
            let r = (0.16 + v) * vig;
            let g = (0.125 + v * 0.85) * vig;
            let b = (0.09 + v * 0.65) * vig;
            cv.px[y * w + x] = [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), 0.95];
        }
    }
    cv.to_bgra_premul()
}

/// Age of Empires 風のベベル付きコマンドボタン背景
fn gen_button_tex(w: usize, h: usize, state: u8) -> Vec<u32> {
    let mut cv = Canvas::new(w, h);
    let (top_c, bot_c, border_hi, border_lo) = match state {
        1 => ([0.52, 0.41, 0.23], [0.30, 0.22, 0.12], [0.98, 0.86, 0.52], [0.45, 0.34, 0.18]), // hover
        2 => ([0.20, 0.18, 0.16], [0.13, 0.12, 0.10], [0.35, 0.32, 0.28], [0.15, 0.13, 0.11]), // disabled
        _ => ([0.36, 0.28, 0.17], [0.19, 0.14, 0.09], [0.82, 0.68, 0.38], [0.32, 0.24, 0.13]), // normal
    };
    for y in 0..h {
        let ty = y as f32 / (h - 1).max(1) as f32;
        for x in 0..w {
            let n = (fbm(x as f32 * 0.12, y as f32 * 0.2, 555, 3) - 0.5) * 0.06;
            let sheen = if ty < 0.42 { (0.42 - ty) * 0.18 } else { 0.0 };
            let mut c = lerp3(top_c, bot_c, ty);
            c = [c[0] + n + sheen, c[1] + n * 0.9 + sheen * 0.9, c[2] + n * 0.7 + sheen * 0.7];
            let edge_l = x <= 1;
            let edge_t = y <= 1;
            let edge_r = x >= w - 2;
            let edge_b = y >= h - 2;
            if edge_t || edge_l {
                c = border_hi;
            } else if edge_b || edge_r {
                c = border_lo;
            } else if x == 2 || y == 2 || x == w - 3 || y == h - 3 {
                c = [c[0] * 0.65, c[1] * 0.65, c[2] * 0.65];
            }
            cv.px[y * w + x] = [c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0), 0.96];
        }
    }
    // 四隅のリベット飾り
    let rc = if state == 2 { [0.45, 0.42, 0.38, 1.0] } else { [0.95, 0.82, 0.48, 1.0] };
    for (cx, cy) in [(5.5, 5.5), (w as f32 - 5.5, 5.5), (5.5, h as f32 - 5.5), (w as f32 - 5.5, h as f32 - 5.5)] {
        cv.circle(cx, cy, 1.8, rc);
    }
    cv.to_bgra_premul()
}

/// タイトル画面用の春秋戦国・青銅鼎と戈の紋章バナー
fn gen_title_crest() -> Vec<u32> {
    let (w, h) = (340, 124);
    let mut cv = Canvas::new(w, h);
    let (cx, cy) = (w as f32 * 0.5, h as f32 * 0.52);

    // 背後の放射光グラデーション
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 - cx) / 162.0;
            let dy = (y as f32 - cy) / 58.0;
            let d = (dx * dx + dy * dy).sqrt();
            if d < 1.0 {
                let ang = (y as f32 - cy).atan2(x as f32 - cx);
                let rays = 0.5 + 0.5 * (ang * 18.0).cos();
                let a = (1.0 - d).powf(1.4) * (0.42 + 0.22 * rays);
                let c = lerp3([0.92, 0.72, 0.28], [0.38, 0.14, 0.06], d);
                cv.blend(x as i32, y as i32, [c[0], c[1], c[2], a]);
            }
        }
    }

    let wood = [0.48, 0.26, 0.14, 1.0];
    let gold = [0.95, 0.80, 0.38, 1.0];
    let bronze = [0.78, 0.62, 0.32, 1.0];
    let shine = [1.0, 0.94, 0.72, 1.0];

    // 左右に交差する青銅の戈（か）と長柄
    cv.capsule(cx - 148.0, cy + 40.0, cx + 142.0, cy - 42.0, 3.0, wood);
    cv.capsule(cx + 148.0, cy + 40.0, cx - 142.0, cy - 42.0, 3.0, wood);
    // 左上の戈刃
    cv.poly(&[(cx - 142.0, cy - 42.0), (cx - 110.0, cy - 52.0), (cx - 118.0, cy - 38.0)], shine);
    cv.poly(&[(cx - 134.0, cy - 39.0), (cx - 148.0, cy - 18.0), (cx - 122.0, cy - 30.0)], bronze);
    // 右上の戈刃
    cv.poly(&[(cx + 142.0, cy - 42.0), (cx + 110.0, cy - 52.0), (cx + 118.0, cy - 38.0)], shine);
    cv.poly(&[(cx + 134.0, cy - 39.0), (cx + 148.0, cy - 18.0), (cx + 122.0, cy - 30.0)], bronze);

    // 左右の戦旗（朱塗りの軍旗＋金縁）
    let crimson = [0.68, 0.12, 0.10, 0.95];
    let crimson_hi = [0.85, 0.22, 0.16, 0.95];
    cv.poly(&[(cx - 138.0, cy - 26.0), (cx - 82.0, cy - 16.0), (cx - 90.0, cy + 26.0), (cx - 146.0, cy + 16.0)], crimson);
    cv.poly(&[(cx - 134.0, cy - 20.0), (cx - 88.0, cy - 12.0), (cx - 94.0, cy + 20.0), (cx - 140.0, cy + 11.0)], crimson_hi);
    cv.poly(&[(cx + 138.0, cy - 26.0), (cx + 82.0, cy - 16.0), (cx + 90.0, cy + 26.0), (cx + 146.0, cy + 16.0)], crimson);
    cv.poly(&[(cx + 134.0, cy - 20.0), (cx + 88.0, cy - 12.0), (cx + 94.0, cy + 20.0), (cx + 140.0, cy + 11.0)], crimson_hi);

    // 左右の小型青銅鼎メダリオン
    for &ox in &[cx - 114.0, cx + 114.0] {
        cv.circle(ox, cy + 2.0, 14.0, [0.12, 0.09, 0.06, 0.92]);
        cv.ring(ox, cy + 2.0, 13.5, 2.0, gold);
        cv.round_rect(ox - 7.0, cy - 4.0, ox + 7.0, cy + 5.0, 3.0, gold);
        cv.capsule(ox - 5.0, cy + 4.0, ox - 6.0, cy + 10.0, 1.5, bronze);
        cv.capsule(ox, cy + 4.0, ox, cy + 10.0, 1.5, gold);
        cv.capsule(ox + 5.0, cy + 4.0, ox + 6.0, cy + 10.0, 1.5, bronze);
    }

    // 中央の黒漆・金縁扁額（題字がくっきり映える台座）
    cv.round_rect(cx - 96.0, cy - 38.0, cx + 96.0, cy + 42.0, 10.0, [0.07, 0.05, 0.04, 0.94]);
    cv.round_rect(cx - 93.0, cy - 35.0, cx + 93.0, cy + 39.0, 8.0, [0.85, 0.70, 0.34, 0.95]);
    cv.round_rect(cx - 90.0, cy - 32.0, cx + 90.0, cy + 36.0, 6.0, [0.14, 0.09, 0.07, 0.98]);
    // 扁額上部の宝珠・龍紋クレスト
    cv.circle(cx, cy - 36.0, 9.0, gold);
    cv.circle(cx, cy - 36.0, 5.5, [0.28, 0.72, 0.56, 1.0]);

    cv.to_bgra_premul()
}

/// 64x64 アイコンに共通の Age of Empires 風額縁と背景グラデーションを描く
fn icon_backdrop(cv: &mut Canvas, top: [f32; 3], bot: [f32; 3], glow: [f32; 3]) {
    let (w, h) = (cv.w, cv.h);
    for y in 0..h {
        let ty = y as f32 / (h - 1) as f32;
        for x in 0..w {
            let dx = (x as f32 - w as f32 * 0.5) / (w as f32 * 0.5);
            let dy = (y as f32 - h as f32 * 0.45) / (h as f32 * 0.5);
            let r2 = (dx * dx + dy * dy).min(1.2);
            let mut c = lerp3(top, bot, ty);
            c = lerp3(glow, c, r2.sqrt().clamp(0.0, 1.0));
            cv.px[y * w + x] = [c[0], c[1], c[2], 1.0];
        }
    }
}

fn icon_frame(cv: &mut Canvas) {
    let w = cv.w as f32;
    let h = cv.h as f32;
    // 内側の影
    cv.round_rect(0.0, 0.0, w, 4.0, 0.0, [0.98, 0.86, 0.52, 0.9]);
    cv.round_rect(0.0, 0.0, 4.0, h, 0.0, [0.92, 0.78, 0.42, 0.9]);
    cv.round_rect(0.0, h - 4.0, w, h, 0.0, [0.38, 0.28, 0.14, 0.95]);
    cv.round_rect(w - 4.0, 0.0, w, h, 0.0, [0.45, 0.34, 0.16, 0.95]);
    // 内枠の黒線
    cv.capsule(4.0, 4.0, w - 4.0, 4.0, 0.9, [0.08, 0.06, 0.04, 0.85]);
    cv.capsule(4.0, h - 4.0, w - 4.0, h - 4.0, 0.9, [0.08, 0.06, 0.04, 0.85]);
    cv.capsule(4.0, 4.0, 4.0, h - 4.0, 0.9, [0.08, 0.06, 0.04, 0.85]);
    cv.capsule(w - 4.0, 4.0, w - 4.0, h - 4.0, 0.9, [0.08, 0.06, 0.04, 0.85]);
    // 四隅の金鋲
    for (cx, cy) in [(4.5, 4.5), (w - 4.5, 4.5), (4.5, h - 4.5), (w - 4.5, h - 4.5)] {
        cv.circle(cx, cy, 2.2, [1.0, 0.92, 0.62, 1.0]);
    }
}

/// 上部バー用の資源・ステータスアイコン（48x48、背景透過の立体エンブレム）
fn gen_res_icon(kind: ResIcon) -> Vec<u32> {
    let mut cv = Canvas::new(48, 48);
    // 背後の丸いメダル台座
    cv.circle(24.0, 25.0, 21.0, [0.0, 0.0, 0.0, 0.55]);
    cv.circle(24.0, 24.0, 20.0, [0.16, 0.13, 0.09, 0.95]);
    cv.ring(24.0, 24.0, 19.5, 2.2, [0.78, 0.64, 0.34, 1.0]);

    match kind {
        ResIcon::Food => {
            // 背後の黄金の稲穂・粟穂
            let stalk = [0.85, 0.68, 0.22, 1.0];
            let grain = [1.0, 0.88, 0.35, 1.0];
            for &(ax, ay, bx, by) in &[(20.0, 36.0, 12.0, 10.0), (24.0, 36.0, 24.0, 7.0), (28.0, 36.0, 36.0, 10.0)] {
                cv.capsule(ax, ay, bx, by, 1.5, stalk);
                for k in 0..4 {
                    let t = 0.45 + k as f32 * 0.14;
                    let gx = ax + (bx - ax) * t;
                    let gy = ay + (by - ay) * t;
                    cv.circle(gx - 2.2, gy, 2.4, grain);
                    cv.circle(gx + 2.2, gy, 2.4, grain);
                }
            }
            // 手前の山盛り穀物鉢（陶器の高坏）
            cv.circle(24.0, 27.0, 10.5, [0.98, 0.86, 0.38, 1.0]);
            cv.poly(&[(12.0, 27.0), (36.0, 27.0), (31.0, 38.0), (17.0, 38.0)], [0.68, 0.32, 0.18, 1.0]);
            cv.capsule(12.0, 27.0, 36.0, 27.0, 2.0, [0.88, 0.52, 0.28, 1.0]);
            cv.capsule(18.0, 39.0, 30.0, 39.0, 2.0, [0.52, 0.24, 0.14, 1.0]);
        }
        ResIcon::Wood => {
            // 三本の丸太（樹皮と年輪）＋斧の刃
            let bark = [0.42, 0.25, 0.13, 1.0];
            let bark_hi = [0.56, 0.35, 0.18, 1.0];
            let cut = [0.92, 0.76, 0.48, 1.0];
            let ring_c = [0.68, 0.48, 0.24, 1.0];
            for &(x0, y0, x1, y1) in &[(11.0, 31.0, 30.0, 34.0), (18.0, 31.0, 37.0, 34.0), (14.0, 22.0, 34.0, 25.0)] {
                cv.capsule(x0, y0, x1, y1, 5.8, bark);
                cv.capsule(x0, y0 - 1.5, x1, y1 - 1.5, 2.8, bark_hi);
                cv.circle(x0, y0, 5.2, cut);
                cv.ring(x0, y0, 2.8, 1.0, ring_c);
            }
            // 若葉のアクセント
            cv.circle(35.0, 17.0, 4.2, [0.32, 0.72, 0.28, 1.0]);
            cv.circle(38.0, 20.0, 3.2, [0.48, 0.85, 0.35, 1.0]);
        }
        ResIcon::Bronze => {
            // 青銅インゴットと孔雀石（銅鉱）の輝き
            let dark_br = [0.58, 0.32, 0.14, 1.0];
            let mid_br = [0.88, 0.54, 0.24, 1.0];
            let hi_br = [1.0, 0.82, 0.48, 1.0];
            // 下段2つの銅塊
            for &ox in &[16.0, 31.0] {
                cv.poly(&[(ox - 8.0, 35.0), (ox + 8.0, 35.0), (ox + 5.5, 27.0), (ox - 5.5, 27.0)], dark_br);
                cv.poly(&[(ox - 7.0, 34.0), (ox + 7.0, 34.0), (ox + 4.8, 28.0), (ox - 4.8, 28.0)], mid_br);
                cv.capsule(ox - 4.0, 28.5, ox + 4.0, 28.5, 1.2, hi_br);
            }
            // 上段の銅塊
            let ox = 23.5;
            cv.poly(&[(ox - 8.5, 27.5), (ox + 8.5, 27.5), (ox + 6.0, 19.0), (ox - 6.0, 19.0)], dark_br);
            cv.poly(&[(ox - 7.5, 26.5), (ox + 7.5, 26.5), (ox + 5.0, 20.0), (ox - 5.0, 20.0)], mid_br);
            cv.capsule(ox - 4.5, 20.5, ox + 4.5, 20.5, 1.4, hi_br);
            // 緑青（マラカイト）の結晶ハイライト
            cv.poly(&[(21.0, 18.0), (24.0, 10.0), (28.0, 17.5)], [0.35, 0.88, 0.68, 1.0]);
            cv.poly(&[(16.0, 19.0), (19.0, 12.5), (22.5, 18.5)], [0.25, 0.72, 0.55, 1.0]);
        }
        ResIcon::Worker => {
            cv.blit_unit_sprite(0, (4.0, 4.0, 44.0, 44.0), (0.16, 0.0, 0.84, 0.68), Some((24.0, 24.0, 18.5)));
            cv.ring(24.0, 24.0, 19.5, 2.0, [0.95, 0.80, 0.38, 1.0]);
        }
        ResIcon::Soldier => {
            cv.blit_unit_sprite(1, (4.0, 4.0, 44.0, 44.0), (0.14, 0.0, 0.86, 0.72), Some((24.0, 24.0, 18.5)));
            cv.ring(24.0, 24.0, 19.5, 2.0, [0.95, 0.80, 0.38, 1.0]);
        }
        ResIcon::Upkeep => {
            // 兵糧袋と炎
            cv.round_rect(14.0, 18.0, 34.0, 37.0, 6.0, [0.76, 0.62, 0.40, 1.0]);
            cv.capsule(18.0, 18.0, 30.0, 18.0, 2.2, [0.85, 0.22, 0.18, 1.0]);
            cv.poly(&[(17.0, 12.0), (31.0, 12.0), (28.0, 18.0), (20.0, 18.0)], [0.82, 0.68, 0.44, 1.0]);
            cv.circle(24.0, 28.0, 5.0, [0.95, 0.55, 0.20, 1.0]);
            cv.circle(24.0, 28.0, 2.8, [1.0, 0.88, 0.40, 1.0]);
        }
        ResIcon::Era => {
            // 竹簡（ちくかん）の巻物
            for k in 0..5 {
                let x = 13.0 + k as f32 * 5.2;
                cv.round_rect(x, 12.0, x + 4.2, 36.0, 1.2, [0.86, 0.72, 0.44, 1.0]);
            }
            cv.capsule(11.0, 17.0, 37.0, 17.0, 1.4, [0.82, 0.20, 0.16, 1.0]);
            cv.capsule(11.0, 31.0, 37.0, 31.0, 1.4, [0.82, 0.20, 0.16, 1.0]);
        }
    }
    cv.to_bgra_premul()
}

/// 64x64 コマンド＆ポートレートアイコン（Age of Empires 風の絵画調アイコン）
fn gen_cmd_icon(kind: CmdIcon) -> Vec<u32> {
    let mut cv = Canvas::new(64, 64);
    match kind {
        CmdIcon::Worker => {
            icon_backdrop(&mut cv, [0.22, 0.38, 0.52], [0.16, 0.24, 0.14], [0.65, 0.72, 0.48]);
            cv.blit_unit_sprite(0, (3.0, 3.0, 61.0, 61.0), (0.14, 0.0, 0.86, 0.72), None);
        }
        CmdIcon::Infantry => {
            icon_backdrop(&mut cv, [0.38, 0.16, 0.12], [0.16, 0.08, 0.06], [0.82, 0.45, 0.24]);
            cv.blit_unit_sprite(1, (3.0, 3.0, 61.0, 61.0), (0.12, 0.0, 0.88, 0.76), None);
        }
        CmdIcon::Archer => {
            icon_backdrop(&mut cv, [0.14, 0.30, 0.22], [0.08, 0.14, 0.10], [0.42, 0.68, 0.48]);
            cv.blit_unit_sprite(2, (3.0, 3.0, 61.0, 61.0), (0.12, 0.0, 0.88, 0.76), None);
        }
        CmdIcon::Chariot => {
            icon_backdrop(&mut cv, [0.42, 0.28, 0.14], [0.20, 0.12, 0.06], [0.88, 0.66, 0.30]);
            cv.blit_unit_sprite(3, (3.0, 3.0, 61.0, 61.0), (0.04, 0.08, 0.96, 0.96), None);
        }
        CmdIcon::Cavalry => {
            icon_backdrop(&mut cv, [0.22, 0.24, 0.42], [0.12, 0.10, 0.18], [0.64, 0.52, 0.74]);
            cv.blit_unit_sprite(4, (3.0, 3.0, 61.0, 61.0), (0.10, 0.0, 0.92, 0.82), None);
        }
        CmdIcon::Farm => {
            icon_backdrop(&mut cv, [0.28, 0.48, 0.68], [0.26, 0.42, 0.18], [0.75, 0.82, 0.52]);
            cv.blit_bld_sprite(3, (3.0, 3.0, 61.0, 61.0), (0.0, 0.0, 1.0, 1.0));
        }
        CmdIcon::Barracks => {
            icon_backdrop(&mut cv, [0.38, 0.18, 0.14], [0.18, 0.12, 0.10], [0.72, 0.42, 0.26]);
            cv.blit_bld_sprite(2, (3.0, 3.0, 61.0, 61.0), (0.0, 0.0, 1.0, 1.0));
        }
        CmdIcon::Town => {
            icon_backdrop(&mut cv, [0.22, 0.36, 0.52], [0.18, 0.24, 0.16], [0.58, 0.66, 0.52]);
            cv.blit_bld_sprite(1, (3.0, 3.0, 61.0, 61.0), (0.0, 0.0, 1.0, 1.0));
        }
        CmdIcon::Fort => {
            icon_backdrop(&mut cv, [0.46, 0.34, 0.20], [0.20, 0.14, 0.10], [0.85, 0.62, 0.32]);
            cv.blit_bld_sprite(4, (2.0, 2.0, 62.0, 62.0), (0.0, 0.0, 1.0, 1.0));
        }
        CmdIcon::Capital => {
            icon_backdrop(&mut cv, [0.48, 0.32, 0.14], [0.22, 0.12, 0.08], [0.98, 0.82, 0.38]);
            cv.blit_bld_sprite(0, (3.0, 3.0, 61.0, 61.0), (0.0, 0.0, 1.0, 1.0));
        }
        CmdIcon::Upgrade => {
            icon_backdrop(&mut cv, [0.16, 0.28, 0.48], [0.08, 0.12, 0.24], [0.45, 0.68, 0.95]);
            // 上昇シェブロン（金色の矢印）
            for &yo in &[24.0, 36.0] {
                cv.capsule(14.0, yo + 10.0, 32.0, yo - 4.0, 3.0, [1.0, 0.82, 0.30, 0.85]);
                cv.capsule(32.0, yo - 4.0, 50.0, yo + 10.0, 3.0, [1.0, 0.82, 0.30, 0.85]);
            }
            // 中央に輝く青銅の宝剣
            cv.capsule(32.0, 56.0, 32.0, 42.0, 2.8, [0.65, 0.38, 0.18, 1.0]);
            cv.capsule(22.0, 42.0, 42.0, 42.0, 2.8, [0.95, 0.80, 0.36, 1.0]);
            cv.poly(&[(28.0, 41.0), (32.0, 8.0), (36.0, 41.0)], [0.92, 0.96, 1.0, 1.0]);
            cv.poly(&[(32.0, 8.0), (36.0, 41.0), (32.0, 41.0)], [0.68, 0.82, 0.95, 1.0]);
        }
        CmdIcon::Convert => {
            icon_backdrop(&mut cv, [0.24, 0.22, 0.16], [0.12, 0.10, 0.08], [0.55, 0.50, 0.32]);
            // 循環する金の矢印リング
            cv.ring(32.0, 32.0, 20.0, 3.2, [0.95, 0.78, 0.32, 0.9]);
            // 左半分に鍬、右半分に戈
            cv.capsule(16.0, 48.0, 28.0, 16.0, 2.4, [0.62, 0.42, 0.22, 1.0]);
            cv.poly(&[(22.0, 14.0), (32.0, 18.0), (28.0, 24.0)], [0.82, 0.86, 0.90, 1.0]);
            cv.capsule(44.0, 48.0, 38.0, 14.0, 2.4, [0.68, 0.32, 0.18, 1.0]);
            cv.poly(&[(38.0, 16.0), (52.0, 22.0), (39.0, 26.0)], [0.98, 0.84, 0.42, 1.0]);
        }
        CmdIcon::Stop => {
            icon_backdrop(&mut cv, [0.22, 0.20, 0.22], [0.10, 0.09, 0.10], [0.45, 0.42, 0.45]);
            // 堅牢な大盾
            cv.round_rect(14.0, 10.0, 50.0, 54.0, 8.0, [0.78, 0.64, 0.32, 1.0]);
            cv.round_rect(17.0, 13.0, 47.0, 51.0, 6.0, [0.58, 0.18, 0.14, 1.0]);
            cv.circle(32.0, 32.0, 8.5, [0.95, 0.82, 0.42, 1.0]);
            cv.ring(32.0, 32.0, 12.0, 1.8, [0.85, 0.70, 0.34, 1.0]);
        }
        CmdIcon::AttackMove => {
            icon_backdrop(&mut cv, [0.45, 0.14, 0.10], [0.18, 0.06, 0.04], [0.92, 0.38, 0.18]);
            // 突撃の軌跡と剣
            cv.poly(&[(10.0, 52.0), (54.0, 12.0), (42.0, 52.0)], [1.0, 0.55, 0.18, 0.55]);
            cv.capsule(12.0, 52.0, 24.0, 40.0, 3.0, [0.62, 0.36, 0.16, 1.0]);
            cv.capsule(18.0, 34.0, 30.0, 46.0, 2.8, [0.95, 0.80, 0.35, 1.0]);
            cv.poly(&[(22.0, 38.0), (54.0, 10.0), (26.0, 42.0)], [1.0, 0.96, 0.82, 1.0]);
            cv.poly(&[(54.0, 10.0), (38.0, 12.0), (52.0, 26.0)], [1.0, 0.85, 0.38, 1.0]);
        }
        CmdIcon::Demolish => {
            icon_backdrop(&mut cv, [0.32, 0.16, 0.12], [0.14, 0.08, 0.06], [0.85, 0.36, 0.16]);
            // 砕ける石材とツルハシ・炎
            cv.round_rect(12.0, 36.0, 34.0, 52.0, 2.0, [0.55, 0.50, 0.44, 1.0]);
            cv.round_rect(32.0, 40.0, 52.0, 54.0, 2.0, [0.48, 0.44, 0.38, 1.0]);
            cv.capsule(14.0, 48.0, 48.0, 14.0, 2.8, [0.58, 0.36, 0.18, 1.0]);
            cv.capsule(36.0, 10.0, 54.0, 28.0, 3.2, [0.82, 0.85, 0.90, 1.0]);
            cv.circle(32.0, 34.0, 7.0, [1.0, 0.62, 0.18, 0.9]);
        }
        CmdIcon::Diplo => {
            icon_backdrop(&mut cv, [0.28, 0.22, 0.14], [0.14, 0.10, 0.06], [0.78, 0.64, 0.32]);
            // 玉璽（ぎょくじ）と盟約の竹簡
            cv.round_rect(12.0, 16.0, 52.0, 48.0, 3.0, [0.86, 0.72, 0.42, 1.0]);
            cv.circle(32.0, 32.0, 10.0, [0.82, 0.18, 0.14, 1.0]);
            cv.ring(32.0, 32.0, 7.0, 1.5, [1.0, 0.85, 0.55, 1.0]);
        }
        CmdIcon::Help => {
            icon_backdrop(&mut cv, [0.16, 0.26, 0.38], [0.08, 0.12, 0.18], [0.42, 0.64, 0.85]);
            // 兵法書（巻物）
            cv.round_rect(14.0, 12.0, 50.0, 52.0, 4.0, [0.90, 0.80, 0.55, 1.0]);
            cv.capsule(12.0, 12.0, 52.0, 12.0, 3.2, [0.68, 0.42, 0.20, 1.0]);
            cv.capsule(12.0, 52.0, 52.0, 52.0, 3.2, [0.68, 0.42, 0.20, 1.0]);
            cv.circle(32.0, 32.0, 9.0, [0.22, 0.38, 0.62, 1.0]);
        }
        CmdIcon::GatherAuto => {
            icon_backdrop(&mut cv, [0.22, 0.36, 0.24], [0.12, 0.18, 0.10], [0.58, 0.78, 0.42]);
            // 黄金の稲穂・丸太・青銅鉱石の3資源と鍬
            cv.circle(20.0, 42.0, 9.0, [0.95, 0.78, 0.25, 1.0]); // 糧
            cv.round_rect(26.0, 38.0, 46.0, 50.0, 3.5, [0.58, 0.36, 0.18, 1.0]); // 木
            cv.circle(46.0, 44.0, 6.0, [0.82, 0.64, 0.36, 1.0]);
            cv.poly(&[(34.0, 22.0), (52.0, 26.0), (46.0, 38.0), (30.0, 34.0)], [0.32, 0.85, 0.65, 1.0]); // 銅・孔雀石
            // 中央を横切る黄金の鍬
            cv.capsule(12.0, 50.0, 46.0, 14.0, 2.8, [0.68, 0.44, 0.20, 1.0]);
            cv.poly(&[(40.0, 10.0), (54.0, 20.0), (48.0, 26.0), (35.0, 16.0)], [0.95, 0.96, 1.0, 1.0]);
        }
        CmdIcon::GatherWood => {
            icon_backdrop(&mut cv, [0.16, 0.34, 0.18], [0.08, 0.16, 0.08], [0.38, 0.68, 0.32]);
            cv.blit_prop_sprite(1, (3.0, 3.0, 61.0, 61.0), (0.0, 0.0, 1.0, 1.0));
            // 輝く伐採斧のオーバーレイ
            cv.capsule(16.0, 52.0, 48.0, 18.0, 2.6, [0.68, 0.44, 0.22, 1.0]);
            cv.poly(&[(40.0, 14.0), (56.0, 24.0), (46.0, 34.0), (36.0, 22.0)], [0.95, 0.96, 1.0, 1.0]);
        }
        CmdIcon::GatherBronze => {
            icon_backdrop(&mut cv, [0.32, 0.22, 0.14], [0.14, 0.10, 0.06], [0.85, 0.56, 0.24]);
            cv.blit_prop_sprite(2, (3.0, 3.0, 61.0, 61.0), (0.0, 0.0, 1.0, 1.0));
            // ツルハシのオーバーレイ
            cv.capsule(14.0, 50.0, 46.0, 18.0, 2.6, [0.65, 0.42, 0.20, 1.0]);
            cv.capsule(34.0, 12.0, 54.0, 32.0, 2.8, [0.98, 0.90, 0.68, 1.0]);
        }
        CmdIcon::Defend => {
            icon_backdrop(&mut cv, [0.16, 0.28, 0.46], [0.08, 0.14, 0.24], [0.42, 0.68, 0.95]);
            // 城壁の胸壁と大型の青い守護盾
            for k in 0..5 {
                let x = 10.0 + k as f32 * 9.0;
                cv.round_rect(x, 12.0, x + 6.0, 22.0, 1.0, [0.68, 0.62, 0.52, 1.0]);
            }
            cv.round_rect(8.0, 18.0, 56.0, 28.0, 1.5, [0.58, 0.52, 0.44, 1.0]);
            cv.poly(&[(16.0, 22.0), (48.0, 22.0), (44.0, 46.0), (32.0, 58.0), (20.0, 46.0)], [0.95, 0.80, 0.36, 1.0]);
            cv.poly(&[(19.0, 25.0), (45.0, 25.0), (41.0, 44.0), (32.0, 54.0), (23.0, 44.0)], [0.18, 0.38, 0.68, 1.0]);
            cv.circle(32.0, 38.0, 6.0, [1.0, 0.88, 0.45, 1.0]);
        }
        CmdIcon::AllArmy => {
            icon_backdrop(&mut cv, [0.44, 0.16, 0.12], [0.18, 0.06, 0.06], [0.95, 0.45, 0.22]);
            // 軍旗と交差する二振りの青銅戈・兜
            cv.poly(&[(16.0, 10.0), (48.0, 16.0), (16.0, 28.0)], [0.88, 0.20, 0.16, 1.0]);
            cv.capsule(14.0, 54.0, 50.0, 14.0, 2.6, [0.85, 0.68, 0.32, 1.0]);
            cv.capsule(50.0, 54.0, 14.0, 14.0, 2.6, [0.85, 0.68, 0.32, 1.0]);
            cv.circle(32.0, 38.0, 12.0, [0.92, 0.76, 0.36, 1.0]);
            cv.poly(&[(20.0, 38.0), (44.0, 38.0), (46.0, 50.0), (18.0, 50.0)], [0.75, 0.58, 0.26, 1.0]);
            cv.poly(&[(32.0, 16.0), (25.0, 28.0), (39.0, 28.0)], [0.95, 0.22, 0.16, 1.0]);
        }
    }
    icon_frame(&mut cv);
    cv.to_bgra_premul()
}

/// 48x48 マップ上のユニット兵種メダルアイコン（文字「民」「戈」等の代わりに表示する高視認性エンブレム）
fn gen_unit_badge(idx: usize) -> Vec<u32> {
    let mut cv = Canvas::new(48, 48);
    // 半透明のドロップシャドウ
    cv.circle(24.0, 25.5, 21.5, [0.0, 0.0, 0.0, 0.55]);
    let bg = match idx {
        0 => [0.14, 0.26, 0.32, 0.94], // 民：紺青・青磁
        1 => [0.38, 0.12, 0.10, 0.94], // 戈兵：朱漆
        2 => [0.12, 0.30, 0.18, 0.94], // 弩兵：深緑
        3 => [0.38, 0.24, 0.10, 0.94], // 戦車：琥珀・青銅
        _ => [0.18, 0.20, 0.38, 0.94], // 騎兵：群青
    };
    cv.circle(24.0, 24.0, 20.0, bg);
    cv.ring(24.0, 24.0, 19.5, 2.2, [0.92, 0.76, 0.36, 0.98]);
    cv.ring(24.0, 24.0, 17.2, 0.9, [0.45, 0.34, 0.16, 0.85]);

    let uv = match idx {
        0 => (0.04, 0.0, 0.96, 0.72),
        1 => (0.04, 0.0, 0.96, 0.72),
        2 => (0.04, 0.0, 0.96, 0.72),
        3 => (0.0, 0.06, 1.0, 0.94),
        _ => (0.06, 0.0, 0.96, 0.84),
    };
    cv.blit_unit_sprite(idx, (4.0, 4.0, 44.0, 44.0), uv, Some((24.0, 24.0, 18.6)));
    cv.ring(24.0, 24.0, 19.5, 2.2, [0.95, 0.80, 0.38, 0.98]);
    cv.ring(24.0, 24.0, 17.6, 0.9, [0.45, 0.34, 0.16, 0.85]);
    cv.to_bgra_premul()
}

impl Art {
    pub fn new() -> Self {
        Self {
            gen: u32::MAX,
            hud_stone: None,
            panel_wood: None,
            title_crest: None,
            btn_norm: None,
            btn_hover: None,
            btn_dis: None,
            res_icons: Default::default(),
            cmd_icons: Default::default(),
            unit_badges: Default::default(),
            unit_sprites: Default::default(),
            unit_sprites_flip: Default::default(),
            bld_sprites: Default::default(),
            bld_sprites_fog: Default::default(),
            prop_sprites: Default::default(),
            prop_sprites_flip: Default::default(),
            prop_sprites_fog: Default::default(),
            prop_sprites_fog_flip: Default::default(),
            hero_sprites: Default::default(),
            hero_sprites_flip: Default::default(),
            divine_unit_sprites: Default::default(),
            divine_unit_sprites_flip: Default::default(),
            divine_bld_sprites: Default::default(),
            divine_bld_sprites_fog: Default::default(),
        }
    }

    pub fn ensure(&mut self, gfx: &Gfx) {
        if self.gen == gfx.d2d_gen && self.hud_stone.is_some() {
            return;
        }
        self.gen = gfx.d2d_gen;
        self.hud_stone = gfx.create_bitmap(256, 128, &gen_hud_stone(256, 128));
        self.panel_wood = gfx.create_bitmap(256, 128, &gen_panel_wood(256, 128));
        self.title_crest = gfx.create_bitmap(340, 124, &gen_title_crest());
        self.btn_norm = gfx.create_bitmap(128, 48, &gen_button_tex(128, 48, 0));
        self.btn_hover = gfx.create_bitmap(128, 48, &gen_button_tex(128, 48, 1));
        self.btn_dis = gfx.create_bitmap(128, 48, &gen_button_tex(128, 48, 2));

        let res_all = [
            ResIcon::Food,
            ResIcon::Wood,
            ResIcon::Bronze,
            ResIcon::Worker,
            ResIcon::Soldier,
            ResIcon::Upkeep,
            ResIcon::Era,
        ];
        for r in res_all {
            self.res_icons[r as usize] = gfx.create_bitmap(48, 48, &gen_res_icon(r));
        }

        let cmd_all = [
            CmdIcon::Worker,
            CmdIcon::Infantry,
            CmdIcon::Archer,
            CmdIcon::Chariot,
            CmdIcon::Cavalry,
            CmdIcon::Farm,
            CmdIcon::Barracks,
            CmdIcon::Town,
            CmdIcon::Capital,
            CmdIcon::Upgrade,
            CmdIcon::Convert,
            CmdIcon::Stop,
            CmdIcon::AttackMove,
            CmdIcon::Demolish,
            CmdIcon::Diplo,
            CmdIcon::Help,
            CmdIcon::GatherAuto,
            CmdIcon::GatherWood,
            CmdIcon::GatherBronze,
            CmdIcon::Defend,
            CmdIcon::AllArmy,
            CmdIcon::Fort,
        ];
        for c in cmd_all {
            self.cmd_icons[c as usize] = gfx.create_bitmap(64, 64, &gen_cmd_icon(c));
        }
        for b in 0..5 {
            self.unit_badges[b] = gfx.create_bitmap(48, 48, &gen_unit_badge(b));
            let sz = UNIT_SPRITE_SZ as u32;
            self.unit_sprites[b] = gfx.create_bitmap(sz, sz, &load_unit_sprite_bgra(b, false));
            self.unit_sprites_flip[b] = gfx.create_bitmap(sz, sz, &load_unit_sprite_bgra(b, true));
        }
        let bsz = BLD_SPRITE_SZ as u32;
        for b in 0..5 {
            self.bld_sprites[b] = gfx.create_bitmap(bsz, bsz, &load_raw_sprite_bgra(RAW_BLD_SPRITES[b], BLD_SPRITE_SZ, false, 1.0));
            self.bld_sprites_fog[b] = gfx.create_bitmap(bsz, bsz, &load_raw_sprite_bgra(RAW_BLD_SPRITES[b], BLD_SPRITE_SZ, false, 0.52));
        }
        let psz = PROP_SPRITE_SZ as u32;
        for p in 0..3 {
            self.prop_sprites[p] = gfx.create_bitmap(psz, psz, &load_raw_sprite_bgra(RAW_PROP_SPRITES[p], PROP_SPRITE_SZ, false, 1.0));
            self.prop_sprites_flip[p] = gfx.create_bitmap(psz, psz, &load_raw_sprite_bgra(RAW_PROP_SPRITES[p], PROP_SPRITE_SZ, true, 1.0));
            self.prop_sprites_fog[p] = gfx.create_bitmap(psz, psz, &load_raw_sprite_bgra(RAW_PROP_SPRITES[p], PROP_SPRITE_SZ, false, 0.52));
            self.prop_sprites_fog_flip[p] = gfx.create_bitmap(psz, psz, &load_raw_sprite_bgra(RAW_PROP_SPRITES[p], PROP_SPRITE_SZ, true, 0.52));
        }
        let hsz = HERO_SPRITE_SZ as u32;
        for h in 0..31 {
            self.hero_sprites[h] = gfx.create_bitmap(hsz, hsz, &load_raw_sprite_bgra(RAW_HERO_SPRITES[h], HERO_SPRITE_SZ, false, 1.0));
            self.hero_sprites_flip[h] = gfx.create_bitmap(hsz, hsz, &load_raw_sprite_bgra(RAW_HERO_SPRITES[h], HERO_SPRITE_SZ, true, 1.0));
        }
        let usz = UNIT_SPRITE_SZ as u32;
        for d in 0..2 {
            self.divine_unit_sprites[d] = gfx.create_bitmap(usz, usz, &load_raw_sprite_bgra(RAW_DIVINE_UNIT_SPRITES[d], UNIT_SPRITE_SZ, false, 1.0));
            self.divine_unit_sprites_flip[d] = gfx.create_bitmap(usz, usz, &load_raw_sprite_bgra(RAW_DIVINE_UNIT_SPRITES[d], UNIT_SPRITE_SZ, true, 1.0));
        }
        for d in 0..2 {
            self.divine_bld_sprites[d] = gfx.create_bitmap(bsz, bsz, &load_raw_sprite_bgra(RAW_DIVINE_BLD_SPRITES[d], BLD_SPRITE_SZ, false, 1.0));
            self.divine_bld_sprites_fog[d] = gfx.create_bitmap(bsz, bsz, &load_raw_sprite_bgra(RAW_DIVINE_BLD_SPRITES[d], BLD_SPRITE_SZ, false, 0.55));
        }
    }

    pub fn hero_sprite(&self, hero_idx: usize, flip_x: bool) -> Option<&ID2D1Bitmap> {
        if hero_idx >= self.hero_sprites.len() {
            return None;
        }
        if flip_x {
            self.hero_sprites_flip[hero_idx].as_ref()
        } else {
            self.hero_sprites[hero_idx].as_ref()
        }
    }

    pub fn divine_unit_sprite(&self, k: UnitKind, flip_x: bool) -> Option<&ID2D1Bitmap> {
        let idx = match k {
            UnitKind::Chariot => 1,
            _ => 0,
        };
        if flip_x {
            self.divine_unit_sprites_flip[idx].as_ref()
        } else {
            self.divine_unit_sprites[idx].as_ref()
        }
    }

    pub fn divine_bld_sprite(&self, k: BKind, fogged: bool) -> Option<&ID2D1Bitmap> {
        let idx = match k {
            BKind::Capital => 0,
            _ => 1,
        };
        if fogged {
            self.divine_bld_sprites_fog[idx].as_ref()
        } else {
            self.divine_bld_sprites[idx].as_ref()
        }
    }

    pub fn bld_sprite(&self, k: BKind, fogged: bool) -> Option<&ID2D1Bitmap> {
        let idx = match k {
            BKind::Capital => 0,
            BKind::Town => 1,
            BKind::Barracks => 2,
            BKind::Farm => 3,
            BKind::Fort => 4,
        };
        if fogged {
            self.bld_sprites_fog[idx].as_ref()
        } else {
            self.bld_sprites[idx].as_ref()
        }
    }

    pub fn mountain_sprite(&self, flip_x: bool, fogged: bool) -> Option<&ID2D1Bitmap> {
        match (fogged, flip_x) {
            (false, false) => self.prop_sprites[0].as_ref(),
            (false, true) => self.prop_sprites_flip[0].as_ref(),
            (true, false) => self.prop_sprites_fog[0].as_ref(),
            (true, true) => self.prop_sprites_fog_flip[0].as_ref(),
        }
    }

    pub fn forest_sprite(&self, flip_x: bool, fogged: bool) -> Option<&ID2D1Bitmap> {
        match (fogged, flip_x) {
            (false, false) => self.prop_sprites[1].as_ref(),
            (false, true) => self.prop_sprites_flip[1].as_ref(),
            (true, false) => self.prop_sprites_fog[1].as_ref(),
            (true, true) => self.prop_sprites_fog_flip[1].as_ref(),
        }
    }

    pub fn mine_sprite(&self, fogged: bool) -> Option<&ID2D1Bitmap> {
        if fogged {
            self.prop_sprites_fog[2].as_ref()
        } else {
            self.prop_sprites[2].as_ref()
        }
    }

    pub fn unit_sprite(&self, k: UnitKind, era: Era, flip_x: bool) -> Option<&ID2D1Bitmap> {
        let idx = match k {
            UnitKind::Worker => 0,
            UnitKind::Infantry => 1,
            UnitKind::Archer => 2,
            UnitKind::Chariot => {
                if era == Era::Spring {
                    3
                } else {
                    4
                }
            }
        };
        if flip_x {
            self.unit_sprites_flip[idx].as_ref()
        } else {
            self.unit_sprites[idx].as_ref()
        }
    }

    pub fn unit_badge(&self, k: UnitKind, era: Era) -> Option<&ID2D1Bitmap> {
        let idx = match k {
            UnitKind::Worker => 0,
            UnitKind::Infantry => 1,
            UnitKind::Archer => 2,
            UnitKind::Chariot => {
                if era == Era::Spring {
                    3
                } else {
                    4
                }
            }
        };
        self.unit_badges[idx].as_ref()
    }

    pub fn unit_icon(&self, k: UnitKind, era: Era) -> Option<&ID2D1Bitmap> {
        let idx = match k {
            UnitKind::Worker => CmdIcon::Worker,
            UnitKind::Infantry => CmdIcon::Infantry,
            UnitKind::Archer => CmdIcon::Archer,
            UnitKind::Chariot => {
                if era == Era::Spring {
                    CmdIcon::Chariot
                } else {
                    CmdIcon::Cavalry
                }
            }
        };
        self.cmd_icons[idx as usize].as_ref()
    }

    pub fn bld_icon(&self, k: BKind) -> Option<&ID2D1Bitmap> {
        let idx = match k {
            BKind::Farm => CmdIcon::Farm,
            BKind::Barracks => CmdIcon::Barracks,
            BKind::Town => CmdIcon::Town,
            BKind::Capital => CmdIcon::Capital,
            BKind::Fort => CmdIcon::Fort,
        };
        self.cmd_icons[idx as usize].as_ref()
    }

    /// Age of Empires 風の石彫・金枠パネルを描画
    pub fn draw_stone_panel(&self, gfx: &Gfx, x0: f32, y0: f32, x1: f32, y1: f32) {
        if let Some(bmp) = &self.hud_stone {
            // タイル状に敷き詰める
            let tw = 256.0;
            let mut x = x0;
            while x < x1 {
                let nx = (x + tw).min(x1);
                gfx.draw_bitmap(bmp, x, y0, nx, y1);
                x += tw;
            }
        } else {
            gfx.fill_rect(x0, y0, x1, y1, [0.11, 0.09, 0.07, 0.96]);
        }
        // 立体的な金・青銅の二重縁取り
        gfx.stroke_rect(x0 + 0.5, y0 + 0.5, x1 - 0.5, y1 - 0.5, 2.0, [0.78, 0.64, 0.34, 0.95]);
        gfx.stroke_rect(x0 + 3.0, y0 + 3.0, x1 - 3.0, y1 - 3.0, 1.0, [0.35, 0.26, 0.14, 0.9]);
    }

    /// 内部のくぼんだ木目・竹簡ボックスを描画
    pub fn draw_inset_box(&self, gfx: &Gfx, x0: f32, y0: f32, x1: f32, y1: f32) {
        if let Some(bmp) = &self.panel_wood {
            gfx.draw_bitmap(bmp, x0, y0, x1, y1);
        } else {
            gfx.fill_round(x0, y0, x1, y1, 4.0, [0.14, 0.11, 0.08, 0.94]);
        }
        // 彫り込みの影と金枠
        gfx.stroke_rect(x0, y0, x1, y1, 1.5, [0.05, 0.04, 0.03, 0.9]);
        gfx.stroke_rect(x0 + 1.5, y0 + 1.5, x1 - 1.5, y1 - 1.5, 1.0, [0.62, 0.50, 0.26, 0.75]);
    }

    /// 四隅に古代中国・青銅器風の角飾り（雷紋金具）を描画
    pub fn draw_corner_ornaments(&self, gfx: &Gfx, x0: f32, y0: f32, x1: f32, y1: f32, s: f32) {
        let gold = [0.92, 0.78, 0.38, 0.95];
        let dark = [0.22, 0.16, 0.08, 0.95];
        for &(cx, cy, dx, dy) in &[
            (x0, y0, 1.0f32, 1.0f32),
            (x1, y0, -1.0, 1.0),
            (x0, y1, 1.0, -1.0),
            (x1, y1, -1.0, -1.0),
        ] {
            let ax0 = cx.min(cx + dx * s);
            let ax1 = cx.max(cx + dx * s);
            let ay0 = cy.min(cy + dy * 4.0);
            let ay1 = cy.max(cy + dy * 4.0);
            gfx.fill_rect(ax0, ay0, ax1, ay1, gold);
            let bx0 = cx.min(cx + dx * 4.0);
            let bx1 = cx.max(cx + dx * 4.0);
            let by0 = cy.min(cy + dy * s);
            let by1 = cy.max(cy + dy * s);
            gfx.fill_rect(bx0, by0, bx1, by1, gold);
            let kx = cx + dx * 5.0;
            let ky = cy + dy * 5.0;
            gfx.fill_circle(kx, ky, 2.2, dark);
            gfx.fill_circle(kx, ky, 1.3, [1.0, 0.92, 0.60, 1.0]);
        }
    }
}

/// タイル (x, y) 内のサブピクセル (px, py) の色を Age of Empires 風に高精細描画する
pub fn paint_aoe_terrain_pixel(
    terrain: &[crate::geo::Terrain],
    shade: &[f32],
    x: i32,
    y: i32,
    px: i32,
    py: i32,
    scale: i32,
) -> [f32; 3] {
    use crate::geo::{Terrain, MAP_H, MAP_W};

    let fx = x as f32 + (px as f32 + 0.5) / scale as f32;
    let fy = y as f32 + (py as f32 + 0.5) / scale as f32;

    // タイル境界の四角いカクつきを消すための有機的ドメインワープ
    let wx = fx + (fbm(fx * 1.4, fy * 1.4, 911, 2) - 0.5) * 0.52;
    let wy = fy + (fbm(fx * 1.4 + 31.0, fy * 1.4 + 17.0, 912, 2) - 0.5) * 0.52;

    let sample_tile = |tx: i32, ty: i32| -> (Terrain, f32) {
        let cx = tx.clamp(0, MAP_W as i32 - 1);
        let cy = ty.clamp(0, MAP_H as i32 - 1);
        let idx = cy as usize * MAP_W + cx as usize;
        (terrain[idx], shade[idx])
    };

    let is_water = |t: Terrain| matches!(t, Terrain::Sea | Terrain::Lake | Terrain::River);
    let lat_t = (fy / MAP_H as f32).clamp(0.0, 1.0);
    let macro_patch = fbm(fx * 0.35, fy * 0.35, 613, 3);
    let micro1 = fbm(fx * 3.4, fy * 3.4, 701, 3) - 0.5;
    let micro2 = hash2(x * scale + px, y * scale + py, 707) - 0.5;

    let tile_surface_color = |t: Terrain, sh: f32| -> [f32; 3] {
        match t {
            Terrain::Sea => {
                let deep = lerp3([0.05, 0.16, 0.33], [0.10, 0.27, 0.48], sh);
                lerp3(deep, [0.08, 0.22, 0.42], macro_patch * 0.35)
            }
            Terrain::Lake => lerp3([0.13, 0.35, 0.52], [0.18, 0.43, 0.60], macro_patch),
            Terrain::River => [0.16 + micro1 * 0.05, 0.39 + micro1 * 0.06, 0.58 + micro1 * 0.06],
            Terrain::Plain => {
                // 北方の黄土混じりの草原から中原の緑野・江南の瑞々しい草地への自然なグラデーション
                let north = [0.60, 0.56, 0.33];
                let mid = [0.42, 0.55, 0.25];
                let south = [0.30, 0.49, 0.21];
                let mut g_col = if lat_t < 0.45 {
                    lerp3(north, mid, (lat_t / 0.45).clamp(0.0, 1.0))
                } else {
                    lerp3(mid, south, ((lat_t - 0.45) / 0.55).clamp(0.0, 1.0))
                };
                // Age of Empires 風の陽光が差す草むらの濃淡パッチ
                g_col = lerp3(g_col, [g_col[0] * 1.12, g_col[1] * 1.10, g_col[2] * 0.94], (macro_patch - 0.35).clamp(0.0, 0.65));
                lerp3(g_col, [g_col[0] * 0.80, g_col[1] * 0.82, g_col[2] * 0.78], sh)
            }
            Terrain::Steppe => {
                let base = lerp3([0.68, 0.60, 0.37], [0.55, 0.49, 0.29], sh);
                [base[0] + micro1 * 0.08, base[1] + micro1 * 0.07, base[2] + micro1 * 0.05]
            }
            Terrain::Forest => {
                // 樹林の下草・苔・腐葉土の深い緑（上にAI生成森林スプライトが重なる）
                let ground = [0.15, 0.29, 0.13];
                let moss = [0.22, 0.39, 0.16];
                let sun_leaf = [0.30, 0.47, 0.20];
                let tree_n = fbm(fx * 3.8, fy * 3.8, 909, 3);
                if tree_n > 0.64 {
                    sun_leaf
                } else if tree_n > 0.36 {
                    moss
                } else {
                    ground
                }
            }
            Terrain::Hill => {
                let h0 = fbm(fx * 1.6, fy * 1.6, 606, 3);
                let h1 = fbm((fx + 0.15) * 1.6, (fy + 0.15) * 1.6, 606, 3);
                let slope = (h0 - h1) * 3.2;
                let base = lerp3([0.55, 0.47, 0.30], [0.44, 0.38, 0.24], sh);
                [
                    (base[0] + slope * 0.18).clamp(0.1, 0.9),
                    (base[1] + slope * 0.16).clamp(0.1, 0.9),
                    (base[2] + slope * 0.12).clamp(0.1, 0.9),
                ]
            }
            Terrain::Mountain => {
                // 山岳基部の岩盤と山裾の陰影（上にAI生成山岳スプライトが重なる）
                let ridge = fbm(fx * 2.2, fy * 2.2, 505, 4);
                let ridge_se = fbm((fx + 0.12) * 2.2, (fy + 0.12) * 2.2, 505, 4);
                let lit = (ridge - ridge_se) * 3.6;
                let rock = lerp3([0.34, 0.31, 0.28], [0.50, 0.46, 0.40], ridge);
                [
                    (rock[0] + lit * 0.18).clamp(0.12, 0.85),
                    (rock[1] + lit * 0.17).clamp(0.12, 0.85),
                    (rock[2] + lit * 0.15).clamp(0.12, 0.85),
                ]
            }
        }
    };

    // 周囲4タイルの双線形スムーズステップ補間でタイルの四角い境界を完全に解消する
    let gx = wx - 0.5;
    let gy = wy - 0.5;
    let x0 = gx.floor() as i32;
    let y0 = gy.floor() as i32;
    let tx = gx - x0 as f32;
    let ty = gy - y0 as f32;
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);

    let (t00, s00) = sample_tile(x0, y0);
    let (t10, s10) = sample_tile(x0 + 1, y0);
    let (t01, s01) = sample_tile(x0, y0 + 1);
    let (t11, s11) = sample_tile(x0 + 1, y0 + 1);

    let c00 = tile_surface_color(t00, s00);
    let c10 = tile_surface_color(t10, s10);
    let c01 = tile_surface_color(t01, s01);
    let c11 = tile_surface_color(t11, s11);

    let c_top = lerp3(c00, c10, sx);
    let c_bot = lerp3(c01, c11, sx);
    let mut c = lerp3(c_top, c_bot, sy);

    let w00 = if is_water(t00) { 1.0 } else { 0.0 };
    let w10 = if is_water(t10) { 1.0 } else { 0.0 };
    let w01 = if is_water(t01) { 1.0 } else { 0.0 };
    let w11 = if is_water(t11) { 1.0 } else { 0.0 };
    let water_w = (w00 * (1.0 - sx) + w10 * sx) * (1.0 - sy) + (w01 * (1.0 - sx) + w11 * sx) * sy;

    // 海岸線・川岸の砂浜と浅瀬の美しいグラデーション
    if water_w > 0.04 && water_w < 0.96 {
        let sand = [0.76, 0.68, 0.46];
        let shallow = [0.22, 0.48, 0.60];
        if water_w < 0.48 {
            // 陸側の砂州・水際
            let beach_t = ((water_w - 0.04) / 0.44).clamp(0.0, 1.0);
            c = lerp3(c, sand, beach_t * 0.78);
        } else {
            // 水側の浅瀬と白いさざ波
            let shoal_t = ((0.96 - water_w) / 0.48).clamp(0.0, 1.0);
            c = lerp3(c, shallow, shoal_t * 0.72);
            let foam = ((water_w - 0.48).abs() < 0.08) as i32 as f32;
            if foam > 0.0 && fbm(fx * 4.0, fy * 4.0, 808, 2) > 0.46 {
                c = lerp3(c, [0.84, 0.90, 0.92], 0.40);
            }
        }
    }

    // 水面のきらめき・草地の微細ディテール
    if water_w >= 0.5 {
        let ripple = ((fx * 3.5 + fy * 1.2).sin() * (fx * 1.8 - fy * 4.0).cos()).max(0.0);
        if ripple > 0.75 {
            c = [c[0] + 0.08, c[1] + 0.10, c[2] + 0.12];
        }
    } else {
        c = [
            c[0] + micro1 * 0.055 + micro2 * 0.03,
            c[1] + micro1 * 0.055 + micro2 * 0.03,
            c[2] + micro1 * 0.045 + micro2 * 0.02,
        ];
    }

    [c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0)]
}

fn c4(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

fn darker(c: [f32; 3], k: f32) -> [f32; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

fn lighter(c: [f32; 3], k: f32) -> [f32; 3] {
    [(c[0] + k).min(1.0), (c[1] + k).min(1.0), (c[2] + k).min(1.0)]
}

fn quad(g: &mut Gfx, a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32), col: [f32; 4]) {
    g.tri(a, b, c, col);
    g.tri(a, c, d, col);
}

/// マップ外周の Age of Empires 風木製・真鍮フレームを描く
pub fn draw_map_frame(g: &mut Gfx, mw: f32, mh: f32) {
    let b1 = 3.5;
    let b2 = 1.2;
    // 外側の重厚な黒檀・木枠
    g.rect(-b1, -b1, mw + b1, 0.0, [0.14, 0.10, 0.07, 1.0]);
    g.rect(-b1, mh, mw + b1, mh + b1, [0.10, 0.07, 0.05, 1.0]);
    g.rect(-b1, 0.0, 0.0, mh, [0.13, 0.09, 0.06, 1.0]);
    g.rect(mw, 0.0, mw + b1, mh, [0.09, 0.06, 0.04, 1.0]);
    // 内側の金・真鍮トリム
    g.rect_outline(-b2, -b2, mw + b2, mh + b2, 0.45, [0.78, 0.64, 0.34, 1.0]);
    g.rect_outline(-0.15, -0.15, mw + 0.15, mh + 0.15, 0.2, [0.25, 0.18, 0.10, 1.0]);
}

/// 遠景（zoom < 5.0）用の森林・山脈の簡易立体表現（zoom >= 5.0 では Direct2D 側でAI生成スプライトを描画）
pub fn draw_world_terrain_props(
    g: &mut Gfx,
    map: &crate::geo::Map,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    zoom: f32,
    fog_at: impl Fn(crate::world::V2) -> u8,
) {
    use crate::geo::{Map, Terrain};
    use crate::world::v2;

    if zoom >= 5.0 || zoom < 3.5 {
        return;
    }
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            if !Map::inb(tx, ty) {
                continue;
            }
            let center = v2(tx as f32 + 0.5, ty as f32 + 0.5);
            if fog_at(center) == 0 {
                continue;
            }
            if map.terrain[Map::idx(tx, ty)] == Terrain::Mountain {
                let px = tx as f32 + 0.48;
                let py = ty as f32 + 0.18;
                let by = ty as f32 + 0.94;
                g.tri((px, py), (tx as f32 - 0.1, by), (px + 0.05, by), [0.54, 0.49, 0.43, 0.85]);
                g.tri((px, py), (px + 0.05, by), (tx as f32 + 1.1, by), [0.26, 0.23, 0.21, 0.88]);
            }
        }
    }
}

/// 銅山の下地影（zoom < 5.0 ではポリゴン描画、zoom >= 5.0 では Direct2D 側でAI生成スプライトを描画）
pub fn draw_world_mine(g: &mut Gfx, c: crate::world::V2, amount: f32, px: f32) {
    let s = 0.36 + 0.16 * (amount / 2500.0).clamp(0.2, 1.0);
    g.circle(c.x + 0.05, c.y + 0.12, s * 0.95, 10, [0.0, 0.0, 0.0, 0.30]);
    if px <= 1.0 / 5.0 {
        return;
    }
    g.tri((c.x - s, c.y + s * 0.65), (c.x - s * 0.35, c.y - s * 0.85), (c.x + s * 0.2, c.y + s * 0.65), [0.52, 0.44, 0.36, 1.0]);
    g.tri((c.x - s * 0.35, c.y - s * 0.85), (c.x + s * 0.45, c.y - s * 0.65), (c.x + s, c.y + s * 0.65), [0.36, 0.30, 0.24, 1.0]);
    g.circle(c.x, c.y, s * 0.32, 6, [0.32, 0.85, 0.65, 1.0]);
}

/// 建物（都・邑・兵営・田）の地面基壇・国色境界を描画（本体は Direct2D 側でAI生成スプライトを描画）
pub fn draw_world_building(g: &mut Gfx, b: &crate::world::Building, nc: [f32; 3], px: f32) {
    let (x0, y0, x1, y1) = (b.tx as f32, b.ty as f32, b.tx as f32 + 2.0, b.ty as f32 + 2.0);
    let c = b.center();
    let alpha = if b.done() { 1.0 } else { 0.62 };

    if px <= 1.0 / 2.2 {
        // 建物の地面落影と国色トリム台座（上部に Direct2D で高精細AI生成建物スプライトを重ねる）
        match b.kind {
            BKind::Farm => {
                g.rect(x0 + 0.06, y0 + 0.06, x1 - 0.06, y1 - 0.06, [0.28, 0.22, 0.13, 0.45 * alpha]);
                g.rect_outline(x0 + 0.04, y0 + 0.04, x1 - 0.04, y1 - 0.04, (1.6 * px).max(0.05), c4(nc, 0.72 * alpha));
            }
            BKind::Barracks | BKind::Fort => {
                g.circle(c.x + 0.06, c.y + 0.14, 1.02, 16, [0.0, 0.0, 0.0, 0.34 * alpha]);
                g.rect_outline(x0 + 0.08, y0 + 0.08, x1 - 0.08, y1 - 0.08, (1.8 * px).max(0.06), c4(nc, 0.78 * alpha));
            }
            BKind::Town => {
                g.circle(c.x + 0.08, c.y + 0.16, 1.18, 18, [0.0, 0.0, 0.0, 0.38 * alpha]);
                g.rect_outline(x0 + 0.02, y0 + 0.02, x1 - 0.02, y1 - 0.02, (2.0 * px).max(0.07), c4(nc, 0.82 * alpha));
            }
            BKind::Capital => {
                g.circle(c.x + 0.10, c.y + 0.18, 1.42, 20, [0.0, 0.0, 0.0, 0.42 * alpha]);
                g.rect_outline(x0 - 0.14, y0 - 0.14, x1 + 0.14, y1 + 0.14, (2.2 * px).max(0.08), c4(nc, 0.88 * alpha));
            }
        }
        return;
    }

    // 超広域ズームアウト時（zoom < 5.0）のフォールバック描画
    let e = if b.kind == BKind::Capital { -0.25 } else { 0.0 };
    g.rect(x0 + e, y0 + e, x1 - e, y1 - e, c4(darker(nc, 0.75), alpha));
    g.rect_outline(x0 + e, y0 + e, x1 - e, y1 - e, (1.8 * px).max(0.06), [0.92, 0.78, 0.36, alpha]);
}

/// ユニット（民・戈兵・弩兵・戦車／騎兵）の足元リング・落影を描画（本体は Direct2D 側でAI生成スプライトを描画）
pub fn draw_world_unit(
    g: &mut Gfx,
    u: &crate::world::Unit,
    nc: [f32; 3],
    era: Era,
    res_colors: &[[f32; 4]; 3],
    px: f32,
) {
    use crate::world::v2;

    let p = u.pos;
    let body = if u.flash > 0.0 { [1.0, 1.0, 1.0, 1.0] } else { c4(nc, 1.0) };
    let dark = c4(darker(nc, 0.32), 1.0);
    let bronze = [0.92, 0.76, 0.36, 1.0];
    let wood = [0.52, 0.34, 0.18, 1.0];

    let sz = 0.82 + 0.06 * (u.power - 1.0).min(5.0);
    let d = if u.dir.len() > 0.01 { u.dir.norm() } else { v2(0.0, 1.0) };
    let n = v2(-d.y, d.x);
    let pt = |fwd: f32, side: f32| -> (f32, f32) {
        let v = p + d * (fwd * sz) + n * (side * sz);
        (v.x, v.y)
    };

    // 足元の落影と国色の台座リング（縮小したユニットサイズに合わせてコンパクト化）
    g.circle(p.x + 0.03, p.y + 0.10, 0.25 * sz, 12, [0.0, 0.0, 0.0, 0.32]);
    if u.selected {
        g.ring(p.x, p.y + 0.08, 0.38, (2.0 * px).max(0.045), 18, [0.35, 1.0, 0.45, 0.98]);
        g.ring(p.x, p.y + 0.08, 0.33, px.max(0.025), 18, [0.95, 1.0, 0.75, 0.65]);
    }
    if px <= 1.0 / 6.5 {
        // ズーム時は Direct2D 側で高精細AI生成ユニットスプライトを描画するため、足元に国色台座リングのみ描画
        g.circle(p.x, p.y + 0.08, 0.22 * sz, 12, [nc[0] * 0.65, nc[1] * 0.65, nc[2] * 0.65, 0.48]);
        g.ring(p.x, p.y + 0.08, 0.24 * sz, (1.5 * px).max(0.038), 14, [nc[0] * 0.65 + 0.35, nc[1] * 0.65 + 0.35, nc[2] * 0.65 + 0.35, 0.92]);
        if u.flash > 0.0 {
            g.circle(p.x, p.y, 0.28 * sz, 12, [1.0, 0.95, 0.85, 0.55]);
        }
        return;
    } else if !u.selected {
        g.ring(p.x, p.y, 0.24 * sz, (1.2 * px).max(0.03), 10, [nc[0], nc[1], nc[2], 0.55]);
    }

    match u.kind {
        UnitKind::Worker => {
            let r = 0.20 * sz;
            g.circle(p.x, p.y, r + 1.5 * px, 10, dark);
            g.circle(p.x, p.y, r, 10, body);
            if u.carry > 0.5 {
                let cc = res_colors[u.carry_kind];
                let (bx, by) = pt(-0.16, -0.18);
                g.circle(bx, by, 0.10 * sz, 8, cc);
            }
        }
        UnitKind::Infantry | UnitKind::Archer => {
            let r = 0.22 * sz;
            g.circle(p.x, p.y, r + 1.5 * px, 10, dark);
            g.circle(p.x, p.y, r, 10, body);
            g.circle(p.x, p.y, r * 0.48, 8, bronze);
        }
        UnitKind::Chariot => {
            let _ = (era, wood);
            let r = 0.25 * sz;
            g.circle(p.x, p.y, r + 1.5 * px, 10, dark);
            g.circle(p.x, p.y, r, 10, body);
            g.circle(p.x, p.y, r * 0.52, 8, bronze);
        }
    }
}


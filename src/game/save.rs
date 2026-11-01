//! 記録（セーブ・ロード）：画面・自動記録・ファイルへの書き出しと読み込み
//!
//! 記録の置き場所は platform が決める（Windows 版は %APPDATA% のファイル）。枠 0 は自動記録（10年ごと）、1〜3 は手動の記録。

use super::*;
use crate::platform;
use crate::save::{RResult, Reader, Writer};
use crate::world::SAVE_VERSION;

const MAGIC: &[u8; 4] = b"YYSV";
/// 枠の数（0=自動記録）
pub(super) const SLOTS: u8 = 4;
/// 自動記録の間隔（年）
const AUTOSAVE_YEARS: f32 = 10.0;

fn slot_name(s: u8) -> &'static str {
    match s {
        0 => t!("自動記録", "Autosave", "自动存档"),
        1 => t!("記録 一", "Slot 1", "存档 一"),
        2 => t!("記録 二", "Slot 2", "存档 二"),
        _ => t!("記録 三", "Slot 3", "存档 三"),
    }
}

impl Game {
    /// 記録を作れるか（できない理由）
    pub(super) fn can_save(&self) -> Result<(), &'static str> {
        if self.screen != Screen::Playing {
            return Err(t!("対局中でなければ記録できない", "You can only save during a game", "只有在对局中才能保存"));
        }
        if self.tut.is_some() {
            return Err(t!("チュートリアル中は記録できない", "Cannot save during the tutorial", "教程中无法保存"));
        }
        if self.world.game_over.is_some() {
            return Err(t!("勝敗が決したので記録できない", "The game is over and cannot be saved", "胜负已分，无法保存"));
        }
        Ok(())
    }

    /// 記録の一覧に出す見出し
    fn save_meta(&self) -> String {
        let n = &self.world.nations[self.world.player];
        let era = if self.world.era == Era::Spring { t!("春秋", "Spring & Autumn", "春秋") } else { t!("戦国", "Warring States", "战国") };
        let mut s = tf!("{}　紀元前{}年（{}）　城邑{}", "{}  {} BC ({})  {} cities", "{}　公元前{}年（{}）　城邑{}", crate::lang::tx(n.name), self.world.year_bc(), era, n.cities);
        if self.world.heroic {
            s.push_str(t!("　英霊", "  Spirits", "　英灵"));
        }
        if self.world.descent.is_some() {
            s.push_str(t!("　上帝降臨", "  Descent", "　上帝降临"));
        }
        s
    }

    fn save_bytes(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.bytes(MAGIC);
        w.u32(SAVE_VERSION);
        // 版の違いを見分けるための寸法
        w.usize(MAP_W);
        w.usize(MAP_H);
        w.usize(NATIONS.len());
        w.usize(HISTORY.len());
        w.usize(TRIBES.len());
        w.usize(HEROES.len());
        w.u32(self.seed);
        // 画面の状態
        w.f32(self.cam.x);
        w.f32(self.cam.y);
        w.f32(self.zoom);
        w.u32(self.speed);
        w.rle(&self.fog);
        w.usize(self.groups.len());
        for g in self.groups.iter() {
            w.usize(g.len());
            for h in g.iter() {
                w.u32(h.idx);
                w.u32(h.gen);
            }
        }
        self.world.save_to(&mut w);
        w.buf
    }

    /// 記録を読み込んで対局を再開する
    pub fn load_bytes(&mut self, data: &[u8]) -> RResult<()> {
        let mut r = Reader::new(data);
        if r.bytes(4).ok() != Some(&MAGIC[..]) {
            return Err(t!("春秋戦国の記録ではない", "This is not a Warring States save", "这不是春秋战国的存档").into());
        }
        let ver = r.u32()?;
        if ver != SAVE_VERSION {
            return Err(tf!("記録の形式が違う（記録は第{}版、このゲームは第{}版）", "Different save format (the save is version {}, this game is version {})", "存档格式不同（存档为第{}版，本游戏为第{}版）", ver, SAVE_VERSION));
        }
        let dims = [MAP_W, MAP_H, NATIONS.len(), HISTORY.len(), TRIBES.len(), HEROES.len()];
        for d in dims {
            if r.usize()? != d {
                return Err(t!("別の版のゲームで作られた記録で、読み込めない", "This save was made by another version of the game and cannot be loaded", "该存档由其他版本的游戏创建，无法读取").into());
            }
        }
        let seed = r.u32()?;
        let cam = v2(r.f32()?, r.f32()?);
        let zoom = r.f32()?;
        let speed = r.u32()?;
        let fog = r.rle(MAP_W * MAP_H)?;
        let ng = r.len(100)?;
        let mut groups = Vec::with_capacity(ng);
        for _ in 0..ng {
            let k = r.len(100_000)?;
            let mut g = Vec::with_capacity(k);
            for _ in 0..k {
                g.push(Handle { idx: r.u32()?, gen: r.u32()? });
            }
            groups.push(g);
        }
        let world = World::load_from(seed, &mut r)?;

        // ---- 対局の画面に戻す
        self.world = world;
        self.seed = seed;
        self.tut = None;
        self.screen = Screen::Playing;
        self.title_sel = Some(self.world.player);
        self.heroic_opt = self.world.heroic;
        self.rebuild_terrain();
        self.cam = cam;
        self.zoom = if zoom.is_finite() { zoom.clamp(1.2, 64.0) } else { 22.0 };
        self.clamp_cam();
        self.speed = match speed {
            1 | 2 | 4 | 8 => speed,
            _ => 1,
        };
        self.paused = false;
        self.fog = fog;
        self.fog_t = 0.0;
        self.update_fog(0.0);
        groups.resize(10, Vec::new());
        self.groups = groups;
        self.sel.clear();
        self.sel_b = None;
        self.last_sel.clear();
        self.mode = Mode::Normal;
        self.cmd_pause = false;
        self.last_orders = self.world.player_orders;
        self.acc = 0.0;
        self.marker = None;
        self.ldown = None;
        self.mdrag = None;
        self.mm_drag = false;
        self.show_diplo = false;
        self.show_help = false;
        self.show_chron = false;
        self.show_market = false;
        self.show_summon = false;
        self.show_save = false;
        self.overlay_ver = u32::MAX;
        self.minimap_ver = u32::MAX;
        self.autosave_years = self.world.years;
        Ok(())
    }

    /// 枠に記録する（quiet なら通知を控えめに）
    pub(super) fn save_slot(&mut self, slot: u8, quiet: bool) {
        if let Err(e) = self.can_save() {
            if !quiet {
                self.notice(e);
            }
            return;
        }
        let data = self.save_bytes();
        let meta = self.save_meta();
        platform::save_slot(slot, &data, &meta);
        self.autosave_years = self.world.years;
        if !quiet {
            self.notice(&tf!("{}に記録した（{}）", "Saved to {} ({})", "已保存到{}（{}）", slot_name(slot), meta));
        }
    }

    /// 自動記録（10年ごと）
    pub(super) fn autosave_tick(&mut self) {
        if self.screen == Screen::Playing && self.can_save().is_ok() && self.world.years - self.autosave_years >= AUTOSAVE_YEARS {
            self.save_slot(0, true);
        }
    }

    /// ブラウザ側からの読み込み結果
    pub fn apply_loaded(&mut self, data: &[u8]) {
        match self.load_bytes(data) {
            Ok(()) => {
                let meta = self.save_meta();
                self.notice(&tf!("記録を読み込んだ（{}）", "Loaded ({})", "已读取存档（{}）", meta));
                let s = tf!("記録から再開した：{}", "Resumed from a save: {}", "从存档继续：{}", meta);
                self.world.push_msg(s, [0.8, 0.95, 1.0]);
            }
            Err(e) => {
                self.notice(&tf!("読み込めなかった：{}", "Could not load: {}", "无法读取：{}", e));
            }
        }
    }

    /// ブラウザ側からの通知
    pub fn notify(&mut self, s: &str) {
        self.notice(s);
    }

    pub(super) fn render_save(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pw = 820.0f32.min(w - 20.0);
        let ph = 470.0f32.min(h - 40.0);
        let x0 = (w - pw) / 2.0;
        let y0 = (h - ph) / 2.0;
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.45]);
        // 下の画面のボタンを押せないようにする
        self.buttons.push(Button {
            x0: 0.0,
            y0: 0.0,
            x1: w,
            y1: h,
            label: BLOCK_LABEL.to_string(),
            key: 0,
            enabled: false,
            act: Act::Noop,
            tip: String::new(),
        });
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + ph);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + ph - 3.0, 16.0);
        let title = t!("記録", "Saves", "存档");
        let nx = x0 + 24.0 + self.gfx.measure(title, Font::Big) + 18.0;
        self.gfx.text_shadow(title, x0 + 24.0, y0 + 10.0, 160.0, 34.0, Font::Big, [1.0, 0.85, 0.5, 1.0]);
        self.gfx.text(
            platform::storage_note(),
            nx,
            y0 + 14.0,
            x0 + pw - 110.0 - nx,
            34.0,
            Font::Small,
            [0.85, 0.82, 0.75, 1.0],
        );
        self.add_button(x0 + pw - 90.0, y0 + 10.0, x0 + pw - 14.0, y0 + 40.0, t!("閉じる", "Close", "关闭"), "", 0, true, Act::CloseOverlay, "");

        let can = self.can_save();
        let playing = self.screen == Screen::Playing && self.tut.is_none();
        let tc = [0.95, 0.92, 0.85, 1.0];
        let dim = [0.62, 0.60, 0.55, 1.0];
        let gold = [1.0, 0.85, 0.5, 1.0];
        let (bx0, bx1) = (x0 + 14.0, x0 + pw - 14.0);
        let mut y = y0 + 58.0;
        let row = ((ph - 140.0) / SLOTS as f32).min(78.0);
        for s in 0..SLOTS {
            self.art.draw_inset_box(&self.gfx, bx0, y, bx1, y + row - 8.0);
            let info = platform::slot_info(s);
            self.gfx.text(slot_name(s), bx0 + 14.0, y + 8.0, 110.0, 26.0, Font::Big, gold);
            let tx = bx0 + 130.0;
            let bw = 92.0;
            let bxr = bx1 - 10.0;
            let text_w = (bxr - 3.0 * (bw + 6.0)) - tx - 8.0;
            match &info {
                Some((meta, date)) => {
                    self.gfx.text(meta, tx, y + 8.0, text_w, 22.0, Font::Normal, tc);
                    self.gfx.text(&tf!("保存 {}", "Saved {}", "保存于 {}", date), tx, y + 32.0, text_w, 20.0, Font::Small, dim);
                }
                None => {
                    let t = if s == 0 { t!("（まだ無い。対局中は10年ごとに自動で記録する）", "(none yet; during a game it saves every 10 years)", "（暂无。对局中每10年自动保存）") } else { t!("（空き）", "(empty)", "（空）") };
                    self.gfx.text(t, tx, y + 18.0, text_w, 22.0, Font::Normal, dim);
                }
            }
            let by = y + (row - 8.0) / 2.0 - 16.0;
            // 右から：書き出す・読込・保存
            let has = info.is_some();
            let ex = bxr - bw;
            self.add_button(ex, by, ex + bw, by + 32.0, t!("書き出す", "Export", "导出"), "", 0, has, Act::ExportSlot(s), t!("この記録をファイルとして保存する（別の端末へ移す・控えにする）", "Save this as a file (to move it to another PC or keep a backup)", "将此存档保存为文件（用于转移到其他电脑或备份）"));
            let lx = ex - bw - 6.0;
            let tip = if playing { t!("この記録から再開する（いまの対局は失われる。必要なら先に記録する）", "Resume from this save (the current game will be lost; save it first if needed)", "从此存档继续（当前对局将丢失，如有需要请先保存）") } else { t!("この記録から再開する", "Resume from this save", "从此存档继续") };
            self.add_button(lx, by, lx + bw, by + 32.0, t!("読込", "Load", "读取"), "", 0, has, Act::LoadSlot(s), tip);
            if s > 0 {
                let sx = lx - bw - 6.0;
                let tip = match can {
                    Ok(()) => {
                        if has {
                            t!("いまの対局をこの枠に記録する（上書き）", "Save the current game in this slot (overwrite)", "将当前对局保存到此栏位（覆盖）")
                        } else {
                            t!("いまの対局をこの枠に記録する", "Save the current game in this slot", "将当前对局保存到此栏位")
                        }
                    }
                    Err(e) => e,
                };
                self.add_button(sx, by, sx + bw, by + 32.0, t!("保存", "Save", "保存"), "", 0, can.is_ok(), Act::SaveSlot(s), tip);
            }
            y += row;
        }
        let by = y0 + ph - 54.0;
        self.add_button(bx0, by, bx0 + 260.0, by + 40.0, t!("ファイルから読み込む", "Load from file", "从文件读取"), "", 0, true, Act::ImportFile, t!("書き出した記録ファイル（.yysav）を選んで再開する", "Choose an exported save file (.yysav) to resume", "选择导出的存档文件（.yysav）继续游戏"));
        if let Err(e) = can {
            if self.screen == Screen::Playing {
                self.gfx.text(e, bx0 + 280.0, by + 10.0, bx1 - bx0 - 290.0, 22.0, Font::Normal, dim);
            }
        } else {
            self.gfx.text(t!("F5 でこの画面を開く", "F5 opens this screen", "按 F5 打开此画面"), bx0 + 280.0, by + 10.0, bx1 - bx0 - 290.0, 22.0, Font::Normal, dim);
        }
        // 説明（ツールチップ）：対局中は共通の仕組みで出るので、タイトル画面だけ
        if self.screen != Screen::Title {
            return;
        }
        let (mx, my) = self.mouse;
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
        if let Some(t) = tip {
            let tw = 420.0f32.min(pw - 40.0);
            let th = self.gfx.measure_h(&t, Font::Small, tw - 16.0) + 10.0;
            let tx0 = (mx + 14.0).min(w - tw - 6.0);
            let ty0 = (my + 18.0).min(h - th - 6.0);
            self.art.draw_inset_box(&self.gfx, tx0, ty0, tx0 + tw, ty0 + th);
            self.gfx.text(&t, tx0 + 8.0, ty0 + 5.0, tw - 16.0, th, Font::Small, [0.95, 0.92, 0.85, 1.0]);
        }
    }
}

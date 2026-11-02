//! スマートフォン向けの画面とタッチ操作
//!
//! 画面が小さいとき（ブラウザ側が「詰めた画面」を指示したとき）は、UI を約 1000×470 に収めて文字を大きくする。
//! 縦に収まらない一覧（外交・交易・史記・ヘルプ・記録・英霊）は、高さ 720 の仮の画面に描いて指でなぞってスクロールする。

use super::*;

/// 一覧を描く仮の画面の高さ（パソコン版の最小の高さ）
const VIRTUAL_H: u32 = 720;
/// スマートフォンの上部バーの高さ（画面上端は OS やブラウザが触れたタッチを横取りしやすいので、ボタンを大きく下まで伸ばす）
pub(super) const TOP_H_COMPACT: f32 = 50.0;

impl Game {
    /// 一覧の画面を開いているか
    pub(super) fn overlay_open(&self) -> bool {
        self.show_diplo || self.show_help || self.show_chron || self.show_market || self.show_summon || self.show_save
    }

    /// いま開いている一覧の種類（変わったらスクロールを戻す）
    pub(super) fn overlay_key(&self) -> u8 {
        if self.show_save {
            6
        } else if self.show_summon {
            5
        } else if self.show_market {
            4
        } else if self.show_chron {
            3
        } else if self.show_help {
            2
        } else if self.show_diplo {
            1
        } else {
            0
        }
    }

    /// 一覧を描く。詰めた画面で縦に収まらなければ、高さ 720 の仮の画面に描いてスクロールさせる
    pub(super) fn scrolled(&mut self, f: fn(&mut Game)) {
        let real_h = self.gfx.height;
        if !self.gfx.compact || real_h >= VIRTUAL_H {
            f(self);
            self.close_button();
            return;
        }
        let max = (VIRTUAL_H - real_h) as f32;
        self.ov_max = max;
        self.ov_scroll = self.ov_scroll.clamp(0.0, max);
        let sc = self.ov_scroll;
        let b0 = self.buttons.len();
        let mouse = self.mouse;
        // 仮の画面の座標で描く（マウスも仮の座標に直す）
        self.gfx.height = VIRTUAL_H;
        self.mouse.1 += sc;
        self.gfx.set_2d_offset(-sc);
        f(self);
        self.gfx.set_2d_offset(0.0);
        self.gfx.height = real_h;
        self.mouse = mouse;
        // ボタンの当たり判定を画面の座標に戻す
        for b in self.buttons[b0..].iter_mut() {
            b.y0 -= sc;
            b.y1 -= sc;
        }
        // スクロールの目印（右端の細い帯）
        let (w, h) = (self.gfx.width as f32, real_h as f32);
        let bar_h = (h * h / VIRTUAL_H as f32).max(30.0);
        let y = (h - bar_h) * (sc / max);
        self.gfx.fill_round(w - 7.0, 2.0, w - 2.0, h - 2.0, 2.5, [0.0, 0.0, 0.0, 0.35]);
        self.gfx.fill_round(w - 7.0, y + 2.0, w - 2.0, y + bar_h - 2.0, 2.5, [1.0, 0.86, 0.5, 0.85]);
        self.close_button();
    }

    /// タッチ向けの画面：一覧をいつでも閉じられる ✕（右上に固定。スクロールしても動かない）
    fn close_button(&mut self) {
        if !self.gfx.compact {
            return;
        }
        let w = self.gfx.width as f32;
        let (x0, y0, x1, y1) = (w - 58.0, 3.0, w - 10.0, 51.0);
        self.gfx.fill_round(x0 - 2.0, y0 - 2.0, x1 + 2.0, y1 + 2.0, 6.0, [0.0, 0.0, 0.0, 0.55]);
        self.add_button(x0, y0, x1, y1, "", "", 0, true, Act::Cancel, t!("閉じる", "Close", "关闭"));
        self.gfx.stroke_round(x0, y0, x1, y1, 4.0, 2.0, [1.0, 0.85, 0.4, 1.0]);
        self.gfx.text_shadow("✕", x0, y0 + 12.0, x1 - x0, 26.0, Font::BigCenter, [1.0, 0.92, 0.7, 1.0]);
    }

    /// タッチした場所に何があるか（実ピクセル）
    /// 0=地図 1=画面部品 2=ミニマップ 3=なぞってスクロールできる一覧 4=タイトル画面の地図
    pub fn hit_kind(&self, px: f32, py: f32) -> u32 {
        let (x, y) = (px / self.gfx.ui, py / self.gfx.ui);
        let w = self.gfx.width as f32;
        if self.screen == Screen::Title {
            if self.show_save {
                return if self.ov_max > 0.0 { 3 } else { 1 };
            }
            return if x > w - 380.0 { 1 } else { 4 };
        }
        if self.overlay_open() {
            return if self.ov_max > 0.0 { 3 } else { 1 };
        }
        if self.world.game_over.is_some() || self.peace_pending() {
            return 1;
        }
        let (mx0, my0, mx1, my1) = self.mm_rect();
        if x >= mx0 && x <= mx1 && y >= my0 && y <= my1 {
            return 2;
        }
        if self.over_ui(x, y) || self.button_at(x, y).is_some() {
            return 1;
        }
        0
    }

    /// 詰めた画面の上部バー（資源・人数・銭・年・速度・各画面）
    pub(super) fn render_topbar_compact(&mut self) {
        let w = self.gfx.width as f32;
        const H: f32 = TOP_H_COMPACT;
        const MID: f32 = H / 2.0;
        let pl = self.world.player;
        self.art.draw_stone_panel(&self.gfx, 0.0, 0.0, w, H);
        let n = &self.world.nations[pl];
        let (res, inc, upkeep) = (n.res, n.income, n.upkeep);
        let (n_workers, w_cap, n_soldiers, m_cap) = (n.workers, n.worker_cap(), n.soldiers, n.mil_cap());
        let (coin, coin_inc) = (n.coin, n.coin_income);
        let res_icons = [ResIcon::Food, ResIcon::Wood, ResIcon::Bronze];
        let mut x = 6.0;
        for k in 0..3 {
            if let Some(ic) = &self.art.res_icons[res_icons[k] as usize] {
                self.gfx.draw_bitmap(ic, x, MID - 13.0, x + 26.0, MID + 13.0);
            }
            // 増減は文字色で示す（減っていれば赤）
            let rate = if k == 0 { inc[k] - upkeep } else { inc[k] };
            let col = if rate < -0.05 { [1.0, 0.45, 0.35, 1.0] } else { RES_COLORS[k] };
            self.gfx.text_shadow(&format!("{}", res[k] as i32), x + 28.0, MID - 11.0, 54.0, 22.0, Font::Big, col);
            let tip = match k {
                0 => tf!("糧 {:+.1}/秒（軍の維持費 -{:.1}/秒を含む）", "Food {:+.1}/s (incl. army upkeep -{:.1}/s)", "粮 {:+.1}/秒（含军队维持费 -{:.1}/秒）", rate, upkeep),
                1 => tf!("木 {:+.1}/秒", "Wood {:+.1}/s", "木 {:+.1}/秒", rate),
                _ => tf!("銅 {:+.1}/秒", "Bronze {:+.1}/s", "铜 {:+.1}/秒", rate),
            };
            self.buttons.push(Button { x0: x, y0: 0.0, x1: x + 80.0, y1: H, label: String::new(), key: 0, enabled: true, act: Act::Noop, tip });
            x += 82.0;
        }
        // 民・兵（押すと全体を選ぶ）
        for (icon, s, act) in [
            (ResIcon::Worker, format!("{}/{}", n_workers, w_cap), Act::SelectAllWorkers),
            (ResIcon::Soldier, format!("{}/{}", n_soldiers, m_cap), Act::SelectAllMilitary),
        ] {
            self.gfx.fill_round(x, 4.0, x + 76.0, H - 4.0, 4.0, [0.16, 0.13, 0.09, 0.75]);
            self.gfx.stroke_round(x, 4.0, x + 76.0, H - 4.0, 4.0, 1.0, [0.75, 0.62, 0.34, 0.85]);
            if let Some(ic) = &self.art.res_icons[icon as usize] {
                self.gfx.draw_bitmap(ic, x + 2.0, MID - 12.0, x + 26.0, MID + 12.0);
            }
            self.gfx.text_shadow(&s, x + 27.0, MID - 10.0, 48.0, 22.0, Font::Normal, [0.95, 0.92, 0.85, 1.0]);
            let tip = if act == Act::SelectAllWorkers { t!("民全体を選ぶ", "Select all workers", "选择全部民") } else { t!("全軍を選ぶ", "Select all soldiers", "选择全军") };
            self.buttons.push(Button { x0: x, y0: 0.0, x1: x + 76.0, y1: H, label: String::new(), key: 0, enabled: true, act, tip: tip.to_string() });
            x += 80.0;
        }
        // 銭（押すと交易画面）
        self.gfx.fill_circle(x + 11.0, MID, 10.0, [0.86, 0.70, 0.30, 1.0]);
        self.gfx.stroke_circle(x + 11.0, MID, 10.0, 1.2, [0.45, 0.32, 0.12, 1.0]);
        self.gfx.fill_rect(x + 7.5, MID - 3.5, x + 14.5, MID + 3.5, [0.30, 0.22, 0.10, 1.0]);
        self.gfx.text_shadow(&format!("{}", coin as i32), x + 24.0, MID - 11.0, 52.0, 22.0, Font::Big, [1.0, 0.86, 0.45, 1.0]);
        let tip = tf!("銭 {:+.1}/秒（押すと交易画面）", "Coin {:+.1}/s (tap for Trade)", "钱 {:+.1}/秒（点击打开贸易）", coin_inc);
        self.buttons.push(Button { x0: x, y0: 0.0, x1: x + 76.0, y1: H, label: String::new(), key: 0, enabled: true, act: Act::Market, tip });
        x += 80.0;

        // 右側：一時停止・速度・各画面
        let bw = 60.0;
        let menu = [
            (t!("交易", "Trade", "贸易"), Act::Market),
            (t!("外交", "Diplo", "外交"), Act::Diplo),
            (t!("記録", "Save", "存档"), Act::SaveMenu),
            ("？", Act::Help),
        ];
        let mut rx = w - 4.0 - menu.len() as f32 * (bw + 3.0);
        let speed_x = rx - 92.0;
        for (k, (lbl, act)) in menu.iter().enumerate() {
            let x0 = rx + k as f32 * (bw + 3.0);
            self.add_button(x0, 3.0, x0 + bw, H - 3.0, lbl, "", 0, true, *act, "");
        }
        rx = speed_x;
        let paused = self.paused;
        self.add_button(rx, 3.0, rx + 40.0, H - 3.0, if paused { "▶" } else { "❚❚" }, "", 0, true, Act::Speed(0), "");
        // 速度は押すたびに ×1 → ×2 → ×4 → ×8
        let next = match self.speed {
            1 => 2,
            2 => 4,
            4 => 8,
            _ => 1,
        };
        let lbl = format!("×{}", self.speed);
        self.add_button(rx + 43.0, 3.0, rx + 88.0, H - 3.0, &lbl, "", 0, true, Act::Speed(next), t!("押すたびに速度が変わる（×1→×2→×4→×8）", "Tap to change speed (×1→×2→×4→×8)", "每次点击切换速度（×1→×2→×4→×8）"));
        // 年（収まる書き方を選ぶ）
        let yb = self.world.year_bc();
        let spring = self.world.era == Era::Spring;
        let room = rx - x - 8.0;
        let cands = [
            (tf!("前{}年 {}", "{} BC {}", "前{}年 {}", yb, if spring { t!("春秋", "S&A", "春秋") } else { t!("戦国", "WS", "战国") }), Font::Big),
            (tf!("前{}年", "{} BC", "前{}年", yb), Font::Big),
            (tf!("前{}年", "{} BC", "前{}年", yb), Font::Normal),
        ];
        let (s, f) = cands.into_iter().find(|(s, f)| self.gfx.measure(s, *f) <= room).unwrap_or((tf!("前{}", "{}BC", "前{}", yb), Font::Small));
        self.gfx.text_shadow(&s, x, if f == Font::Big { MID - 11.0 } else { MID - 9.0 }, room.max(40.0), 24.0, f, [1.0, 0.9, 0.6, 1.0]);
    }
}

impl Game {
    /// 詰めた画面のタイトル下部（2段のボタン）
    pub(super) fn render_title_buttons_compact(&mut self, px: f32) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let (x0, x1) = (px + 20.0, w - 20.0);
        let gap = 4.0;
        // 1段目：続きから・英霊モード・チュートリアル
        let ya = h - 94.0;
        let cw = (x1 - x0 - gap * 2.0) / 3.0;
        let hl = if self.heroic_opt { t!("英霊：オン", "Spirits: On", "英灵：开") } else { t!("英霊：オフ", "Spirits: Off", "英灵：关") };
        let row = [
            (t!("続きから", "Continue", "继续"), Act::SaveMenu, t!("記録した対局を再開する。書き出したファイルからも読み込める", "Resume a saved game. Exported files can be loaded too", "继续已保存的对局。也可读取导出的文件")),
            (hl, Act::ToggleHeroic, t!("英霊モード：民を生贄にして三皇五帝から春秋戦国の英霊を呼べる（一国に一体）", "Heroic Spirits mode: sacrifice workers to summon spirits of the past (one per state)", "英灵模式：以民为祭品召唤自三皇五帝至春秋战国的英灵（每国一位）")),
            (t!("チュートリアル", "Tutorial", "教程"), Act::Tutorial, t!("晋の君主として、操作と遊び方を1段ずつ学ぶ", "Learn the controls and the game step by step as ruler of Jin", "以晋国君主身份逐步学习操作与玩法")),
        ];
        for (k, (lbl, act, tip)) in row.into_iter().enumerate() {
            let bx = x0 + k as f32 * (cw + gap);
            self.add_button(bx, ya, bx + cw, ya + 42.0, lbl, "", 0, true, act, tip);
            if act == Act::ToggleHeroic && self.heroic_opt {
                self.gfx.stroke_round(bx, ya, bx + cw, ya + 42.0, 3.0, 2.0, [1.0, 0.85, 0.4, 1.0]);
            }
        }
        // 2段目：文字の大きさ・言語・全画面
        let yb = h - 48.0;
        self.add_button(x0, yb, x0 + 50.0, yb + 42.0, "A−", "", 0, true, Act::Scale(-1), t!("文字と画面部品を小さくする", "Make text and interface smaller", "缩小文字与界面"));
        self.add_button(x0 + 54.0, yb, x0 + 104.0, yb + 42.0, "A＋", "", 0, true, Act::Scale(1), t!("文字と画面部品を大きくする", "Make text and interface larger", "放大文字与界面"));
        let ll = format!("{} ▸", crate::lang::lang().native());
        self.add_button(x0 + 108.0, yb, x0 + 214.0, yb + 42.0, &ll, "", 0, true, Act::Lang, "表示言語を切り替える / Change language / 切换语言");
        let fl = if self.gfx.is_fullscreen() { t!("ウィンドウ", "Window", "窗口") } else { t!("全画面", "Fullscreen", "全屏") };
        self.add_button(x0 + 218.0, yb, x1, yb + 42.0, fl, "", 0, true, Act::Fullscreen, t!("全画面表示を切り替える（横向きに固定）", "Toggle fullscreen (locks landscape)", "切换全屏显示（锁定横屏）"));
        // ツールチップ（タッチした直後だけ）
        let (mx, my) = self.mouse;
        let tip = self.buttons.iter().find(|b| mx >= b.x0 && mx <= b.x1 && my >= b.y0 && my <= b.y1 && !b.tip.is_empty()).map(|b| b.tip.clone());
        if let Some(t) = tip.filter(|_| !self.show_save && !(self.touch && self.clock > self.tip_until)) {
            let th = self.gfx.measure_h(&t, Font::Small, x1 - x0 - 16.0) + 10.0;
            let y1 = ya - 6.0;
            self.art.draw_inset_box(&self.gfx, x0, y1 - th - 4.0, x1, y1);
            self.gfx.text(&t, x0 + 8.0, y1 - th, x1 - x0 - 16.0, th, Font::Small, [0.95, 0.92, 0.85, 1.0]);
        }
    }
}

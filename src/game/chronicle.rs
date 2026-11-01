//! 史記：史実イベントの記録と、各国の君主家・在世の人物

use super::*;

impl Game {
    pub(super) fn render_chron(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pl = self.world.player;
        let pw = 1080.0f32.min(w - 20.0);
        let ph = 660.0f32.min(h - 40.0);
        let x0 = (w - pw) / 2.0;
        let y0 = (h - ph) / 2.0;
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.45]);
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + ph);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + ph - 3.0, 16.0);
        self.gfx.text_shadow(t!("史記", "Annals", "史记"), x0 + 24.0, y0 + 10.0, 120.0, 34.0, Font::Big, [1.0, 0.85, 0.5, 1.0]);
        self.gfx.text(t!("英雄・諸子百家・政治指導者の登場と、反乱・簒奪による君主家の交代（史実の年に起こる。滅んだ国の出来事は起こらない）", "The rise of heroes, philosophers and statesmen, and changes of ruling house by revolt and usurpation (they happen in their historical year; not for fallen states)", "英雄、诸子百家、政治家的登场，以及叛乱、篡位造成的君主家更替（于史实年份发生，已灭亡国家的事件不会发生）"), x0 + 100.0, y0 + 16.0, pw - 220.0, 24.0, Font::Small, [0.85, 0.82, 0.75, 1.0]);
        self.add_button(x0 + pw - 90.0, y0 + 10.0, x0 + pw - 14.0, y0 + 40.0, t!("閉じる", "Close", "关闭"), "", 0, true, Act::CloseOverlay, "");

        // ---- 左：自国と諸侯の人物
        let lw = 360.0;
        let (lx0, ly0, lx1, ly1) = (x0 + 12.0, y0 + 48.0, x0 + 12.0 + lw, y0 + ph - 12.0);
        self.art.draw_inset_box(&self.gfx, lx0, ly0, lx1, ly1);
        let tc = [0.95, 0.92, 0.85, 1.0];
        let dim = [0.72, 0.69, 0.62, 1.0];
        let gold = [1.0, 0.85, 0.5, 1.0];
        let mut y = ly0 + 8.0;
        let n = &self.world.nations[pl];
        let s = tf!("{} {}　君主家：{}", "{} {}   House: {}", "{} {}　君主家：{}", if n.alive { t!("自国", "Our state", "我国") } else { t!("旧国", "Former state", "故国") }, crate::lang::tx(n.name), crate::lang::tx(n.house));
        self.gfx.text(&s, lx0 + 12.0, y, lw - 24.0, 24.0, Font::Normal, gold);
        y += 26.0;
        let s = tf!("生産 ×{:.2}　　武 ×{:.2}", "Production ×{:.2}    Military ×{:.2}", "生产 ×{:.2}　　武 ×{:.2}", n.eco_mul, n.mil_mul);
        self.gfx.text(&s, lx0 + 12.0, y, lw - 24.0, 20.0, Font::Small, tc);
        y += 22.0;
        // 国の特徴
        let yb = self.world.year_bc();
        for t in all_traits_of(pl) {
            let on = traits_of(pl, yb).any(|x| std::ptr::eq(x, t));
            let s = format!("【{}】{}{}", crate::lang::tx(t.name), crate::lang::tx(t.desc), if on { "" } else { t!("（この年代には無効）", " (not active in this period)", "（本时期无效）") });
            let th = self.gfx.measure_h(&s, Font::Small, lw - 32.0);
            self.gfx.text(&s, lx0 + 20.0, y, lw - 32.0, th + 2.0, Font::Small, if on { [0.70, 0.92, 1.0, 1.0] } else { dim });
            y += th + 4.0;
        }
        let years = self.world.years;
        let lines = Self::figure_lines(&self.world.nations[pl], years);
        if lines.is_empty() {
            self.gfx.text(t!("仕える人物はいない", "No notable figures serve us", "没有效力的人物"), lx0 + 20.0, y, lw - 32.0, 20.0, Font::Small, dim);
            y += 20.0;
        }
        for (s, bad) in lines {
            let fh = self.gfx.measure_h(&s, Font::Small, lw - 32.0).max(18.0);
            self.gfx.text(&s, lx0 + 20.0, y, lw - 32.0, fh + 2.0, Font::Small, if bad { [1.0, 0.6, 0.5, 1.0] } else { tc });
            y += fh + 2.0;
        }
        y += 10.0;
        self.gfx.fill_rect(lx0 + 10.0, y, lx1 - 10.0, y + 1.0, [0.75, 0.62, 0.34, 0.6]);
        y += 8.0;
        self.gfx.text(t!("諸侯の君主家と人物", "Houses and figures of the lords", "诸侯的君主家与人物"), lx0 + 12.0, y, lw - 24.0, 22.0, Font::Normal, gold);
        y += 26.0;
        // 人物がいるか、君主家が変わった国を強い順に
        let mut list: Vec<usize> = (0..NATIONS.len())
            .filter(|&i| {
                let n = &self.world.nations[i];
                i != pl && n.alive && (n.figures.iter().any(|f| f.until > years) || n.house != INITIAL_HOUSE[i] || NATIONS[i].seven)
            })
            .collect();
        list.sort_by(|&a, &b| self.world.nations[b].strength.partial_cmp(&self.world.nations[a].strength).unwrap());
        for o in list {
            if y > ly1 - 40.0 {
                break;
            }
            let n = &self.world.nations[o];
            let figs: Vec<&str> = n.figures.iter().filter(|f| f.until > years).map(|f| crate::lang::tx(f.name)).collect();
            let s = format!("{}　{}", crate::lang::tx(n.name), crate::lang::tx(n.house));
            let c = n.color;
            self.gfx.fill_rect(lx0 + 12.0, y + 4.0, lx0 + 24.0, y + 16.0, c4(c, 1.0));
            self.gfx.text(&s, lx0 + 30.0, y, lw - 42.0, 20.0, Font::Small, tc);
            y += 19.0;
            if !figs.is_empty() {
                let s = figs.join(sep());
                let fh = self.gfx.measure_h(&s, Font::Small, lw - 54.0).min(40.0);
                self.gfx.text(&s, lx0 + 42.0, y, lw - 54.0, fh + 2.0, Font::Small, dim);
                y += fh + 2.0;
            }
            y += 3.0;
        }

        // ---- 右：年表（新しい順）
        let (rx0, ry0, rx1, ry1) = (lx1 + 10.0, ly0, x0 + pw - 12.0, ly1);
        self.art.draw_inset_box(&self.gfx, rx0, ry0, rx1, ry1);
        let tw = rx1 - rx0 - 28.0;
        let total = self.world.chronicle.len();
        if total == 0 {
            self.gfx.text(t!("まだ記すべき出来事はない。最初の出来事は紀元前743年。", "Nothing to record yet. The first event is in 743 BC.", "尚无可记之事。第一件事发生在公元前743年。"), rx0 + 14.0, ry0 + 10.0, tw, 24.0, Font::Normal, dim);
            return;
        }
        self.chron_scroll = self.chron_scroll.min(total - 1);
        let mut y = ry0 + 8.0;
        let mut shown = 0;
        for k in (0..total - self.chron_scroll).rev() {
            let e = &self.world.chronicle[k];
            let name = crate::lang::tx(NATIONS[e.nation].name);
            let head = if e.skipped {
                tf!("前{}年　{}　{}（{}は既に無く、起こらなかった）", "{} BC  {}  {} (did not happen: {} had already fallen)", "前{}年　{}　{}（{}已不存在，未发生）", e.bc, name, crate::lang::tx(e.title), name)
            } else {
                tf!("前{}年　{}【{}】{}", "{} BC  {} [{}] {}", "前{}年　{}【{}】{}", e.bc, name, e.kind.label(), crate::lang::tx(e.title))
            };
            let body = crate::lang::tx(e.text);
            let bh = if e.skipped { 0.0 } else { self.gfx.measure_h(body, Font::Small, tw - 16.0) };
            if y + 22.0 + bh > ry1 - 44.0 {
                break;
            }
            let kc = e.kind.color();
            let hc = if e.skipped {
                [0.55, 0.53, 0.5, 1.0]
            } else if e.nation == pl {
                gold
            } else {
                [kc[0], kc[1], kc[2], 1.0]
            };
            self.gfx.text(&head, rx0 + 14.0, y, tw, 22.0, Font::Normal, hc);
            y += 22.0;
            if !e.skipped {
                self.gfx.text(body, rx0 + 30.0, y, tw - 16.0, bh + 4.0, Font::Small, tc);
                y += bh + 6.0;
            }
            shown += 1;
        }
        // ページ送り
        let by = ry1 - 38.0;
        let older = self.chron_scroll + shown < total;
        let s = tf!("{}件中 {}〜{}件目（新しい順）", "{1}–{2} of {0} (newest first)", "共{}件　第{}〜{}件（由新到旧）", total, self.chron_scroll + 1, self.chron_scroll + shown);
        self.gfx.text(&s, rx0 + 14.0, by + 6.0, 300.0, 22.0, Font::Small, dim);
        self.add_button(rx1 - 250.0, by, rx1 - 132.0, by + 30.0, t!("▲ 新しい", "▲ Newer", "▲ 较新"), "", 0, self.chron_scroll > 0, Act::ChronPage(-(shown.max(1) as i32)), "");
        self.add_button(rx1 - 126.0, by, rx1 - 8.0, by + 30.0, t!("▼ 古い", "▼ Older", "▼ 较旧"), "", 0, older, Act::ChronPage(shown.max(1) as i32), "");
    }

    /// 在世の人物を「名前 効果 残り年」の行にする（不利な人物かどうか）
    fn figure_lines(n: &Nation, years: f32) -> Vec<(String, bool)> {
        n.figures
            .iter()
            .filter(|f| f.until > years)
            .map(|f| {
                let mut s = String::from(crate::lang::tx(f.name));
                if f.eco != 0.0 {
                    s.push_str(&tf!("　生産{:+.0}%", "  production {:+.0}%", "　生产{:+.0}%", f.eco * 100.0));
                }
                if f.mil != 0.0 {
                    s.push_str(&tf!("　武{:+.0}%", "  military {:+.0}%", "　武{:+.0}%", f.mil * 100.0));
                }
                s.push_str(&tf!("　残り{}年", "  {} yrs left", "　剩余{}年", (f.until - years).ceil() as i32));
                (s, f.eco < 0.0 || f.mil < 0.0)
            })
            .collect()
    }
}

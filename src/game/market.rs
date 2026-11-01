//! 交易画面（税率・市場・西域の道・山岳民と遊牧民・傭兵）と隊商の描画

use super::*;

/// 隊商の色（ミニマップ・目印）
pub fn caravan_color(k: CaravanKind) -> [f32; 4] {
    match k {
        CaravanKind::Merchant => [1.0, 0.85, 0.35, 1.0],
        CaravanKind::Western(_) => [0.80, 0.62, 1.0, 1.0],
        CaravanKind::Tribe(_) => [0.85, 0.55, 0.30, 1.0],
    }
}

/// 地図上の隊商：中原の商人は牛車、西域の商人は駱駝、部族は馬
pub fn draw_caravan(g: &mut Gfx, c: &Caravan, px: f32) {
    let s = (0.9f32).max(8.0 * px);
    let (x, y) = (c.pos.x, c.pos.y);
    let f = if c.dir.x < -0.05 { -1.0 } else { 1.0 };
    let dark = [0.12, 0.09, 0.06, 0.95];
    g.circle(x, y + s * 0.42, s * 0.62, 10, [0.0, 0.0, 0.0, 0.25]);
    match c.kind {
        CaravanKind::Merchant => {
            // 牛
            g.circle(x + f * s * 0.55, y, s * 0.24, 8, [0.45, 0.33, 0.22, 1.0]);
            g.circle(x + f * s * 0.80, y - s * 0.08, s * 0.13, 6, [0.40, 0.29, 0.19, 1.0]);
            // 車
            g.rect(x - s * 0.55, y - s * 0.35, x + s * 0.25, y + s * 0.12, [0.62, 0.46, 0.26, 1.0]);
            g.rect(x - s * 0.50, y - s * 0.55, x + s * 0.20, y - s * 0.30, [0.88, 0.80, 0.55, 1.0]);
            g.ring(x - s * 0.15, y + s * 0.22, s * 0.18, s * 0.07, 8, dark);
            g.circle(x - s * 0.15, y - s * 0.75, s * 0.12, 8, [1.0, 0.85, 0.3, 1.0]);
        }
        CaravanKind::Western(_) => {
            // 駱駝：胴・二つの瘤・首・頭
            let tan = [0.78, 0.62, 0.38, 1.0];
            g.line((x - f * s * 0.35, y + s * 0.05), (x - f * s * 0.35, y + s * 0.45), s * 0.09, tan);
            g.line((x + f * s * 0.30, y + s * 0.05), (x + f * s * 0.30, y + s * 0.45), s * 0.09, tan);
            g.circle(x, y, s * 0.36, 10, tan);
            g.circle(x - f * s * 0.15, y - s * 0.30, s * 0.17, 8, tan);
            g.circle(x + f * s * 0.17, y - s * 0.28, s * 0.15, 8, tan);
            g.line((x + f * s * 0.35, y - s * 0.05), (x + f * s * 0.62, y - s * 0.50), s * 0.12, tan);
            g.circle(x + f * s * 0.68, y - s * 0.55, s * 0.12, 6, tan);
            // 荷（絹と玉）
            g.rect(x - s * 0.22, y - s * 0.12, x + s * 0.22, y + s * 0.14, [0.55, 0.30, 0.75, 1.0]);
            g.rect(x - s * 0.22, y - s * 0.02, x + s * 0.22, y + s * 0.03, [0.95, 0.85, 0.4, 1.0]);
        }
        CaravanKind::Tribe(_) => {
            // 馬と騎手
            let br = [0.42, 0.28, 0.18, 1.0];
            g.line((x - f * s * 0.30, y + s * 0.05), (x - f * s * 0.38, y + s * 0.45), s * 0.08, br);
            g.line((x + f * s * 0.28, y + s * 0.05), (x + f * s * 0.36, y + s * 0.45), s * 0.08, br);
            g.circle(x, y, s * 0.30, 10, br);
            g.circle(x - f * s * 0.20, y + s * 0.02, s * 0.25, 8, br);
            g.line((x + f * s * 0.25, y - s * 0.05), (x + f * s * 0.55, y - s * 0.35), s * 0.13, br);
            g.circle(x - f * s * 0.05, y - s * 0.38, s * 0.15, 8, [0.75, 0.55, 0.35, 1.0]);
            g.rect(x - s * 0.45, y - s * 0.15, x - s * 0.15, y + s * 0.10, [0.85, 0.75, 0.55, 1.0]);
        }
    }
    // 種類が遠目にも分かる小旗
    let col = caravan_color(c.kind);
    g.line((x, y - s * 0.4), (x, y - s * 1.2), s * 0.06, dark);
    g.tri((x, y - s * 1.2), (x + s * 0.45, y - s * 1.04), (x, y - s * 0.88), col);
}

impl Game {
    pub(super) fn render_market(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pl = self.world.player;
        let pw = 1080.0f32.min(w - 20.0);
        let ph = 660.0f32.min(h - 40.0);
        let x0 = (w - pw) / 2.0;
        let y0 = (h - ph) / 2.0;
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.45]);
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + ph);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + ph - 3.0, 16.0);
        self.gfx.text_shadow(t!("交易", "Trade", "贸易"), x0 + 24.0, y0 + 10.0, 120.0, 34.0, Font::Big, [1.0, 0.85, 0.5, 1.0]);
        self.gfx.text(t!("商人は生産力の高い国に集まり、領内を通るだけで税（銭）を納める。銭で資源を買い、山岳民・遊牧民を傭兵に雇える", "Merchants gather in productive states and pay tax (coin) just by passing through. Use coin to buy resources and hire hill and steppe peoples as mercenaries", "商人聚集于生产力高的国家，只要途经领内就会缴税（钱）。可用钱购买资源、雇佣山地与游牧民为佣兵"), x0 + 100.0, y0 + 16.0, pw - 220.0, 24.0, Font::Small, [0.85, 0.82, 0.75, 1.0]);
        self.add_button(x0 + pw - 90.0, y0 + 10.0, x0 + pw - 14.0, y0 + 40.0, t!("閉じる", "Close", "关闭"), "", 0, true, Act::CloseOverlay, "");

        let tc = [0.95, 0.92, 0.85, 1.0];
        let dim = [0.72, 0.69, 0.62, 1.0];
        let gold = [1.0, 0.85, 0.5, 1.0];

        // ---- 左：自国の銭・税率・市場
        let lw = 420.0;
        let (lx0, ly0, lx1, ly1) = (x0 + 12.0, y0 + 48.0, x0 + 12.0 + lw, y0 + ph - 12.0);
        self.art.draw_inset_box(&self.gfx, lx0, ly0, lx1, ly1);
        let mut y = ly0 + 10.0;
        let n = &self.world.nations[pl];
        let (coin, inc, tax) = (n.coin, n.coin_income, n.tax as usize);
        let s = tf!("{}の銭　{:.0}　（{:+.1}/秒）", "Coin of {}  {:.0}  ({:+.1}/s)", "{}的钱　{:.0}　（{:+.1}/秒）", crate::lang::tx(n.name), coin, inc);
        self.gfx.text(&s, lx0 + 12.0, y, lw - 24.0, 24.0, Font::Normal, gold);
        y += 30.0;

        // 領内の隊商
        let tr = &self.world.trade;
        let mut here = [0usize; 3];
        for c in tr.caravans.iter() {
            let (x, yy) = c.pos.tile();
            if Map::inb(x, yy) && self.world.owner[Map::idx(x, yy)] as usize == pl {
                let k = match c.kind {
                    CaravanKind::Merchant => 0,
                    CaravanKind::Western(_) => 1,
                    CaravanKind::Tribe(_) => 2,
                };
                here[k] += 1;
            }
        }
        let total_m = tr.count(|k| k == CaravanKind::Merchant);
        let total_w = tr.count(|k| matches!(k, CaravanKind::Western(_)));
        let s = tf!("領内の隊商：商人 {}　西域の商人 {}　部族 {}", "Caravans in our land: merchants {}, Western {}, tribal {}", "领内商队：商人 {}　西域商人 {}　部族 {}", here[0], here[1], here[2]);
        self.gfx.text(&s, lx0 + 12.0, y, lw - 24.0, 22.0, Font::Normal, tc);
        y += 24.0;
        let s = tf!("中原を巡る商人 {}人（中原全体の生産量に比例）　西域の商人 {}人", "{} merchants travel the Central Plains (grows with total production); {} Western merchants", "周游中原的商人 {}人（与中原总产量成正比）　西域商人 {}人", total_m, total_w);
        let mh = self.gfx.measure_h(&s, Font::Small, lw - 24.0).max(20.0);
        self.gfx.text(&s, lx0 + 12.0, y, lw - 24.0, mh, Font::Small, dim);
        y += mh + 10.0;

        self.gfx.text(t!("税率", "Tax rate", "税率"), lx0 + 12.0, y, 80.0, 22.0, Font::Normal, gold);
        y += 26.0;
        let tips = [
            t!("軽税：税は6割だが、商人が1.5倍集まりやすい", "Light: 60% of the tax, but merchants are 1.5× more likely to come", "轻税：税收为六成，但商人聚集概率为1.5倍"),
            t!("中税：標準", "Normal: standard", "中税：标准"),
            t!("重税：税は1.5倍だが、商人が寄りつきにくい（0.55倍）", "Heavy: 1.5× the tax, but merchants stay away (0.55×)", "重税：税收为1.5倍，但商人不愿前来（0.55倍）"),
        ];
        for k in 0..3 {
            let bx = lx0 + 14.0 + k as f32 * 130.0;
            self.add_button(bx, y, bx + 122.0, y + 30.0, tax_name(k), "", 0, true, Act::Tax(k as u8), tips[k]);
            if k == tax {
                self.gfx.stroke_round(bx, y, bx + 122.0, y + 30.0, 3.0, 2.0, [1.0, 0.9, 0.4, 1.0]);
            }
        }
        y += 36.0;
        self.gfx.text(tips[tax], lx0 + 14.0, y, lw - 28.0, 20.0, Font::Small, dim);
        y += 30.0;

        self.gfx.fill_rect(lx0 + 10.0, y, lx1 - 10.0, y + 1.0, [0.75, 0.62, 0.34, 0.6]);
        y += 10.0;
        self.gfx.text(t!("市場で買う", "Buy at the market", "在市场购买"), lx0 + 12.0, y, 200.0, 22.0, Font::Normal, gold);
        y += 26.0;
        for k in 0..3 {
            let bx = lx0 + 14.0 + k as f32 * 130.0;
            let lbl = format!("{} +{}", res_name(k), MARKET_LOT as i32);
            let price = self.world.market_price(pl, k);
            let sub = tf!("{}銭", "{} coin", "{}钱", price as i32);
            let tip = tf!("銭{}で{}を{}買う", "Buy {2} {1} for {0} coin", "花{}钱购买{}{}", price as i32, res_name(k), MARKET_LOT as i32);
            self.add_button(bx, y, bx + 122.0, y + 30.0, &lbl, "", 0, coin >= price, Act::Buy(k), &tip);
            self.gfx.text(&sub, bx, y + 32.0, 122.0, 18.0, Font::Small, dim);
        }
        y += 60.0;

        self.gfx.fill_rect(lx0 + 10.0, y, lx1 - 10.0, y + 1.0, [0.75, 0.62, 0.34, 0.6]);
        y += 10.0;
        self.gfx.text(t!("雇っている傭兵", "Hired mercenaries", "受雇的佣兵"), lx0 + 12.0, y, 200.0, 22.0, Font::Normal, gold);
        y += 26.0;
        let years = self.world.years;
        let mut mercs: Vec<(usize, i32)> = Vec::new();
        for u in self.world.units.iter() {
            if u.alive && u.nation as usize == pl && u.merc_until > 0.0 {
                let t = u.tribe as usize;
                let left = (u.merc_until - years).ceil() as i32;
                match mercs.iter_mut().find(|m| m.0 == t) {
                    Some(m) => m.1 = m.1.min(left),
                    None => mercs.push((t, left)),
                }
            }
        }
        if mercs.is_empty() {
            self.gfx.text(t!("いない（右の部族から雇える。雇った部族はその間、自国を襲わない）", "None (hire from the peoples on the right; while hired they will not raid you)", "无（可从右侧部族雇佣，受雇期间不会袭击我国）"), lx0 + 20.0, y, lw - 32.0, 40.0, Font::Small, dim);
        }
        for (t, left) in mercs {
            let cnt = self.world.units.iter().filter(|u| u.alive && u.nation as usize == pl && u.merc_until > 0.0 && u.tribe as usize == t).count();
            let s = tf!("{}の兵 {}　残り{}年", "{} warriors {}, {} years left", "{}兵 {}　剩余{}年", TRIBES.get(t).map_or("?", |d| crate::lang::tx(d.name)), cnt, left.max(0));
            self.gfx.text(&s, lx0 + 20.0, y, lw - 32.0, 20.0, Font::Small, tc);
            y += 20.0;
            if y > ly1 - 20.0 {
                break;
            }
        }

        // ---- 右：西域の道と部族
        let (rx0, ry0, rx1, ry1) = (lx1 + 10.0, ly0, x0 + pw - 12.0, ly1);
        self.art.draw_inset_box(&self.gfx, rx0, ry0, rx1, ry1);
        let rw = rx1 - rx0 - 24.0;
        let mut y = ry0 + 10.0;
        self.gfx.text(t!("西域への道（西域の商人は道の先の地域にだけ来る）", "Roads to the West (Western merchants only visit the lands at the end of a road)", "通往西域之路（西域商人只会来到道路尽头的地区）"), rx0 + 12.0, y, rw, 22.0, Font::Normal, gold);
        y += 26.0;
        let yb = self.world.year_bc();
        for (r, rt) in ROUTES.iter().enumerate() {
            let open = route_open(r, yb);
            let st = if open { t!("通じている", "open", "已开通").to_string() } else { tf!("前{}年ごろから", "from about {} BC", "约前{}年起", rt.from_bc) };
            let s = format!("{}　{}", crate::lang::tx(rt.name), st);
            self.gfx.text(&s, rx0 + 18.0, y, rw, 20.0, Font::Small, if open { [0.82, 0.70, 1.0, 1.0] } else { dim });
            y += 19.0;
            let dh = self.gfx.measure_h(crate::lang::tx(rt.desc), Font::Small, rw - 20.0).max(20.0);
            self.gfx.text(crate::lang::tx(rt.desc), rx0 + 34.0, y, rw - 20.0, dh, Font::Small, dim);
            y += dh + 2.0;
        }
        y += 6.0;
        self.gfx.fill_rect(rx0 + 10.0, y, rx1 - 10.0, y + 1.0, [0.75, 0.62, 0.34, 0.6]);
        y += 10.0;
        let cost = self.world.merc_cost();
        let s = tf!("山岳民と遊牧民（傭兵は{}人・{}年で{}銭）", "Hill and steppe peoples (mercenaries: {} for {} years, {} coin)", "山地与游牧民（佣兵{}名・{}年{}钱）", MERC_N, MERC_YEARS as i32, cost);
        self.gfx.text(&s, rx0 + 12.0, y, rw, 22.0, Font::Normal, gold);
        y += 28.0;
        let active: Vec<usize> = (0..TRIBES.len()).filter(|&t| tribe_active(t, yb)).collect();
        if active.is_empty() {
            self.gfx.text(t!("いまは活動している部族はいない", "No peoples are active now", "目前没有活动的部族"), rx0 + 18.0, y, rw, 20.0, Font::Small, dim);
        }
        for t in active {
            if y > ry1 - 40.0 {
                break;
            }
            let td = &TRIBES[t];
            let st = &self.world.trade.tribes[t];
            let raiding_me = self.world.trade.bands.iter().any(|b| b.tribe == t && b.target == pl && b.phase == 0);
            let hired = st.contract[pl] > years;
            let kind = if td.nomad { t!("遊牧", "steppe", "游牧") } else { t!("山岳", "hill", "山地") };
            let head = tf!("{}（{}）", "{} ({})", "{}（{}）", crate::lang::tx(td.name), kind);
            let hc = if raiding_me { [1.0, 0.45, 0.35, 1.0] } else if hired { [0.6, 0.95, 0.6, 1.0] } else { tc };
            self.gfx.text(&head, rx0 + 18.0, y, 230.0, 20.0, Font::Normal, hc);
            let state = if raiding_me {
                t!("自国へ来寇中", "raiding us", "正在侵犯我国").to_string()
            } else if hired {
                tf!("自国に雇われている（残り{}年）", "hired by us ({} years left)", "受雇于我国（剩余{}年）", (st.contract[pl] - years).ceil() as i32)
            } else if st.last.is_empty() {
                t!("静か", "quiet", "平静").to_string()
            } else {
                st.last.clone()
            };
            self.gfx.text(&state, rx0 + 250.0, y + 2.0, rw - 350.0, 20.0, Font::Small, dim);
            let ok = self.world.can_hire(pl, t);
            let tip = match ok {
                Ok(()) => tf!("{}\n{}の{}を{}人、{}年雇う", "{}\nHire {3} {1} {2} for {4} years", "{}\n雇佣{}{}{}名，为期{}年", crate::lang::tx(td.desc), crate::lang::tx(td.name), if td.nomad { t!("騎兵", "riders", "骑兵") } else { t!("兵", "warriors", "兵") }, MERC_N, MERC_YEARS as i32),
                Err(e) => tf!("{}\n雇えない：{}", "{}\nCannot hire: {}", "{}\n无法雇佣：{}", crate::lang::tx(td.desc), e),
            };
            self.add_button(rx1 - 96.0, y - 2.0, rx1 - 10.0, y + 22.0, t!("雇う", "Hire", "雇佣"), "", 0, ok.is_ok(), Act::Hire(t), &tip);
            y += 22.0;
            self.gfx.text(crate::lang::tx(td.desc), rx0 + 34.0, y, rw - 130.0, 20.0, Font::Small, [0.6, 0.58, 0.52, 1.0]);
            y += 24.0;
        }
    }
}

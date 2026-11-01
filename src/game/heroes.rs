//! 英霊召喚画面と、地図上の英霊の名札

use super::*;

impl Game {
    pub(super) fn render_summon(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pl = self.world.player;
        let pw = 1120.0f32.min(w - 20.0);
        let ph = 680.0f32.min(h - 30.0);
        let x0 = (w - pw) / 2.0;
        let y0 = (h - ph) / 2.0;
        self.gfx.fill_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.5]);
        self.art.draw_stone_panel(&self.gfx, x0, y0, x0 + pw, y0 + ph);
        self.art.draw_corner_ornaments(&self.gfx, x0 + 3.0, y0 + 3.0, x0 + pw - 3.0, y0 + ph - 3.0, 16.0);
        self.gfx.text_shadow(t!("英霊召喚", "Summon Spirit", "召唤英灵"), x0 + 24.0, y0 + 10.0, 160.0, 34.0, Font::Big, [1.0, 0.85, 0.5, 1.0]);
        self.add_button(x0 + pw - 90.0, y0 + 10.0, x0 + pw - 14.0, y0 + 40.0, t!("閉じる", "Close", "关闭"), "", 0, true, Act::CloseOverlay, "");

        let own = self.own_sel();
        let (victims, p) = self.world.sacrifice_power(pl, &own);
        let gold = [1.0, 0.85, 0.5, 1.0];
        let tc = [0.95, 0.92, 0.85, 1.0];
        let dim = [0.72, 0.69, 0.62, 1.0];
        let head = match self.world.hero_of(pl) {
            Some(i) => tf!("自国の英霊：{}（英霊は一国に一体だけ）", "Our spirit: {} (one spirit per state)", "我国英灵：{}（每国只能有一位英灵）", crate::lang::tx(HEROES[self.world.units[i].hero as usize].name)),
            None if victims.is_empty() => t!("自国領内の民を選んでから開くと、その民を生贄に捧げて英霊を呼べる", "Select workers in your territory first, then open this to sacrifice them and summon a spirit", "先选择己方领土内的民再打开，即可献祭他们召唤英灵").to_string(),
            None => tf!("生贄：民{}人（能力の合計 {:.1}）。英霊の能力＝生贄の合計×英霊の倍率（最大{}人）", "Sacrifice: {} workers (total power {:.1}). Spirit power = sacrifice total × the spirit's multiplier (up to {} workers)", "祭品：民{}名（能力总和 {:.1}）。英灵能力＝祭品总和×英灵倍率（最多{}名）", victims.len(), p, SACRIFICE_MAX),
        };
        self.gfx.text(&head, x0 + 190.0, y0 + 16.0, pw - 300.0, 24.0, Font::Normal, tc);

        self.art.draw_inset_box(&self.gfx, x0 + 10.0, y0 + 48.0, x0 + pw - 10.0, y0 + ph - 10.0);
        let cols = 2;
        let colw = (pw - 30.0) / cols as f32;
        let rowh = ((ph - 70.0) / ((HEROES.len() + 1) / 2) as f32).min(40.0);
        let era = self.world.era;
        for (k, d) in HEROES.iter().enumerate() {
            let c = k / ((HEROES.len() + 1) / 2);
            let r = k % ((HEROES.len() + 1) / 2);
            let rx = x0 + 15.0 + c as f32 * colw;
            let ry = y0 + 54.0 + r as f32 * rowh;
            if r % 2 == 0 {
                self.gfx.fill_rect(rx, ry, rx + colw - 8.0, ry + rowh, [1.0, 1.0, 1.0, 0.04]);
            }
            let owner = self.world.units.iter().find(|u| u.alive && u.hero as usize == k).map(|u| u.nation as usize);
            let isz = (rowh - 4.0).clamp(24.0, 36.0);
            let ix0 = rx + 4.0;
            let iy0 = ry + (rowh - isz) * 0.5;
            self.gfx.fill_round(ix0, iy0, ix0 + isz, iy0 + isz, 3.0, [0.10, 0.08, 0.05, 0.92]);
            if let Some(hbmp) = self.art.hero_sprite(k, false) {
                self.gfx.draw_bitmap(hbmp, ix0 + 1.0, iy0 + 1.0, ix0 + isz - 1.0, iy0 + isz - 1.0);
            }
            self.gfx.stroke_round(ix0, iy0, ix0 + isz, iy0 + isz, 3.0, 1.0, if owner.is_some() { [0.45, 0.40, 0.32, 0.8] } else { [0.92, 0.78, 0.36, 0.92] });

            let tx = rx + isz + 10.0;
            let ue = if d.kind == UnitKind::Chariot { Era::Warring } else { era };
            let ny = ry + (rowh - 36.0).max(0.0) * 0.5;
            self.gfx.text(crate::lang::tx(d.name), tx, ny, 190.0, 20.0, Font::Normal, if owner.is_some() { dim } else { gold });
            self.gfx.text(crate::lang::tx(d.era), tx, ny + 19.0, 190.0, 17.0, Font::Small, dim);
            let s = format!("{} ×{:.1}", d.kind.name(ue), d.mult);
            self.gfx.text(&s, tx + 192.0, ry + (rowh - 20.0) * 0.5, 124.0, 20.0, Font::Small, tc);
            let st = match owner {
                Some(o) => tf!("{}が召喚中", "with {}", "{}召唤中", crate::lang::tx(self.world.nations[o].name)),
                None if p > 0.0 => tf!("能力 {:.1}", "power {:.1}", "能力 {:.1}", p * d.mult),
                None => String::new(),
            };
            self.gfx.text(&st, tx + 318.0, ry + (rowh - 20.0) * 0.5, colw - (isz + 10.0) - 412.0, 20.0, Font::Small, if owner.is_some() { [1.0, 0.6, 0.5, 1.0] } else { [0.7, 0.92, 1.0, 1.0] });
            let ok = self.world.can_summon(pl, k, &own);
            let tip = match ok {
                Ok(()) => tf!("{}\n民{}人を生贄に捧げて召喚する（能力 {:.1}）", "{}\nSacrifice {} workers to summon (power {:.1})", "{}\n献祭{}名民召唤（能力 {:.1}）", crate::lang::tx(d.desc), victims.len(), p * d.mult),
                Err(e) => tf!("{}\n召喚できない：{}", "{}\nCannot summon: {}", "{}\n无法召唤：{}", crate::lang::tx(d.desc), e),
            };
            self.add_button(rx + colw - 92.0, ry + 3.0, rx + colw - 14.0, ry + rowh - 3.0, t!("召喚", "Summon", "召唤"), "", 0, ok.is_ok(), Act::SummonHero(k), &tip);
        }
    }

    /// 英霊の名札（地図上）
    pub(super) fn render_hero_labels(&mut self) {
        if !self.world.heroic {
            return;
        }
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pl = self.world.player;
        let z = self.zoom;
        let mut labels = Vec::new();
        for u in self.world.units.iter() {
            if !u.alive || (u.hero as usize) >= HEROES.len() || (u.nation as usize != pl && !self.seen(u.pos)) {
                continue;
            }
            let (sx, sy) = self.world_to_screen(u.pos);
            if sx < -80.0 || sy < -40.0 || sx > w + 80.0 || sy > h + 40.0 {
                continue;
            }
            labels.push((sx, sy, crate::lang::tx(HEROES[u.hero as usize].name), self.world.nations[u.nation as usize].color));
        }
        for (sx, sy, name, c) in labels {
            let ty = sy - (z * 1.3).clamp(14.0, 34.0) - 22.0;
            self.gfx.fill_round(sx - 40.0, ty, sx + 40.0, ty + 20.0, 4.0, [0.10, 0.07, 0.02, 0.85]);
            self.gfx.stroke_round(sx - 40.0, ty, sx + 40.0, ty + 20.0, 4.0, 1.4, [1.0, 0.82, 0.35, 0.95]);
            self.gfx.text(name, sx - 40.0, ty + 1.0, 80.0, 18.0, Font::NormalCenter, [1.0, 0.92, 0.6, 1.0]);
            let _ = c;
        }
    }
}

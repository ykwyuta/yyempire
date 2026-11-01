//! 上帝降臨：史実で中華が統一された年（前221年）までに天下が定まらなければ、
//! 上帝が秦に降り、抗えない軍で10年のうちに中華を統一する（強制終了）

use super::*;

const QIN: usize = 2;
/// 上帝の軍が天下を統一するまでの年数
pub const DESCENT_YEARS: f32 = 10.0;
/// 上帝の兵の能力（誰も傷つけられない）
const DIVINE_POWER: f32 = 12.0;
/// 同時にいる上帝の兵の上限
const DIVINE_MAX: usize = 90;

pub struct Descent {
    /// 降臨した年（経過年）
    pub start: f32,
    /// 滅んでいた秦が蘇ったか
    pub revived: bool,
    /// 降臨時に秦以外が持っていた城邑の数
    pub total: usize,
    pub taken: usize,
}

impl World {
    /// 残り年数（天下統一まで）
    pub fn descent_left(&self) -> Option<f32> {
        self.descent.as_ref().map(|d| (d.start + DESCENT_YEARS - self.years).max(0.0))
    }

    /// 上帝の兵か
    pub fn divine(&self, i: usize) -> bool {
        self.units[i].divine
    }

    /// 前221年：上帝が秦に降臨する
    pub(super) fn begin_descent(&mut self) {
        let mut revived = false;
        if !self.nations[QIN].alive {
            revived = true;
            self.revive_qin();
        }
        // 秦は諸国すべてと戦う（和睦はできない）
        self.coalitions.clear();
        for o in 0..NATIONS.len() {
            if o != QIN && self.nations[o].alive {
                self.declare_war(QIN, o, true);
            }
        }
        let total = self.blds.iter().filter(|b| b.alive && b.kind.city() && b.done() && b.nation as usize != QIN).count();
        self.descent = Some(Descent { start: self.years, revived, total, taken: 0 });
        // 都に上帝の軍が現れる
        if let Some(c) = self.nations[QIN].capital {
            self.spawn_divine(c, 8);
        }
        let head = if revived { "滅びた秦が蘇り、" } else { "" };
        let text = format!(
            "紀元前221年、天下はなお定まらない。{}上帝が秦に降臨した。上帝の軍には何者も抗えず、{}年のうちに中華は秦に統一される。",
            head, DESCENT_YEARS as i32
        );
        self.push_msg("上帝が秦に降臨した".to_string(), [1.0, 0.85, 0.3]);
        self.banner = None;
        self.banner_queue.clear();
        self.queue_banner(Banner { title: "上帝降臨".to_string(), text, t: 12.0 });
    }

    /// 滅んだ秦を咸陽に蘇らせる（そこにある城邑を奪うか、新たに都を築く）
    fn revive_qin(&mut self) {
        let home = {
            let (x, y) = ll_to_tile(108.7, 34.35);
            v2(x, y)
        };
        let n = &mut self.nations[QIN];
        n.alive = true;
        n.appeared = true;
        n.capital = None;
        n.res = [500.0, 300.0, 300.0];
        n.home = home;
        let mut best: Option<(f32, usize)> = None;
        for (i, b) in self.blds.iter().enumerate() {
            if b.alive && b.kind.city() && b.done() {
                let d = b.center().dist(home);
                if d < 60.0 && best.map_or(true, |x| d < x.0) {
                    best = Some((d, i));
                }
            }
        }
        match best {
            Some((_, bi)) => {
                // 奪った城邑はそのまま秦の都になる
                self.capture_city(bi, QIN);
            }
            None => {
                if let Some(c) = self.found_city(QIN, 108.7, 34.35, BKind::Capital) {
                    self.nations[QIN].capital = Some(c);
                    self.territory_dirty = true;
                }
            }
        }
        if let Some(c) = self.nations[QIN].capital.and_then(|h| self.bld(h)) {
            let p = c.center();
            self.nations[QIN].home = p;
        }
    }

    /// 城邑に上帝の兵を出す
    fn spawn_divine(&mut self, city: Handle, n: usize) {
        let have = self.units.iter().filter(|u| u.alive && u.divine).count();
        for k in 0..n.min(DIVINE_MAX.saturating_sub(have)) {
            let Some(sp) = self.spawn_point(city) else { break };
            let kind = if k % 2 == 0 { UnitKind::Chariot } else { UnitKind::Infantry };
            let h = self.spawn_unit(QIN, kind, sp, DIVINE_POWER);
            self.unit_mut(h).unwrap().divine = true;
        }
    }

    /// 毎年：予定どおりに城邑を秦のものにし、上帝の兵を次の城へ向かわせる
    pub(super) fn yearly_descent(&mut self) {
        let Some(d) = &self.descent else { return };
        let p = ((self.years - d.start) / DESCENT_YEARS).clamp(0.0, 1.0);
        let (total, taken) = (d.total, d.taken);
        let home = self.nations[QIN].home;
        // 秦の都から近い順
        let mut rest: Vec<(f32, usize)> = self
            .blds
            .iter()
            .enumerate()
            .filter(|(_, b)| b.alive && b.kind.city() && b.done() && b.nation as usize != QIN)
            .map(|(i, b)| (b.center().dist(home), i))
            .collect();
        rest.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let want = if p >= 1.0 { taken + rest.len() } else { ((total as f32 * p).ceil() as usize).max(taken) };
        // 英霊が秦を滅ぼしていれば、もう攻略は進まない
        let n = if self.nations[QIN].alive { (want - taken).min(rest.len()) } else { 0 };
        // 英霊の守る城邑は奪えない
        let mut done = 0;
        for &(_, bi) in rest.iter() {
            if done >= n {
                break;
            }
            if !self.blds[bi].alive || self.blds[bi].nation as usize == QIN || self.hero_guarded(bi) {
                continue;
            }
            self.capture_city(bi, QIN);
            let h = self.bh(bi);
            self.spawn_divine(h, 2);
            done += 1;
        }
        if let Some(d) = &mut self.descent {
            d.taken += done;
        }
        // 上帝の兵は次の城へ
        let next: Vec<V2> = rest.iter().skip(n).take(6).map(|&(_, bi)| self.blds[bi].center()).collect();
        if !next.is_empty() {
            let idle: Vec<Handle> = (0..self.units.len()).filter(|&i| self.units[i].alive && self.units[i].divine && !matches!(self.units[i].order, Order::Attack(_))).map(|i| self.uh(i)).collect();
            for (k, h) in idle.into_iter().enumerate() {
                let g = next[k % next.len()];
                self.order_move(&[h], g, true);
            }
        }
        // 英霊モード：10年後の版図が上帝の秦より広ければ勝利
        if p >= 1.0 && self.game_over.is_none() && self.heroic {
            self.recompute_territory();
            self.update_stats();
            let pl = self.player;
            let (mine, god) = (self.nations[pl].tiles, self.nations[QIN].tiles);
            let text = if pl != QIN && self.nations[pl].alive && mine > god {
                self.game_over = Some((true, format!("英霊とともに上帝の軍を退けた！ {} の版図 {} は上帝の秦 {} を上回った", self.nations[pl].name, mine, god)));
                return;
            } else if pl == QIN {
                "上帝の軍が天下を統一した。秦の覇業は上帝のものとなった……".to_string()
            } else {
                format!("{} の版図 {} は上帝の秦 {} に及ばなかった。天命は秦に……", self.nations[pl].name, mine, god)
            };
            self.game_over = Some((false, text));
            return;
        }
        // 天下統一：強制終了
        if p >= 1.0 && self.game_over.is_none() {
            let pl = self.player;
            let text = if pl == QIN {
                format!("上帝の軍が天下を統一した。秦の覇業は上帝のものとなった……")
            } else {
                format!("上帝の軍が天下を統一した。{} の覇業はここに終わる……", self.nations[pl].name)
            };
            self.game_over = Some((false, text));
        }
    }
}

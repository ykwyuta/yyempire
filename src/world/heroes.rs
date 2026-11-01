//! 英霊召喚（英霊モード）：三皇五帝から春秋戦国までの英霊を、民を生贄にして呼び出す。
//! 時代と国を超えて誰でも呼べるが、一国が同時に従えられる英霊は一体だけ。
//! 英霊は上帝の兵を傷つけられ、英霊の守る城邑は上帝の軍にも奪われない。

use super::*;

/// 一度に生贄にできる民の上限
pub const SACRIFICE_MAX: usize = 10;
/// 英霊が守る城邑の範囲（タイル）
pub const HERO_GUARD: f32 = 15.0;

pub struct HeroDef {
    pub name: &'static str,
    /// 時代・出自
    pub era: &'static str,
    pub kind: UnitKind,
    /// 生贄にした民の能力に対する倍率（1.1〜3.0）
    pub mult: f32,
    pub desc: &'static str,
}

macro_rules! hd {
    ($n:expr, $e:expr, $k:ident, $m:expr, $d:expr) => {
        HeroDef { name: $n, era: $e, kind: UnitKind::$k, mult: $m, desc: $d }
    };
}

pub const HEROES: &[HeroDef] = &[
    // 三皇
    hd!("伏羲", "三皇", Archer, 2.2, "八卦を画し、網を結んで狩りを教えた人の祖"),
    hd!("女媧", "三皇", Infantry, 2.4, "五色の石で天を補い、人を造った女神"),
    hd!("神農", "三皇", Infantry, 2.0, "百草を嘗めて農耕と医薬を教えた炎帝"),
    // 五帝
    hd!("黄帝", "五帝", Chariot, 3.0, "涿鹿の戦いで蚩尤を破った五帝の筆頭"),
    hd!("顓頊", "五帝", Infantry, 2.0, "黄帝の孫。天と地の通いを絶った"),
    hd!("帝嚳", "五帝", Archer, 1.9, "日月の運行を定めた聖王"),
    hd!("堯", "五帝", Infantry, 1.8, "徳をもって天下を治め、位を舜に譲った"),
    hd!("舜", "五帝", Infantry, 1.8, "孝の聖王。四凶を退けた"),
    // 夏・殷・周
    hd!("禹", "夏", Infantry, 2.6, "黄河の治水を成し遂げ、夏を開いた"),
    hd!("湯王", "殷", Chariot, 2.3, "鳴条の戦いで夏の桀を討ち、殷を建てた"),
    hd!("太公望", "周", Archer, 2.5, "牧野の戦いで殷を破った周の軍師"),
    hd!("周の武王", "周", Chariot, 2.4, "牧野で殷の紂王を討った周の王"),
    hd!("周公旦", "周", Infantry, 1.6, "礼楽を定め、三監の乱を鎮めた"),
    // 春秋
    hd!("管仲", "春秋・斉", Archer, 1.4, "桓公を覇者にした宰相"),
    hd!("晋の文公", "春秋・晋", Chariot, 1.9, "城濮の戦いで楚を破った覇者"),
    hd!("孫武", "春秋・呉", Archer, 2.6, "『孫子』の兵法家。柏挙で楚を破った"),
    hd!("伍子胥", "春秋・呉", Infantry, 2.1, "郢を陥れ、平王の墓を鞭打った"),
    hd!("越王勾践", "春秋・越", Infantry, 1.7, "臥薪嘗胆の末に呉を滅ぼした"),
    hd!("孔子", "春秋・魯", Archer, 1.1, "仁と礼を説いた聖人。六芸の射に通じた"),
    // 戦国
    hd!("呉起", "戦国・魏", Infantry, 2.5, "魏武卒を鍛え、七十六戦して負けなかった"),
    hd!("孫臏", "戦国・斉", Archer, 2.3, "桂陵・馬陵で龐涓を破った兵法家"),
    hd!("楽毅", "戦国・燕", Chariot, 2.2, "五国の兵で斉の七十余城を落とした"),
    hd!("田単", "戦国・斉", Chariot, 2.0, "火牛の計で即墨から斉を取り戻した"),
    hd!("廉頗", "戦国・趙", Infantry, 2.4, "趙の老将。長平で秦を防いだ"),
    hd!("趙奢", "戦国・趙", Archer, 2.0, "閼与で秦軍を大破した"),
    hd!("信陵君", "戦国・魏", Archer, 2.1, "割符を盗んで趙を救った魏の公子"),
    hd!("李牧", "戦国・趙", Chariot, 2.7, "匈奴を大破し、秦を二度退けた名将"),
    hd!("白起", "戦国・秦", Infantry, 2.9, "長平で趙軍四十万を破った秦の名将"),
    hd!("王翦", "戦国・秦", Chariot, 2.8, "趙・楚を滅ぼした秦の老将"),
    hd!("項燕", "戦国・楚", Infantry, 2.2, "楚の最後の名将。李信を破った"),
    hd!("荊軻", "戦国・燕", Infantry, 1.5, "易水を渡り秦王を刺そうとした刺客"),
];

impl World {
    /// その国が従えている英霊（ユニット番号）
    pub fn hero_of(&self, n: usize) -> Option<usize> {
        (0..self.units.len()).find(|&i| self.units[i].alive && self.units[i].nation as usize == n && (self.units[i].hero as usize) < HEROES.len())
    }

    /// 英霊がいま地上にいるか（同じ英霊は二国に同時には現れない）
    pub fn hero_on_field(&self, h: usize) -> bool {
        self.units.iter().any(|u| u.alive && u.hero as usize == h)
    }

    /// 生贄にできる民と、その能力の合計
    pub fn sacrifice_power(&self, n: usize, hs: &[Handle]) -> (Vec<Handle>, f32) {
        let mut v = Vec::new();
        let mut p = 0.0;
        for &h in hs {
            if let Some(u) = self.unit(h) {
                if u.nation as usize == n && u.kind == UnitKind::Worker && v.len() < SACRIFICE_MAX {
                    let (x, y) = u.pos.tile();
                    if Map::inb(x, y) && self.owner[Map::idx(x, y)] as usize == n {
                        v.push(h);
                        p += u.power;
                    }
                }
            }
        }
        (v, p)
    }

    /// 召喚できるか
    pub fn can_summon(&self, n: usize, hero: usize, hs: &[Handle]) -> Result<(), &'static str> {
        if !self.heroic {
            return Err(t!("英霊モードではない", "Heroic Spirits mode is off", "未开启英灵模式"));
        }
        if self.hero_of(n).is_some() {
            return Err(t!("英霊は一国に一体だけ", "Only one heroic spirit per state", "每国只能有一位英灵"));
        }
        if self.hero_on_field(hero) {
            return Err(t!("この英霊は既に他国に召喚されている", "Another state has already summoned this spirit", "该英灵已被他国召唤"));
        }
        let (v, _) = self.sacrifice_power(n, hs);
        if v.is_empty() {
            return Err(t!("自国領内の民を選んで生贄にする", "Select workers inside your territory to sacrifice", "请选择己方领土内的民作为祭品"));
        }
        Ok(())
    }

    /// 民を生贄に捧げて英霊を召喚する。英霊の能力＝生贄の能力の合計×英霊の倍率
    pub fn summon_hero(&mut self, n: usize, hero: usize, hs: &[Handle]) -> Result<Handle, &'static str> {
        self.can_summon(n, hero, hs)?;
        let (v, p) = self.sacrifice_power(n, hs);
        let mut c = v2(0.0, 0.0);
        for &h in v.iter() {
            c = c + self.unit(h).unwrap().pos;
        }
        let c = self.passable_at_or_near(c * (1.0 / v.len() as f32));
        for &h in v.iter() {
            self.kill_unit(h.idx as usize);
        }
        let d = &HEROES[hero];
        let power = (p * d.mult).max(1.0);
        let h = self.spawn_unit(n, d.kind, c, power);
        self.unit_mut(h).unwrap().hero = hero as u8;
        let who = tx(self.nations[n].name);
        if n == self.player {
            let text = tf!("民{}人を生贄に捧げ、{}（{}）の英霊が降り立った。能力 {:.1}（生贄 {:.1} ×{:.1}）", "{} workers were sacrificed, and the spirit of {} ({}) has descended. Power {:.1} (sacrifice {:.1} ×{:.1})", "献祭{}名民，{}（{}）的英灵降临了。能力{:.1}（祭品{:.1}×{:.1}）", v.len(), tx(d.name), tx(d.era), power, p, d.mult);
            self.push_msg(text.clone(), [1.0, 0.85, 0.5]);
            self.queue_banner(Banner { title: tf!("英霊召喚・{}", "Heroic Spirit: {}", "召唤英灵・{}", tx(d.name)), text: format!("{}\n{}", tx(d.desc), text), t: 7.0 });
        } else {
            self.push_msg(tf!("{} が英霊・{}を召喚した", "{} summoned the heroic spirit of {}", "{}召唤了英灵{}", who, tx(d.name)), [0.95, 0.8, 0.6]);
        }
        Ok(h)
    }

    fn passable_at_or_near(&self, p: V2) -> V2 {
        if self.passable_at(p) {
            p
        } else {
            let (cx, cy) = p.tile();
            for r in 1..10i32 {
                for dy in -r..=r {
                    for dx in -r..=r {
                        let q = v2((cx + dx) as f32 + 0.5, (cy + dy) as f32 + 0.5);
                        if self.passable_at(q) {
                            return q;
                        }
                    }
                }
            }
            p
        }
    }

    /// 英霊が守っている城邑か（その国の英霊が近くにいる）
    pub fn hero_guarded(&self, bi: usize) -> bool {
        if !self.heroic {
            return false;
        }
        let b = &self.blds[bi];
        let n = b.nation as usize;
        let c = b.center();
        self.hero_of(n).map_or(false, |i| self.units[i].pos.dist(c) <= HERO_GUARD)
    }

    /// AI：戦時や上帝降臨時に、民を生贄に英霊を呼ぶ
    pub fn ai_summon(&mut self, n: usize, rng: &mut Rng) {
        if !self.heroic || self.hero_of(n).is_some() {
            return;
        }
        let urgent = self.descent.is_some();
        let at_war = (0..NATIONS.len()).any(|o| self.at_war(n, o));
        if !(urgent || at_war) || !rng.chance(if urgent { 0.2 } else { 0.03 }) {
            return;
        }
        let workers: Vec<Handle> = (0..self.units.len())
            .filter(|&i| self.units[i].alive && self.units[i].nation as usize == n && self.units[i].kind == UnitKind::Worker)
            .map(|i| self.uh(i))
            .collect();
        if workers.len() < 6 {
            return;
        }
        let k = if urgent { workers.len() / 2 } else { 3 };
        let hs: Vec<Handle> = workers.into_iter().take(k.clamp(2, SACRIFICE_MAX)).collect();
        // 地上にいない英霊から、強いほど選ばれやすく
        let cands: Vec<usize> = (0..HEROES.len()).filter(|&h| !self.hero_on_field(h)).collect();
        if cands.is_empty() {
            return;
        }
        let tot: f32 = cands.iter().map(|&h| HEROES[h].mult).sum();
        let mut x = rng.f32() * tot;
        let mut pick = cands[0];
        for &h in cands.iter() {
            x -= HEROES[h].mult;
            pick = h;
            if x <= 0.0 {
                break;
            }
        }
        let _ = self.summon_hero(n, pick, &hs);
    }
}

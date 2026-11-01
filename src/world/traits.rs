//! 国ごとの史実に基づく特徴

use super::*;

/// 特徴による補正（1.0＝補正なし）
#[derive(Clone, Copy, Debug)]
pub struct TraitMods {
    /// 生産（城邑の税・採集）
    pub eco: f32,
    /// 兵種ごとの攻撃（民・戈兵・弩兵・戦車/騎兵）
    pub atk: [f32; 4],
    /// 兵種ごとの訓練費
    pub cost: [f32; 4],
    /// 軍の維持費
    pub upkeep: f32,
    /// 城邑・建物が受ける損害
    pub def: f32,
    /// 商人の集まりやすさ
    pub attract: f32,
    /// 市場の値
    pub market: f32,
    /// 部族の略奪で失う蓄え
    pub plunder: f32,
    /// 鍛錬・教化の費用
    pub upgrade: f32,
}

impl TraitMods {
    pub const NONE: TraitMods = TraitMods { eco: 1.0, atk: [1.0; 4], cost: [1.0; 4], upkeep: 1.0, def: 1.0, attract: 1.0, market: 1.0, plunder: 1.0, upgrade: 1.0 };
}

pub struct Trait {
    pub nation: usize,
    pub name: &'static str,
    pub desc: &'static str,
    /// この年（紀元前）から有効
    pub from_bc: i32,
    /// この年（紀元前）まで有効
    pub to_bc: i32,
    pub fx: fn(&mut TraitMods),
}

const ZHOU: usize = 0;
const JIN_: usize = 1;
const QIN: usize = 2;
const CHU: usize = 3;
const QI: usize = 4;
const YAN: usize = 5;
const LU: usize = 6;
const SONG: usize = 7;
const EI: usize = 8;
const ZHENG: usize = 9;
const WU: usize = 14;
const YUE: usize = 15;
const ZHONGSHAN: usize = 16;
const BA: usize = 22;
const SHU: usize = 23;
const DI: usize = 28;
const YIQU: usize = 29;
const DAI: usize = 36;
const LOUFAN: usize = 37;

const ALL: (i32, i32) = (770, 221);

macro_rules! tr {
    ($n:expr, $name:expr, $desc:expr, $span:expr, $f:expr) => {
        Trait { nation: $n, name: $name, desc: $desc, from_bc: $span.0, to_bc: $span.1, fx: $f }
    };
}

pub const TRAITS: &[Trait] = &[
    tr!(ZHOU, "天子の王畿", "春秋時代は天子の権威により滅びない（最後の城邑を落とされても攻め手は兵を引く）。洛邑は天下の中心で商人が集まる（×1.3）", ALL, |m| m.attract *= 1.3),
    tr!(JIN_, "戦車の大国", "城濮の戦いで楚を破った覇者。戦車の攻撃+20%、維持費-10%", (770, 453), |m| {
        m.atk[3] *= 1.2;
        m.upkeep *= 0.9;
    }),
    tr!(QIN, "関中の険", "函谷関に守られた四塞の地。城邑・建物の受ける損害-30%", ALL, |m| m.def *= 0.7),
    tr!(QIN, "軍功爵制", "商鞅の変法以後、首級で爵位を得る。兵の訓練費-15%", (356, 221), |m| {
        for k in 1..4 {
            m.cost[k] *= 0.85;
        }
    }),
    tr!(CHU, "地方五千里", "長江・漢水の豊かな南方の大国。生産+10%、戈兵の攻撃+10%", ALL, |m| {
        m.eco *= 1.1;
        m.atk[1] *= 1.1;
    }),
    tr!(QI, "魚塩の利", "管仲以来、海の塩と魚で富む商業の国。生産+10%、商人が集まる（×1.4）", ALL, |m| {
        m.eco *= 1.1;
        m.attract *= 1.4;
    }),
    tr!(YAN, "北辺の守り", "長城を築いて胡に備えた国。部族の略奪で失う蓄え半減、騎兵の攻撃+10%", ALL, |m| {
        m.plunder *= 0.5;
        m.atk[3] *= 1.1;
    }),
    tr!(LU, "周礼尽く魯に在り", "周公の国、礼と学問の国（孔子の故国）。鍛錬・教化の費用-30%", ALL, |m| m.upgrade *= 0.7),
    tr!(SONG, "殷の後裔", "商の遺民の国で、商人の語源とも言われる。商人が集まる（×1.6）", ALL, |m| m.attract *= 1.6),
    tr!(EI, "小国の知恵", "大国の間を巧みに渡り、前209年まで続いた。城邑・建物の受ける損害-20%", ALL, |m| m.def *= 0.8),
    tr!(ZHENG, "商人の国", "商人と盟約を結び（弦高の故事）、新鄭は交易で栄えた。市場の値-25%、商人が集まる（×1.2）", ALL, |m| {
        m.market *= 0.75;
        m.attract *= 1.2;
    }),
    tr!(WU, "呉越の名剣", "干将・莫邪の剣、孫武の兵法。戈兵の攻撃+25%", ALL, |m| m.atk[1] *= 1.25),
    tr!(YUE, "臥薪嘗胆", "苦難に耐えて国を富ませ、呉を滅ぼした。生産+10%、維持費-20%", ALL, |m| {
        m.eco *= 1.1;
        m.upkeep *= 0.8;
    }),
    tr!(ZHONGSHAN, "千乗の国", "白狄が建てた堅固な城の国。城邑・建物の受ける損害-25%、騎兵の攻撃+10%", ALL, |m| {
        m.def *= 0.75;
        m.atk[3] *= 1.1;
    }),
    tr!(BA, "巴師の勇", "武王の殷討伐で歌い舞って戦った巴の兵。戈兵の攻撃+20%", ALL, |m| m.atk[1] *= 1.2),
    tr!(SHU, "天府の国", "成都平原の沃野。生産+20%", ALL, |m| m.eco *= 1.2),
    tr!(DI, "騎馬の民", "北狄の騎馬。騎兵（戦車）の攻撃+20%、訓練費-20%", ALL, |m| {
        m.atk[3] *= 1.2;
        m.cost[3] *= 0.8;
    }),
    tr!(YIQU, "騎馬の民", "西戎の雄。騎兵（戦車）の攻撃+20%、訓練費-20%", ALL, |m| {
        m.atk[3] *= 1.2;
        m.cost[3] *= 0.8;
    }),
    tr!(DAI, "騎馬の民", "北辺の代は良馬の産地。騎兵（戦車）の攻撃+20%、訓練費-20%", ALL, |m| {
        m.atk[3] *= 1.2;
        m.cost[3] *= 0.8;
    }),
    tr!(LOUFAN, "騎馬の民", "北狄の騎射。騎兵（戦車）の攻撃+20%、訓練費-20%", ALL, |m| {
        m.atk[3] *= 1.2;
        m.cost[3] *= 0.8;
    }),
    tr!(HAN, "天下の強弓勁弩", "「天下の強弓勁弩は皆韓より出づ」。弩兵の攻撃+25%", ALL, |m| m.atk[2] *= 1.25),
    tr!(ZHAO, "胡服騎射", "前307年、武霊王が胡の服と騎射を取り入れた。騎兵の攻撃+20%、訓練費-20%", (307, 221), |m| {
        m.atk[3] *= 1.2;
        m.cost[3] *= 0.8;
    }),
    tr!(WEI, "魏武卒", "呉起が鍛えた重装歩兵と李悝の変法。戈兵の攻撃+20%、生産+5%", ALL, |m| {
        m.atk[1] *= 1.2;
        m.eco *= 1.05;
    }),
];

/// その年に有効な特徴
pub fn traits_of(n: usize, yb: i32) -> impl Iterator<Item = &'static Trait> {
    TRAITS.iter().filter(move |t| t.nation == n && yb <= t.from_bc && yb >= t.to_bc)
}

/// 年代を問わずその国の特徴すべて
pub fn all_traits_of(n: usize) -> impl Iterator<Item = &'static Trait> {
    TRAITS.iter().filter(move |t| t.nation == n)
}

impl World {
    /// 特徴の補正を計算し直す（毎年）
    pub(super) fn refresh_traits(&mut self) {
        let yb = self.year_bc();
        for i in 0..NATIONS.len() {
            let mut m = TraitMods::NONE;
            for t in traits_of(i, yb) {
                (t.fx)(&mut m);
            }
            self.nations[i].tm = m;
        }
    }

    /// 周は春秋時代には滅びない
    pub fn immortal(&self, n: usize) -> bool {
        n == ZHOU && self.era == Era::Spring
    }

    /// その国の兵種の訓練費
    pub fn train_cost(&self, n: usize, k: UnitKind) -> [f32; 3] {
        let m = if n < NATIONS.len() { self.nations[n].tm.cost[k as usize] } else { 1.0 };
        let c = k.cost();
        [(c[0] * m).round(), (c[1] * m).round(), (c[2] * m).round()]
    }

    /// 市場の値
    pub fn market_price(&self, n: usize, k: usize) -> f32 {
        (MARKET_PRICE[k] * self.nations[n].tm.market).round()
    }
}

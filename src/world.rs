//! ゲーム世界のシミュレーション（ユニット・建物・経済・戦闘・外交・イベント）

use crate::ai::AiState;
use crate::geo::*;
use crate::path::PathFinder;
use crate::rng::Rng;
use std::collections::VecDeque;

mod history;
mod trade;
mod traits;
mod descent;
mod heroes;
mod save;
pub use history::{Chron, Person, HISTORY, INITIAL_HOUSE};
#[cfg(test)]
pub use history::Fx;
pub use trade::*;
pub use traits::*;
pub use descent::*;
pub use heroes::*;
pub use save::SAVE_VERSION;
pub use crate::lang::{sep, tx};

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}
pub fn v2(x: f32, y: f32) -> V2 {
    V2 { x, y }
}
impl std::ops::Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
}
impl std::ops::Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
}
impl std::ops::Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, s: f32) -> V2 {
        v2(self.x * s, self.y * s)
    }
}
impl V2 {
    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    pub fn dist(self, o: V2) -> f32 {
        (self - o).len()
    }
    pub fn norm(self) -> V2 {
        let l = self.len();
        if l > 1e-6 {
            self * (1.0 / l)
        } else {
            v2(0.0, 0.0)
        }
    }
    pub fn tile(self) -> (i32, i32) {
        (self.x.floor() as i32, self.y.floor() as i32)
    }
}

/// 1年の長さ（秒）。上帝降臨までに天下統一を目指せるよう、ゆっくり流す
pub const SEC_PER_YEAR: f32 = 8.0;
/// 戦国時代は時間の流れを遅くする（1年＝16秒）
pub const WARRING_SLOW: f32 = 2.0;
pub const START_BC: f32 = 770.0;
pub const WARRING_BC: f32 = 453.0;
pub const END_BC: f32 = 221.0;
pub const DT: f32 = 1.0 / 30.0;
pub const NONE: u8 = 255;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Era {
    Spring,
    Warring,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnitKind {
    Worker,
    Infantry,
    Archer,
    Chariot,
}

impl UnitKind {
    pub fn military(self) -> bool {
        self != UnitKind::Worker
    }
    /// 視界半径（タイル）
    pub fn vision(self) -> i32 {
        match self {
            UnitKind::Worker => 6,
            UnitKind::Infantry => 7,
            UnitKind::Archer => 8,
            UnitKind::Chariot => 9,
        }
    }
    pub fn name(self, era: Era) -> &'static str {
        match self {
            UnitKind::Worker => t!("民", "Worker", "民"),
            UnitKind::Infantry => t!("戈兵", "Halberdier", "戈兵"),
            UnitKind::Archer => t!("弩兵", "Crossbowman", "弩兵"),
            UnitKind::Chariot => {
                if era == Era::Spring {
                    t!("戦車", "Chariot", "战车")
                } else {
                    t!("騎兵", "Cavalry", "骑兵")
                }
            }
        }
    }
    /// [糧, 木, 銅]
    pub fn cost(self) -> [f32; 3] {
        match self {
            UnitKind::Worker => [50.0, 0.0, 0.0],
            UnitKind::Infantry => [60.0, 0.0, 25.0],
            UnitKind::Archer => [50.0, 35.0, 15.0],
            UnitKind::Chariot => [80.0, 45.0, 40.0],
        }
    }
    pub fn train_time(self) -> f32 {
        match self {
            UnitKind::Worker => 8.0,
            UnitKind::Infantry => 10.0,
            UnitKind::Archer => 11.0,
            UnitKind::Chariot => 14.0,
        }
    }
    pub fn base_hp(self) -> f32 {
        match self {
            UnitKind::Worker => 40.0,
            UnitKind::Infantry => 120.0,
            UnitKind::Archer => 75.0,
            UnitKind::Chariot => 140.0,
        }
    }
    pub fn base_atk(self) -> f32 {
        match self {
            UnitKind::Worker => 3.0,
            UnitKind::Infantry => 12.0,
            UnitKind::Archer => 9.0,
            UnitKind::Chariot => 14.0,
        }
    }
    pub fn range(self) -> f32 {
        match self {
            UnitKind::Archer => 4.5,
            _ => 0.75,
        }
    }
    pub fn cooldown(self) -> f32 {
        match self {
            UnitKind::Archer => 1.6,
            UnitKind::Chariot => 1.4,
            _ => 1.2,
        }
    }
    pub fn speed(self, era: Era) -> f32 {
        match self {
            UnitKind::Worker => 1.9,
            UnitKind::Infantry => 1.9,
            UnitKind::Archer => 1.75,
            UnitKind::Chariot => {
                if era == Era::Spring {
                    2.9
                } else {
                    3.5
                }
            }
        }
    }
    pub fn sight(self) -> f32 {
        match self {
            UnitKind::Worker => 4.0,
            UnitKind::Archer => 7.0,
            _ => 6.0,
        }
    }
    /// 相性補正
    pub fn bonus(self, def: UnitKind) -> f32 {
        match (self, def) {
            (UnitKind::Infantry, UnitKind::Chariot) => 1.6,
            (UnitKind::Chariot, UnitKind::Archer) => 1.7,
            (UnitKind::Archer, UnitKind::Infantry) => 1.4,
            (UnitKind::Chariot, UnitKind::Worker) => 1.3,
            _ => 1.0,
        }
    }
    pub fn vs_building(self) -> f32 {
        match self {
            UnitKind::Infantry => 1.0,
            UnitKind::Archer => 0.35,
            UnitKind::Chariot => 0.7,
            UnitKind::Worker => 0.4,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BKind {
    Capital,
    Town,
    Barracks,
    Farm,
    /// 要塞：近づく敵兵を射る（数に上限あり）
    Fort,
}

/// 城邑が無くても持てる民・兵の数（滅んだ国から分けられる量）
pub const BASE_CAP: u32 = 4;

/// 要塞の射程・攻撃間隔
pub const FORT_RANGE: f32 = 9.0;
pub const FORT_COOLDOWN: f32 = 1.5;

impl BKind {
    pub fn name(self) -> &'static str {
        match self {
            BKind::Capital => t!("都", "Capital", "都城"),
            BKind::Town => t!("邑", "Town", "邑"),
            BKind::Barracks => t!("兵営", "Barracks", "兵营"),
            BKind::Farm => t!("田", "Field", "田"),
            BKind::Fort => t!("要塞", "Fort", "要塞"),
        }
    }
    pub fn cost(self) -> [f32; 3] {
        match self {
            BKind::Capital => [0.0, 0.0, 0.0],
            BKind::Town => [100.0, 250.0, 80.0],
            BKind::Barracks => [0.0, 150.0, 30.0],
            BKind::Farm => [0.0, 60.0, 0.0],
            BKind::Fort => [300.0, 400.0, 300.0],
        }
    }
    pub fn max_hp(self) -> f32 {
        match self {
            BKind::Capital => 3000.0,
            BKind::Town => 1600.0,
            BKind::Barracks => 1200.0,
            BKind::Farm => 300.0,
            BKind::Fort => 2500.0,
        }
    }
    pub fn build_time(self) -> f32 {
        match self {
            BKind::Capital => 60.0,
            BKind::Town => 40.0,
            BKind::Barracks => 20.0,
            BKind::Farm => 8.0,
            BKind::Fort => 30.0,
        }
    }
    pub fn radius(self) -> i32 {
        match self {
            BKind::Capital => 10,
            BKind::Town => 7,
            _ => 0,
        }
    }
    pub fn city(self) -> bool {
        matches!(self, BKind::Capital | BKind::Town)
    }
    pub fn blocks(self) -> bool {
        self != BKind::Farm
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Handle {
    pub idx: u32,
    pub gen: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Target {
    Unit(Handle),
    Bld(Handle),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Src {
    Farm(Handle),
    Forest(i32, i32),
    Mine(usize),
}
impl Src {
    pub fn res(self) -> usize {
        match self {
            Src::Farm(_) => 0,
            Src::Forest(..) => 1,
            Src::Mine(_) => 2,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Order {
    Idle,
    Move(V2),
    AttackMove(V2),
    Attack(Target),
    /// phase 0:資源へ 1:採集中 2:集積地へ
    Gather { src: Src, phase: u8 },
    Build(Handle),
}

pub struct Unit {
    pub gen: u32,
    pub alive: bool,
    pub nation: u8,
    pub kind: UnitKind,
    pub pos: V2,
    pub hp: f32,
    pub power: f32,
    pub order: Order,
    pub resume: Option<V2>,
    pub path: Vec<(i16, i16)>,
    pub path_i: usize,
    pub path_goal: (i32, i32),
    pub path_pending: bool,
    pub path_done: bool,
    pub cooldown: f32,
    pub carry_kind: usize,
    pub carry: f32,
    pub scan_t: f32,
    pub dir: V2,
    pub flash: f32,
    pub selected: bool,
    pub last_attacker: Option<Handle>,
    /// 部族の兵・傭兵の出身部族（NONE=なし）
    pub tribe: u8,
    /// 傭兵の契約満了年（0=傭兵ではない）
    pub merc_until: f32,
    /// 上帝の兵（英霊にしか傷つけられない）
    pub divine: bool,
    /// 英霊（HEROES の番号、NONE=英霊ではない）
    pub hero: u8,
    /// 建設の予約（今の建設を終えたら順に取りかかる）
    pub build_queue: VecDeque<Handle>,
}

impl Unit {
    pub fn max_hp(&self) -> f32 {
        self.kind.base_hp() * self.power
    }
    pub fn reset_nav(&mut self) {
        self.path.clear();
        self.path_i = 0;
        self.path_goal = (-9, -9);
        self.path_done = false;
    }
}

pub struct Building {
    pub gen: u32,
    pub alive: bool,
    pub nation: u8,
    pub kind: BKind,
    pub tx: i32,
    pub ty: i32,
    pub hp: f32,
    pub progress: f32,
    pub queue: VecDeque<UnitKind>,
    pub queue_t: f32,
    pub rally: Option<V2>,
    pub cooldown: f32,
}

impl Building {
    pub fn center(&self) -> V2 {
        v2(self.tx as f32 + 1.0, self.ty as f32 + 1.0)
    }
    pub fn done(&self) -> bool {
        self.progress >= 1.0
    }
    pub fn dist_to_footprint(&self, p: V2) -> f32 {
        let cx = p.x.clamp(self.tx as f32, self.tx as f32 + 2.0);
        let cy = p.y.clamp(self.ty as f32, self.ty as f32 + 2.0);
        p.dist(v2(cx, cy))
    }
}

pub struct Nation {
    pub name: &'static str,
    pub color: [f32; 3],
    pub bias: f32,
    pub alive: bool,
    pub appeared: bool,
    pub res: [f32; 3],
    pub aggression: f32,
    pub hegemon_cd: f32,
    pub capital: Option<Handle>,
    pub workers: u32,
    pub soldiers: u32,
    pub q_workers: u32,
    pub q_soldiers: u32,
    pub cities: u32,
    pub mil_power: f32,
    pub eco_power: f32,
    pub tiles: u32,
    pub upkeep: f32,
    pub strength: f32,
    pub earned: [f32; 3],
    pub income: [f32; 3],
    pub home: V2,
    /// 君主家（国姓）
    pub house: &'static str,
    /// 仕えている人物
    pub figures: Vec<Person>,
    /// 人物による生産・武の倍率
    pub eco_mul: f32,
    pub mil_mul: f32,
    /// 銭（商人からの税収）
    pub coin: f32,
    pub coin_earned: f32,
    pub coin_income: f32,
    /// 税率 0:軽税 1:中税 2:重税
    pub tax: u8,
    /// 国の特徴による補正
    pub tm: TraitMods,
    /// 滅んだ国から分けられた人口上限（民・兵それぞれに加わる）
    pub cap_bonus: u32,
}

impl Nation {
    pub fn worker_cap(&self) -> u32 {
        (4 + 3 * self.cities).min(25) + self.cap_bonus
    }
    pub fn mil_cap(&self) -> u32 {
        (4 + 3 * self.cities).min(25) + self.cap_bonus
    }
    /// 築ける要塞の数（城邑2つごとに1つ、最大6）
    pub fn fort_cap(&self) -> u32 {
        (1 + self.cities / 2).min(6)
    }
}

/// プレイヤーへの和睦の申し入れ（受諾か拒否かを選ぶ）
#[derive(Clone, Debug)]
pub struct PeaceAsk {
    pub from: usize,
    pub years: f32,
    pub why: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CoalKind {
    Hegemon,
    Hezong,
}

pub struct Coalition {
    pub kind: CoalKind,
    pub target: usize,
    pub leader: usize,
    pub members: Vec<usize>,
    pub end: f32,
}

pub struct Msg {
    pub text: String,
    pub time: f32,
    pub color: [f32; 3],
}

pub struct Shot {
    pub a: V2,
    pub b: V2,
    pub t: f32,
}

pub struct Banner {
    pub title: String,
    pub text: String,
    pub t: f32,
}

pub const GCELL: i32 = 4;
pub const GW: i32 = (MAP_W as i32 + GCELL - 1) / GCELL;
pub const GH: i32 = (MAP_H as i32 + GCELL - 1) / GCELL;

pub struct World {
    pub map: Map,
    pub cost: Vec<f32>,
    pub bmap: Vec<u32>,
    pub owner: Vec<u8>,
    pub units: Vec<Unit>,
    pub blds: Vec<Building>,
    free_units: Vec<u32>,
    free_blds: Vec<u32>,
    pub nations: Vec<Nation>,
    pub war: Vec<Vec<bool>>,
    pub war_start: Vec<Vec<f32>>,
    pub truce: Vec<Vec<f32>>,
    pub peace_offer: Vec<bool>,
    /// 和睦の申し入れを断られた国は、この年まで申し入れない
    pub offer_cd: Vec<f32>,
    /// プレイヤーの返答を待つ和睦の申し入れ
    pub peace_asks: VecDeque<PeaceAsk>,
    pub years: f32,
    pub era: Era,
    pub player: usize,
    pub coalitions: Vec<Coalition>,
    pub hezong_next: f32,
    pub msgs: Vec<Msg>,
    pub banner: Option<Banner>,
    banner_queue: VecDeque<Banner>,
    /// 次に起こる史実イベント
    hist_i: usize,
    /// 史書（起きた出来事・起こらなかった出来事）
    pub chronicle: Vec<Chron>,
    pub shots: Vec<Shot>,
    pub rng: Rng,
    pf: PathFinder,
    path_queue: VecDeque<Handle>,
    pub grid: Vec<Vec<u32>>,
    pub territory_dirty: bool,
    pub territory_version: u32,
    /// 国ごとの領土のマス数（recompute_territory で数え直す）
    tile_count: Vec<u32>,
    /// 国ごとの完成した城邑（毎ステップ作り直す。運び先を探すのに全建物を見ないため）
    city_list: Vec<Vec<u32>>,
    /// 押し合いを1ステップおきにするための拍子
    sep_tick: bool,
    pub terrain_dirty: Vec<(i32, i32)>,
    pub game_over: Option<(bool, String)>,
    /// チュートリアル中はプレイヤーへの宣戦を禁じる
    pub protect_player: bool,
    /// プレイヤーが部隊に命令した回数
    pub player_orders: u32,
    /// 上帝降臨（前221年までに天下が定まらなかった）
    pub descent: Option<Descent>,
    /// 撮影用：プレイヤーの国も AI が動かす
    pub ai_player: bool,
    /// 英霊モード（ゲーム開始時に選ぶ。既定は無効）
    pub heroic: bool,
    pub time: f32,
    pub ai: Vec<AiState>,
    /// 商人・西域の商人・山岳民と遊牧民
    pub trade: Trade,
    stat_t: f32,
    income_t: f32,
    year_tick: f32,
}

impl World {
    pub fn new(seed: u32) -> World {
        let map = generate_map(seed);
        let n = MAP_W * MAP_H;
        let mut cost = vec![0.0; n];
        for i in 0..n {
            cost[i] = map.terrain[i].move_cost();
        }
        let nn = NATIONS.len();
        let mut w = World {
            map,
            cost,
            bmap: vec![0; n],
            owner: vec![NONE; n],
            tile_count: Vec::new(),
            city_list: Vec::new(),
            sep_tick: false,
            units: Vec::new(),
            blds: Vec::new(),
            free_units: Vec::new(),
            free_blds: Vec::new(),
            nations: Vec::new(),
            war: vec![vec![false; nn]; nn],
            war_start: vec![vec![0.0; nn]; nn],
            truce: vec![vec![0.0; nn]; nn],
            peace_offer: vec![false; nn],
            offer_cd: vec![0.0; nn],
            peace_asks: VecDeque::new(),
            years: 0.0,
            era: Era::Spring,
            player: 0,
            coalitions: Vec::new(),
            hezong_next: 0.0,
            msgs: Vec::new(),
            banner: None,
            banner_queue: VecDeque::new(),
            hist_i: 0,
            chronicle: Vec::new(),
            shots: Vec::new(),
            rng: Rng::new(seed as u64 * 7 + 3),
            pf: PathFinder::new(),
            path_queue: VecDeque::new(),
            grid: vec![Vec::new(); (GW * GH) as usize],
            territory_dirty: true,
            territory_version: 0,
            terrain_dirty: Vec::new(),
            game_over: None,
            protect_player: false,
            player_orders: 0,
            descent: None,
            ai_player: false,
            heroic: false,
            time: 0.0,
            ai: Vec::new(),
            trade: Trade::new(&mut Rng::new(seed as u64 * 13 + 5)),
            stat_t: 0.0,
            income_t: 0.0,
            year_tick: 0.0,
        };
        for (i, d) in NATIONS.iter().enumerate() {
            let (hx, hy) = ll_to_tile(d.lon, d.lat);
            w.nations.push(Nation {
                name: d.name,
                color: d.color,
                bias: d.bias,
                alive: !d.dormant,
                appeared: !d.dormant,
                res: [200.0, 200.0, 100.0],
                aggression: 0.0,
                hegemon_cd: 0.0,
                capital: None,
                workers: 0,
                soldiers: 0,
                q_workers: 0,
                q_soldiers: 0,
                cities: 0,
                mil_power: 0.0,
                eco_power: 0.0,
                tiles: 0,
                upkeep: 0.0,
                strength: 0.0,
                earned: [0.0; 3],
                income: [0.0; 3],
                home: v2(hx, hy),
                house: INITIAL_HOUSE[i],
                figures: Vec::new(),
                eco_mul: 1.0,
                mil_mul: 1.0,
                coin: 0.0,
                coin_earned: 0.0,
                coin_income: 0.0,
                tax: 1,
                tm: TraitMods::NONE,
                cap_bonus: 0,
            });
            w.ai.push(AiState::new(i as u64 * 31 + seed as u64));
        }
        // 部族の兵の仮の枠（国としては扱わない）
        w.nations.push(Nation {
            name: "夷狄",
            color: [0.62, 0.42, 0.24],
            bias: 0.0,
            alive: false,
            appeared: false,
            res: [0.0; 3],
            aggression: 0.0,
            hegemon_cd: 0.0,
            capital: None,
            workers: 0,
            soldiers: 0,
            q_workers: 0,
            q_soldiers: 0,
            cities: 0,
            mil_power: 0.0,
            eco_power: 0.0,
            tiles: 0,
            upkeep: 0.0,
            strength: 0.0,
            earned: [0.0; 3],
            income: [0.0; 3],
            home: v2(0.0, 0.0),
            house: "",
            figures: Vec::new(),
            eco_mul: 1.0,
            mil_mul: 1.0,
            coin: 0.0,
            coin_earned: 0.0,
            coin_income: 0.0,
            tax: 1,
            tm: TraitMods::NONE,
            cap_bonus: 0,
        });
        for i in 0..nn {
            let d = &NATIONS[i];
            if d.dormant {
                continue;
            }
            let c = w.found_city(i, d.lon, d.lat, BKind::Capital);
            if let Some(c) = c {
                w.nations[i].capital = Some(c);
                let p = w.blds[c.idx as usize].center();
                w.nations[i].home = p;
                for k in [UnitKind::Worker, UnitKind::Worker, UnitKind::Worker, UnitKind::Infantry, UnitKind::Infantry, UnitKind::Archer] {
                    if let Some(sp) = w.spawn_point(c) {
                        w.spawn_unit(i, k, sp, 1.0);
                    }
                }
            }
            for &(lo, la) in d.extra {
                w.found_city(i, lo, la, BKind::Town);
            }
        }
        w.recompute_territory();
        w.update_stats();
        w.refresh_traits();
        w
    }

    fn found_city(&mut self, nation: usize, lon: f32, lat: f32, kind: BKind) -> Option<Handle> {
        let (fx, fy) = ll_to_tile(lon, lat);
        let (cx, cy) = (fx as i32 - 1, fy as i32 - 1);
        for r in 0..8i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (tx, ty) = (cx + dx, cy + dy);
                    if self.footprint_free(tx, ty) {
                        return Some(self.spawn_building(nation, kind, tx, ty, true));
                    }
                }
            }
        }
        None
    }

    // ------------------------------------------------------------ 基本操作
    pub fn unit(&self, h: Handle) -> Option<&Unit> {
        let u = self.units.get(h.idx as usize)?;
        if u.alive && u.gen == h.gen {
            Some(u)
        } else {
            None
        }
    }
    pub fn unit_mut(&mut self, h: Handle) -> Option<&mut Unit> {
        let u = self.units.get_mut(h.idx as usize)?;
        if u.alive && u.gen == h.gen {
            Some(u)
        } else {
            None
        }
    }
    pub fn bld(&self, h: Handle) -> Option<&Building> {
        let b = self.blds.get(h.idx as usize)?;
        if b.alive && b.gen == h.gen {
            Some(b)
        } else {
            None
        }
    }
    pub fn bld_mut(&mut self, h: Handle) -> Option<&mut Building> {
        let b = self.blds.get_mut(h.idx as usize)?;
        if b.alive && b.gen == h.gen {
            Some(b)
        } else {
            None
        }
    }
    pub fn uh(&self, i: usize) -> Handle {
        Handle { idx: i as u32, gen: self.units[i].gen }
    }
    pub fn bh(&self, i: usize) -> Handle {
        Handle { idx: i as u32, gen: self.blds[i].gen }
    }

    pub fn spawn_unit(&mut self, nation: usize, kind: UnitKind, pos: V2, power: f32) -> Handle {
        let u = Unit {
            gen: 0,
            alive: true,
            nation: nation as u8,
            kind,
            pos,
            hp: kind.base_hp() * power,
            power,
            order: Order::Idle,
            resume: None,
            path: Vec::new(),
            path_i: 0,
            path_goal: (-1, -1),
            path_pending: false,
            path_done: false,
            cooldown: 0.0,
            carry_kind: 0,
            carry: 0.0,
            scan_t: self.rng.f32() * 0.5,
            dir: v2(0.0, 1.0),
            flash: 0.0,
            selected: false,
            last_attacker: None,
            tribe: NONE,
            merc_until: 0.0,
            divine: false,
            hero: NONE,
            build_queue: VecDeque::new(),
        };
        if let Some(i) = self.free_units.pop() {
            let gen = self.units[i as usize].gen + 1;
            self.units[i as usize] = Unit { gen, ..u };
            Handle { idx: i, gen }
        } else {
            self.units.push(u);
            Handle { idx: self.units.len() as u32 - 1, gen: 0 }
        }
    }

    pub fn kill_unit(&mut self, i: usize) {
        if self.units[i].alive {
            self.units[i].alive = false;
            self.units[i].selected = false;
            self.units[i].path.clear();
            self.free_units.push(i as u32);
        }
    }

    pub fn footprint_free(&self, tx: i32, ty: i32) -> bool {
        for dy in 0..2 {
            for dx in 0..2 {
                let (x, y) = (tx + dx, ty + dy);
                if !Map::inb(x, y) {
                    return false;
                }
                let i = Map::idx(x, y);
                if !self.map.terrain[i].buildable() || self.bmap[i] != 0 {
                    return false;
                }
                if self.map.mines.iter().any(|m| m.x == x && m.y == y && m.amount > 0.0) {
                    return false;
                }
            }
        }
        true
    }

    /// 建設可能か（理由付き）
    pub fn can_place(&self, nation: usize, kind: BKind, tx: i32, ty: i32) -> Result<(), &'static str> {
        if !self.footprint_free(tx, ty) {
            return Err(t!("ここには建設できない", "Cannot build here", "此处无法建造"));
        }
        let c = v2(tx as f32 + 1.0, ty as f32 + 1.0);
        let own = |x: i32, y: i32| self.owner[Map::idx(x, y)];
        match kind {
            BKind::Town | BKind::Capital => {
                for b in self.blds.iter() {
                    if b.alive && b.kind.city() && b.center().dist(c) < 9.0 {
                        return Err(t!("他の城邑に近すぎる", "Too close to another city", "离其他城邑太近"));
                    }
                }
                for dy in 0..2 {
                    for dx in 0..2 {
                        let o = own(tx + dx, ty + dy);
                        if o != NONE && o as usize != nation {
                            return Err(t!("他国の領土には築城できない", "Cannot found a city in another state's territory", "不能在他国领土筑城"));
                        }
                    }
                }
                // 自国領から遠すぎない
                let near = self.blds.iter().any(|b| b.alive && b.nation as usize == nation && b.kind.city() && b.center().dist(c) < 26.0);
                if !near {
                    return Err(t!("自国の城邑から遠すぎる", "Too far from your cities", "离己方城邑太远"));
                }
            }
            _ => {
                for dy in 0..2 {
                    for dx in 0..2 {
                        if own(tx + dx, ty + dy) as usize != nation {
                            return Err(t!("自国の領土内に建設する必要がある", "Must be built inside your territory", "必须建在己方领土内"));
                        }
                    }
                }
                if kind == BKind::Fort && self.fort_count(nation) >= self.nations[nation].fort_cap() {
                    return Err(t!("要塞の数が上限に達している（城邑を増やすと上限が上がる）", "Fort limit reached (more cities raise the limit)", "要塞数量已达上限（城邑越多上限越高）"));
                }
            }
        }
        Ok(())
    }

    /// 建設中を含む要塞の数
    pub fn fort_count(&self, nation: usize) -> u32 {
        self.blds.iter().filter(|b| b.alive && b.nation as usize == nation && b.kind == BKind::Fort).count() as u32
    }

    pub fn spawn_building(&mut self, nation: usize, kind: BKind, tx: i32, ty: i32, complete: bool) -> Handle {
        let b = Building {
            gen: 0,
            alive: true,
            nation: nation as u8,
            kind,
            tx,
            ty,
            hp: if complete { kind.max_hp() } else { kind.max_hp() * 0.1 },
            progress: if complete { 1.0 } else { 0.0 },
            queue: VecDeque::new(),
            queue_t: 0.0,
            rally: None,
            cooldown: 0.0,
        };
        let h = if let Some(i) = self.free_blds.pop() {
            let gen = self.blds[i as usize].gen + 1;
            self.blds[i as usize] = Building { gen, ..b };
            Handle { idx: i, gen }
        } else {
            self.blds.push(b);
            Handle { idx: self.blds.len() as u32 - 1, gen: 0 }
        };
        for dy in 0..2 {
            for dx in 0..2 {
                let i = Map::idx(tx + dx, ty + dy);
                self.bmap[i] = h.idx + 1;
                if kind.blocks() {
                    self.cost[i] = f32::INFINITY;
                }
            }
        }
        if kind.city() && complete {
            self.territory_dirty = true;
        }
        h
    }

    pub fn remove_building(&mut self, i: usize) {
        let b = &self.blds[i];
        if !b.alive {
            return;
        }
        let (tx, ty, city) = (b.tx, b.ty, b.kind.city());
        for dy in 0..2 {
            for dx in 0..2 {
                let ti = Map::idx(tx + dx, ty + dy);
                self.bmap[ti] = 0;
                self.cost[ti] = self.map.terrain[ti].move_cost();
            }
        }
        self.blds[i].alive = false;
        self.free_blds.push(i as u32);
        if city {
            self.territory_dirty = true;
        }
    }

    pub fn spawn_point(&self, h: Handle) -> Option<V2> {
        let b = self.bld(h)?;
        let mut fallback = None;
        for r in 1..6 {
            let mut cands = Vec::new();
            for dy in -r..=1 + r {
                for dx in -r..=1 + r {
                    let (x, y) = (b.tx + dx, b.ty + dy);
                    if dx > -r && dx < 1 + r && dy > -r && dy < 1 + r {
                        continue;
                    }
                    if Map::inb(x, y) && self.cost[Map::idx(x, y)].is_finite() {
                        cands.push(v2(x as f32 + 0.5, y as f32 + 0.5));
                    }
                }
            }
            if !cands.is_empty() {
                let free: Vec<V2> = cands
                    .iter()
                    .copied()
                    .filter(|&p| !self.units.iter().any(|u| u.alive && u.pos.dist(p) < 0.85))
                    .collect();
                if !free.is_empty() {
                    let k = (self.units.len().wrapping_mul(3) + (self.time * 7.3) as usize) % free.len();
                    return Some(free[k]);
                }
                if fallback.is_none() {
                    let k = (self.units.len() + (self.time * 7.3) as usize) % cands.len();
                    fallback = Some(cands[k]);
                }
            }
        }
        fallback
    }

    pub fn passable_at(&self, p: V2) -> bool {
        let (x, y) = p.tile();
        Map::inb(x, y) && self.cost[Map::idx(x, y)].is_finite()
    }

    pub fn line_clear(&self, a: V2, b: V2) -> bool {
        let d = b - a;
        let l = d.len();
        let n = (l / 0.4).ceil() as i32;
        for s in 1..=n {
            let p = a + d * (s as f32 / n as f32);
            if !self.passable_at(p) {
                return false;
            }
        }
        true
    }

    pub fn push_msg(&mut self, text: String, color: [f32; 3]) {
        self.msgs.push(Msg { text, time: self.time, color });
        if self.msgs.len() > 60 {
            self.msgs.remove(0);
        }
    }

    /// 1年あたりの秒数（戦国時代は遅くなる）
    pub fn sec_per_year(&self) -> f32 {
        if self.era == Era::Spring {
            SEC_PER_YEAR
        } else {
            SEC_PER_YEAR * WARRING_SLOW
        }
    }

    pub fn year_bc(&self) -> i32 {
        (START_BC - self.years).ceil() as i32
    }

    pub fn power_cap(&self) -> f32 {
        if self.era == Era::Spring {
            4.0
        } else {
            6.0
        }
    }

    /// 全国共通の時代補正（一部の国だけ文明が進むことはない）
    pub fn era_mult(&self) -> f32 {
        1.0 + 0.5 * (self.years / (START_BC - END_BC))
    }

    pub fn at_war(&self, a: usize, b: usize) -> bool {
        // 略奪に来た部族の兵とはどの国も戦う
        a != b && (a == TRIBE || b == TRIBE || self.war[a][b])
    }

    pub fn coalition_locked(&self, a: usize, b: usize) -> bool {
        self.coalitions.iter().any(|c| (c.target == a && c.members.contains(&b)) || (c.target == b && c.members.contains(&a)))
    }

    pub fn declare_war(&mut self, a: usize, b: usize, forced: bool) -> bool {
        if a == b || self.war[a][b] || !self.nations[a].alive || !self.nations[b].alive {
            return false;
        }
        if !forced && self.truce[a][b] > self.years {
            return false;
        }
        self.war[a][b] = true;
        self.war[b][a] = true;
        self.war_start[a][b] = self.years;
        self.war_start[b][a] = self.years;
        self.peace_offer[a] = false;
        self.peace_offer[b] = false;
        if !forced {
            self.nations[a].aggression += 30.0;
        }
        let p = self.player;
        if a == p || b == p {
            let t = tf!("{} が {} に宣戦布告した", "{} declared war on {}", "{}向{}宣战", tx(self.nations[a].name), tx(self.nations[b].name));
            self.push_msg(t, [1.0, 0.45, 0.35]);
        }
        true
    }

    pub fn make_peace(&mut self, a: usize, b: usize, truce_years: f32) {
        if !self.war[a][b] {
            return;
        }
        // 上帝の降臨した秦とは和睦できない
        if self.descent.is_some() && (a == 2 || b == 2) {
            return;
        }
        self.war[a][b] = false;
        self.war[b][a] = false;
        self.truce[a][b] = self.years + truce_years;
        self.truce[b][a] = self.years + truce_years;
        if a == self.player || b == self.player {
            let o = if a == self.player { b } else { a };
            self.peace_offer[o] = false;
            let t = tf!("{} と和睦した（{}年間の休戦）", "Made peace with {} ({}-year truce)", "与{}议和（休战{}年）", tx(self.nations[o].name), truce_years as i32);
            self.push_msg(t, [0.6, 0.9, 0.6]);
        }
    }

    /// 和睦を申し入れる。プレイヤーが当事者なら返答を待ち（false）、それ以外は即座に和睦する（true）
    pub fn propose_peace(&mut self, a: usize, b: usize, truce_years: f32, why: &str) -> bool {
        let p = self.player;
        if (a != p && b != p) || self.ai_player {
            self.make_peace(a, b, truce_years);
            return true;
        }
        let o = if a == p { b } else { a };
        if !self.war[a][b] || (self.descent.is_some() && o == 2) || self.peace_asks.iter().any(|q| q.from == o) {
            return false;
        }
        let t = tf!("{} が和睦を申し入れてきた（{}）", "{} offers peace ({})", "{}提出议和（{}）", tx(self.nations[o].name), why);
        self.push_msg(t, [0.7, 0.9, 1.0]);
        self.peace_asks.push_back(PeaceAsk { from: o, years: truce_years, why: why.to_string() });
        false
    }

    /// 返答待ちの申し入れのうち、既に意味のないもの（交戦が終わった・滅んだ）を捨てる
    pub fn prune_peace_asks(&mut self) {
        let p = self.player;
        let (war, nations) = (&self.war, &self.nations);
        self.peace_asks.retain(|q| q.from != p && nations[q.from].alive && war[p][q.from]);
    }

    /// 先頭の和睦の申し入れに答える
    pub fn answer_peace(&mut self, accept: bool) {
        self.prune_peace_asks();
        let Some(q) = self.peace_asks.pop_front() else { return };
        let p = self.player;
        if accept {
            self.make_peace(p, q.from, q.years);
        } else {
            self.peace_offer[q.from] = false;
            self.offer_cd[q.from] = self.years + 8.0;
            let t = tf!("{} の和睦の申し入れを退けた（交戦を続ける）", "Refused {}'s offer of peace (the war goes on)", "拒绝了{}的议和（继续交战）", tx(self.nations[q.from].name));
            self.push_msg(t, [1.0, 0.6, 0.4]);
        }
    }

    // ------------------------------------------------------------ 領土
    pub fn recompute_territory(&mut self) {
        let n = MAP_W * MAP_H;
        let mut best = vec![f32::MAX; n];
        self.owner.fill(NONE);
        for b in self.blds.iter() {
            if !b.alive || !b.kind.city() || !b.done() {
                continue;
            }
            let r = b.kind.radius();
            let c = b.center();
            let (cx, cy) = (c.x as i32, c.y as i32);
            for dy in -r..=r {
                for dx in -r..=r {
                    let (x, y) = (cx + dx, cy + dy);
                    if !Map::inb(x, y) {
                        continue;
                    }
                    let d = ((x as f32 + 0.5 - c.x).powi(2) + (y as f32 + 0.5 - c.y).powi(2)).sqrt();
                    let rr = r as f32;
                    if d > rr {
                        continue;
                    }
                    let i = Map::idx(x, y);
                    if self.map.terrain[i] == Terrain::Sea {
                        continue;
                    }
                    let score = d / rr;
                    if score < best[i] {
                        best[i] = score;
                        self.owner[i] = b.nation;
                    }
                }
            }
        }
        let mut tc = vec![0u32; self.nations.len()];
        for &o in self.owner.iter() {
            if (o as usize) < tc.len() {
                tc[o as usize] += 1;
            }
        }
        self.tile_count = tc;
        self.territory_dirty = false;
        self.territory_version += 1;
    }

    // ------------------------------------------------------------ 統計
    pub fn update_stats(&mut self) {
        for n in self.nations.iter_mut() {
            n.workers = 0;
            n.soldiers = 0;
            n.q_workers = 0;
            n.q_soldiers = 0;
            n.cities = 0;
            n.mil_power = 0.0;
            n.eco_power = 0.0;
            n.upkeep = 0.0;
            n.tiles = 0;
        }
        for u in self.units.iter() {
            if !u.alive {
                continue;
            }
            let n = &mut self.nations[u.nation as usize];
            if u.merc_until > 0.0 || u.divine || u.hero != NONE {
                // 傭兵は銭で雇われており、上帝の兵は天のもの。兵数の上限・維持費の外
                n.mil_power += u.power * (u.hp / u.max_hp()).max(0.2);
            } else if u.kind.military() {
                n.soldiers += 1;
                n.mil_power += u.power * (u.hp / u.max_hp()).max(0.2);
                n.upkeep += 0.07 * u.power;
            } else {
                n.workers += 1;
                n.eco_power += u.power;
            }
        }
        for b in self.blds.iter() {
            if !b.alive {
                continue;
            }
            let n = &mut self.nations[b.nation as usize];
            if b.kind.city() && b.done() {
                n.cities += 1;
            }
            for k in b.queue.iter() {
                if k.military() {
                    n.q_soldiers += 1;
                } else {
                    n.q_workers += 1;
                }
            }
        }
        for (k, n) in self.nations.iter_mut().enumerate() {
            n.tiles = self.tile_count.get(k).copied().unwrap_or(0);
        }
        for n in self.nations.iter_mut() {
            n.upkeep *= n.tm.upkeep;
            n.strength = n.mil_power * 10.0 + n.cities as f32 * 25.0 + n.eco_power * 4.0 + n.tiles as f32 * 0.03;
        }
    }

    // ------------------------------------------------------------ 命令
    /// プレイヤーの部隊への命令を数える（指揮中の一時停止を解くため）
    fn note_player_order(&mut self, hs: &[Handle]) {
        let p = self.player;
        if hs.iter().any(|&h| self.unit(h).map_or(false, |u| u.nation as usize == p)) {
            self.player_orders = self.player_orders.wrapping_add(1);
        }
    }

    pub fn order_move(&mut self, hs: &[Handle], goal: V2, attack: bool) {
        self.note_player_order(hs);
        let n = hs.len();
        let cols = (n as f32).sqrt().ceil() as i32;
        for (k, &h) in hs.iter().enumerate() {
            let (r, c) = (k as i32 / cols.max(1), k as i32 % cols.max(1));
            let off = v2((c - cols / 2) as f32 * 0.8, (r - cols / 2) as f32 * 0.8);
            let mut g = goal + if n > 1 { off } else { v2(0.0, 0.0) };
            if !self.passable_at(g) {
                g = goal;
            }
            if let Some(u) = self.unit_mut(h) {
                u.order = if attack && u.kind.military() { Order::AttackMove(g) } else { Order::Move(g) };
                u.resume = None;
                u.build_queue.clear();
                u.reset_nav();
            }
        }
    }

    pub fn order_attack(&mut self, hs: &[Handle], t: Target) {
        self.note_player_order(hs);
        for &h in hs {
            if let Some(u) = self.unit_mut(h) {
                u.order = Order::Attack(t);
                u.resume = None;
                u.build_queue.clear();
                u.reset_nav();
            }
        }
    }

    pub fn order_gather(&mut self, hs: &[Handle], src: Src) {
        self.note_player_order(hs);
        for &h in hs {
            if let Some(u) = self.unit_mut(h) {
                if u.kind == UnitKind::Worker {
                    u.order = Order::Gather { src, phase: 0 };
                    u.build_queue.clear();
                    u.reset_nav();
                }
            }
        }
    }

    pub fn order_build(&mut self, hs: &[Handle], b: Handle) {
        self.note_player_order(hs);
        for &h in hs {
            if let Some(u) = self.unit_mut(h) {
                if u.kind == UnitKind::Worker {
                    u.order = Order::Build(b);
                    u.build_queue.clear();
                    u.reset_nav();
                }
            }
        }
    }

    /// 建設を予約する：建設中の民は今の建設を終えてから順に取りかかり、それ以外の民はすぐに建て始める
    pub fn queue_build(&mut self, hs: &[Handle], b: Handle) {
        self.note_player_order(hs);
        for &h in hs {
            let busy = match self.unit(h) {
                Some(u) if u.kind == UnitKind::Worker => match u.order {
                    Order::Build(cur) => cur != b && self.bld(cur).map_or(false, |x| !x.done()),
                    _ => false,
                },
                _ => continue,
            };
            let u = self.unit_mut(h).unwrap();
            if busy {
                if !u.build_queue.contains(&b) {
                    u.build_queue.push_back(b);
                }
            } else {
                u.order = Order::Build(b);
                u.build_queue.clear();
                u.reset_nav();
            }
        }
    }

    /// 予約された次の建設に取りかかる（なければ false）
    fn next_queued_build(&mut self, i: usize) -> bool {
        let ni = self.units[i].nation as usize;
        while let Some(b) = self.units[i].build_queue.pop_front() {
            if self.bld(b).map_or(false, |x| x.nation as usize == ni && !x.done()) {
                let u = &mut self.units[i];
                u.order = Order::Build(b);
                u.reset_nav();
                return true;
            }
        }
        false
    }

    pub fn try_pay(&mut self, nation: usize, cost: [f32; 3]) -> bool {
        let r = &mut self.nations[nation].res;
        if r[0] >= cost[0] && r[1] >= cost[1] && r[2] >= cost[2] {
            for k in 0..3 {
                r[k] -= cost[k];
            }
            true
        } else {
            false
        }
    }

    pub fn place_building(&mut self, nation: usize, kind: BKind, tx: i32, ty: i32) -> Result<Handle, &'static str> {
        self.can_place(nation, kind, tx, ty)?;
        if !self.try_pay(nation, kind.cost()) {
            return Err(t!("資源が足りない", "Not enough resources", "资源不足"));
        }
        Ok(self.spawn_building(nation, kind, tx, ty, false))
    }

    pub fn can_train(&self, b: &Building, kind: UnitKind) -> bool {
        if !b.done() {
            return false;
        }
        match b.kind {
            BKind::Capital | BKind::Town => kind == UnitKind::Worker,
            BKind::Barracks => kind.military(),
            BKind::Farm | BKind::Fort => false,
        }
    }

    pub fn enqueue(&mut self, bh: Handle, kind: UnitKind) -> Result<(), &'static str> {
        let Some(b) = self.bld(bh) else { return Err("") };
        if !self.can_train(b, kind) {
            return Err(t!("ここでは訓練できない", "Cannot train that here", "此处无法训练"));
        }
        if b.queue.len() >= 5 {
            return Err(t!("訓練待ちが一杯", "Training queue is full", "训练队列已满"));
        }
        let ni = b.nation as usize;
        let n = &self.nations[ni];
        if kind.military() {
            if n.soldiers + n.q_soldiers >= n.mil_cap() {
                return Err(t!("軍事ユニットが上限に達している", "Soldier limit reached", "军事单位已达上限"));
            }
        } else if n.workers + n.q_workers >= n.worker_cap() {
            return Err(t!("生産ユニットが上限に達している", "Worker limit reached", "生产单位已达上限"));
        }
        if !self.try_pay(ni, self.train_cost(ni, kind)) {
            return Err(t!("資源が足りない", "Not enough resources", "资源不足"));
        }
        let b = self.bld_mut(bh).unwrap();
        b.queue.push_back(kind);
        let n = &mut self.nations[ni];
        if kind.military() {
            n.q_soldiers += 1;
        } else {
            n.q_workers += 1;
        }
        Ok(())
    }

    pub fn upgrade_cost(&self, u: &Unit) -> [f32; 3] {
        let p = u.power;
        let ni = u.nation as usize;
        let m = if ni < NATIONS.len() { self.nations[ni].tm.upgrade } else { 1.0 };
        let total = (40.0 * p * m).round();
        // 蓄えの多い資源から優先して払う（資源の偏りで余らせないため）。蓄えが乏しいときは兵は糧・銅、民は糧・木が基本
        let base = if u.kind.military() { [25.0, 0.0, 15.0] } else { [25.0, 15.0, 0.0] };
        let res = if ni < NATIONS.len() { self.nations[ni].res } else { [0.0; 3] };
        let have = res[0].max(0.0) + res[1].max(0.0) + res[2].max(0.0);
        let w: [f32; 3] = if have >= total && have > 0.0 { [res[0].max(0.0), res[1].max(0.0), res[2].max(0.0)] } else { base };
        let ws = w[0] + w[1] + w[2];
        let mut c = [0.0f32; 3];
        for k in 0..3 {
            c[k] = (total * w[k] / ws).floor();
        }
        // 端数は最も多い資源で払う
        let big = (0..3).max_by(|&a, &b| w[a].partial_cmp(&w[b]).unwrap()).unwrap();
        c[big] += total - (c[0] + c[1] + c[2]);
        c
    }

    /// 訓練（兵）・教化（民）：能力を高める
    pub fn upgrade_unit(&mut self, h: Handle) -> Result<(), &'static str> {
        let cap = self.power_cap();
        let Some(u) = self.unit(h) else { return Err("") };
        if u.merc_until > 0.0 {
            return Err(t!("傭兵は鍛錬できない", "Mercenaries cannot be trained", "佣兵无法训练"));
        }
        if u.hero != NONE {
            return Err(t!("英霊は鍛錬できない", "Heroic spirits cannot be trained", "英灵无法训练"));
        }
        if u.power + 0.01 >= cap {
            return Err(t!("この時代の能力上限に達している", "Already at this era's power limit", "已达本时代能力上限"));
        }
        let (x, y) = u.pos.tile();
        if self.owner[Map::idx(x, y)] != u.nation {
            return Err(t!("自国領内でのみ鍛錬できる", "Can only train inside your territory", "只能在己方领土内训练"));
        }
        let ni = u.nation as usize;
        let cost = self.upgrade_cost(u);
        if !self.try_pay(ni, cost) {
            return Err(t!("資源が足りない", "Not enough resources", "资源不足"));
        }
        let u = self.unit_mut(h).unwrap();
        let ratio = u.hp / u.max_hp();
        u.power = (u.power + 0.5).min(cap);
        u.hp = ratio * u.max_hp();
        Ok(())
    }

    /// 兵⇔民の転換（能力は 2/3 に目減り）
    pub fn convert_unit(&mut self, h: Handle) -> Result<(), &'static str> {
        let Some(u) = self.unit(h) else { return Err("") };
        if u.merc_until > 0.0 {
            return Err(t!("傭兵は民にできない", "Mercenaries cannot become workers", "佣兵无法转为民"));
        }
        if u.hero != NONE {
            return Err(t!("英霊は民にできない", "Heroic spirits cannot become workers", "英灵无法转为民"));
        }
        let ni = u.nation as usize;
        let n = &self.nations[ni];
        let to_mil = !u.kind.military();
        if to_mil && n.soldiers + n.q_soldiers >= n.mil_cap() {
            return Err(t!("軍事ユニットが上限に達している", "Soldier limit reached", "军事单位已达上限"));
        }
        if !to_mil && n.workers + n.q_workers >= n.worker_cap() {
            return Err(t!("生産ユニットが上限に達している", "Worker limit reached", "生产单位已达上限"));
        }
        let u = self.unit_mut(h).unwrap();
        let ratio = u.hp / u.max_hp();
        u.power = (u.power * 2.0 / 3.0).max(0.34);
        u.kind = if to_mil { UnitKind::Infantry } else { UnitKind::Worker };
        u.hp = ratio * u.max_hp();
        u.order = Order::Idle;
        u.carry = 0.0;
        let n = &mut self.nations[ni];
        if to_mil {
            n.soldiers += 1;
            n.workers -= 1;
        } else {
            n.soldiers -= 1;
            n.workers += 1;
        }
        Ok(())
    }

    // ------------------------------------------------------------ 検索
    pub fn rebuild_grid(&mut self) {
        for c in self.grid.iter_mut() {
            c.clear();
        }
        for (i, u) in self.units.iter().enumerate() {
            if u.alive {
                let gx = (u.pos.x as i32 / GCELL).clamp(0, GW - 1);
                let gy = (u.pos.y as i32 / GCELL).clamp(0, GH - 1);
                self.grid[(gy * GW + gx) as usize].push(i as u32);
            }
        }
    }

    /// 敵対ユニット探索
    pub fn nearest_enemy_unit(&self, nation: usize, p: V2, r: f32) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        let g0x = ((p.x - r) as i32 / GCELL).clamp(0, GW - 1);
        let g1x = ((p.x + r) as i32 / GCELL).clamp(0, GW - 1);
        let g0y = ((p.y - r) as i32 / GCELL).clamp(0, GH - 1);
        let g1y = ((p.y + r) as i32 / GCELL).clamp(0, GH - 1);
        for gy in g0y..=g1y {
            for gx in g0x..=g1x {
                for &i in self.grid[(gy * GW + gx) as usize].iter() {
                    let u = &self.units[i as usize];
                    if !u.alive || !self.at_war(nation, u.nation as usize) {
                        continue;
                    }
                    let d = u.pos.dist(p);
                    // 兵を優先
                    let d2 = if u.kind.military() { d } else { d + 2.0 };
                    if d <= r && best.map_or(true, |b| d2 < b.0) {
                        best = Some((d2, i as usize));
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    pub fn nearest_enemy_bld(&self, nation: usize, p: V2, r: f32) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (i, b) in self.blds.iter().enumerate() {
            if !b.alive || !self.at_war(nation, b.nation as usize) {
                continue;
            }
            let d = b.center().dist(p);
            if d <= r && best.map_or(true, |x| d < x.0) {
                best = Some((d, i));
            }
        }
        best.map(|b| b.1)
    }

    /// 国ごとの完成した城邑の一覧を作り直す
    fn rebuild_city_list(&mut self) {
        let n = self.nations.len();
        if self.city_list.len() != n {
            self.city_list = vec![Vec::new(); n];
        }
        for l in self.city_list.iter_mut() {
            l.clear();
        }
        for (i, b) in self.blds.iter().enumerate() {
            if b.alive && b.kind.city() && b.done() {
                if let Some(l) = self.city_list.get_mut(b.nation as usize) {
                    l.push(i as u32);
                }
            }
        }
    }

    pub fn nearest_dropsite(&self, nation: usize, p: V2) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        // 一覧は毎ステップ作り直すが、途中で落ちた城邑は確かめて除く
        let list: Vec<usize> = match self.city_list.get(nation) {
            Some(l) => l.iter().map(|&i| i as usize).collect(),
            None => (0..self.blds.len()).collect(),
        };
        for i in list {
            let b = &self.blds[i];
            if b.alive && b.nation as usize == nation && b.kind.city() && b.done() {
                let d = b.center().dist(p);
                if best.map_or(true, |x| d < x.0) {
                    best = Some((d, i));
                }
            }
        }
        best.map(|b| b.1)
    }

    pub fn nearest_forest_for(&self, nation: Option<usize>, p: V2, maxr: i32) -> Option<(i32, i32)> {
        let (cx, cy) = p.tile();
        let mut best: Option<(i32, (i32, i32))> = None;
        for r in 0..=maxr {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (x, y) = (cx + dx, cy + dy);
                    if Map::inb(x, y) {
                        let idx = Map::idx(x, y);
                        if self.map.terrain[idx] == Terrain::Forest {
                            if let Some(n) = nation {
                                let o = self.owner[idx];
                                if o != NONE && o as usize != n {
                                    continue;
                                }
                            }
                            let d = dx * dx + dy * dy;
                            if best.map_or(true, |b| d < b.0) {
                                best = Some((d, (x, y)));
                            }
                        }
                    }
                }
            }
            if best.is_some() {
                return best.map(|b| b.1);
            }
        }
        None
    }

    pub fn nearest_forest(&self, p: V2, maxr: i32) -> Option<(i32, i32)> {
        self.nearest_forest_for(None, p, maxr)
    }

    pub fn nearest_mine_for(&self, nation: Option<usize>, p: V2, maxd: f32) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (i, m) in self.map.mines.iter().enumerate() {
            if m.amount <= 0.0 {
                continue;
            }
            if let Some(n) = nation {
                if Map::inb(m.x, m.y) {
                    let o = self.owner[Map::idx(m.x, m.y)];
                    if o != NONE && o as usize != n {
                        continue;
                    }
                }
            }
            let d = v2(m.x as f32 + 0.5, m.y as f32 + 0.5).dist(p);
            if d <= maxd && best.map_or(true, |b| d < b.0) {
                best = Some((d, i));
            }
        }
        best.map(|b| b.1)
    }

    pub fn nearest_mine(&self, p: V2, maxd: f32) -> Option<usize> {
        self.nearest_mine_for(None, p, maxd)
    }

    pub fn target_info(&self, t: Target) -> Option<(V2, f32, usize)> {
        match t {
            Target::Unit(h) => self.unit(h).map(|u| (u.pos, 0.3, u.nation as usize)),
            Target::Bld(h) => self.bld(h).map(|b| (b.center(), 1.0, b.nation as usize)),
        }
    }

    // ------------------------------------------------------------ 戦闘
    fn damage_unit(&mut self, ti: usize, dmg: f32, attacker: Option<Handle>, an: usize) {
        let killed;
        // 上帝の兵を傷つけられるのは英霊だけ
        let by_hero = attacker.and_then(|h| self.unit(h)).map_or(false, |a| a.hero != NONE);
        {
            let u = &mut self.units[ti];
            if !u.alive || (u.divine && !by_hero) {
                return;
            }
            // 英霊の一撃は上帝の兵によく効く
            u.hp -= if u.divine { dmg * 3.0 } else { dmg };
            u.flash = 0.15;
            u.last_attacker = attacker;
            killed = u.hp <= 0.0;
            if !killed && u.kind.military() && u.order == Order::Idle {
                if let Some(a) = attacker {
                    u.order = Order::Attack(Target::Unit(a));
                }
            }
        }
        if killed {
            let vn = self.units[ti].nation as usize;
            self.kill_unit(ti);
            if an != NONE as usize {
                self.nations[an].aggression += 1.5;
            }
            let _ = vn;
        }
    }

    fn damage_bld(&mut self, bi: usize, dmg: f32, an: usize) {
        let vn = self.blds[bi].nation as usize;
        let dmg = if vn < NATIONS.len() { dmg * self.nations[vn].tm.def } else { dmg };
        let b = &mut self.blds[bi];
        if !b.alive {
            return;
        }
        b.hp -= dmg;
        if b.hp > 0.0 {
            return;
        }
        if b.kind.city() && b.done() && an == TRIBE {
            self.plunder_city(bi);
        } else if b.kind.city() && b.done() {
            self.capture_city(bi, an);
        } else {
            let (vn, kind) = (b.nation as usize, b.kind);
            self.remove_building(bi);
            if vn == self.player {
                self.push_msg(tf!("{}が破壊された", "Our {} was destroyed", "{}被摧毁了", kind.name()), [1.0, 0.5, 0.4]);
            }
        }
    }

    fn capture_city(&mut self, bi: usize, an: usize) {
        let old = self.blds[bi].nation as usize;
        // 周の天子は春秋時代には滅びない：最後の城邑を落とされそうになると攻め手は兵を引く
        let last = !self.blds.iter().enumerate().any(|(i, b)| i != bi && b.alive && b.nation as usize == old && b.kind.city() && b.done());
        if last && self.immortal(old) {
            self.blds[bi].hp = self.blds[bi].kind.max_hp() * 0.5;
            self.nations[an].aggression += 20.0;
            if self.war[an][old] {
                self.war[an][old] = false;
                self.war[old][an] = false;
                self.truce[an][old] = self.years + 15.0;
                self.truce[old][an] = self.years + 15.0;
            }
            for i in 0..self.units.len() {
                if self.units[i].alive && self.units[i].nation as usize == an {
                    if let Order::Attack(Target::Bld(h)) = self.units[i].order {
                        if h.idx as usize == bi {
                            self.units[i].order = Order::Idle;
                        }
                    }
                }
            }
            let t = tf!("{} は天子の都を前に兵を引いた（春秋時代、周は滅びない）", "{} withdrew before the capital of the Son of Heaven (Zhou cannot fall in the Spring and Autumn period)", "{}在天子都城前撤兵（春秋时代周不会灭亡）", tx(self.nations[an].name));
            self.push_msg(t, [1.0, 0.9, 0.5]);
            return;
        }
        let was_cap = self.blds[bi].kind == BKind::Capital;
        {
            let b = &mut self.blds[bi];
            b.nation = an as u8;
            b.hp = b.kind.max_hp() * 0.3;
            b.queue.clear();
            b.queue_t = 0.0;
            b.rally = None;
            b.kind = BKind::Town;
        }
        let bh = self.bh(bi);
        if self.nations[an].capital.and_then(|h| self.bld(h)).is_none() {
            self.blds[bi].kind = BKind::Capital;
            self.nations[an].capital = Some(bh);
        }
        self.territory_dirty = true;
        self.nations[an].aggression += 25.0;
        let p = self.player;
        let name = if was_cap { t!("都", "capital", "都城") } else { t!("邑", "town", "邑") };
        let text = tf!("{} が {} の{}を攻略した", "{} took {}'s {}", "{}攻占了{}的{}", tx(self.nations[an].name), tx(self.nations[old].name), name);
        let col = if an == p { [0.5, 1.0, 0.5] } else if old == p { [1.0, 0.35, 0.3] } else { [0.85, 0.85, 0.7] };
        if an == p || old == p || was_cap {
            self.push_msg(text, col);
        }
        if was_cap {
            // 遷都
            let mut newcap = None;
            for (i, b) in self.blds.iter().enumerate() {
                if b.alive && b.nation as usize == old && b.kind == BKind::Town && b.done() {
                    newcap = Some(i);
                    break;
                }
            }
            match newcap {
                Some(i) => {
                    self.blds[i].kind = BKind::Capital;
                    let h = self.bh(i);
                    self.nations[old].capital = Some(h);
                    self.nations[old].home = self.blds[i].center();
                }
                None => self.nations[old].capital = None,
            }
        }
        let has_city = self.blds.iter().any(|b| b.alive && b.nation as usize == old && b.kind.city() && b.done());
        if !has_city {
            self.eliminate(old, an);
        }
    }

    pub fn eliminate(&mut self, n: usize, by: usize) {
        if !self.nations[n].alive {
            return;
        }
        self.nations[n].alive = false;
        for i in 0..self.units.len() {
            if self.units[i].alive && self.units[i].nation as usize == n {
                self.kill_unit(i);
            }
        }
        for i in 0..self.blds.len() {
            if self.blds[i].alive && self.blds[i].nation as usize == n {
                self.remove_building(i);
            }
        }
        for o in 0..NATIONS.len() {
            self.war[n][o] = false;
            self.war[o][n] = false;
        }
        self.share_cap(n, by);
        let by_name = if by < self.nations.len() { tx(self.nations[by].name) } else { "?" };
        let text = tf!("{} は {} に滅ぼされた", "{} was destroyed by {}", "{}被{}所灭", tx(self.nations[n].name), by_name);
        self.push_msg(text, [1.0, 0.8, 0.3]);
        if n == self.player {
            let t = if self.descent.is_some() {
                tf!("{} は上帝の軍に滅ぼされた……", "{} was destroyed by the army of the Supreme Deity...", "{}被上帝之军所灭……", tx(self.nations[n].name))
            } else {
                tf!("{} は {} に滅ぼされた……", "{} was destroyed by {}...", "{}被{}所灭……", tx(self.nations[n].name), by_name)
            };
            self.game_over = Some((false, t));
        }
    }

    /// 滅んだ国の人口上限（基本の4と、それまでに分けられた分）を生き残った国に分ける。
    /// 等分し、余りは攻め滅ぼした国、次いで国力の高い国から順に1ずつ
    fn share_cap(&mut self, n: usize, by: usize) {
        let pts = BASE_CAP + self.nations[n].cap_bonus;
        self.nations[n].cap_bonus = 0;
        let mut order: Vec<usize> = (0..NATIONS.len()).filter(|&i| i != n && self.nations[i].alive).collect();
        if order.is_empty() {
            return;
        }
        order.sort_by(|&a, &b| self.nations[b].strength.partial_cmp(&self.nations[a].strength).unwrap());
        if let Some(k) = order.iter().position(|&i| i == by) {
            let c = order.remove(k);
            order.insert(0, c);
        }
        let (each, rest) = (pts / order.len() as u32, pts as usize % order.len());
        let mut mine = 0;
        for (k, &i) in order.iter().enumerate() {
            let add = each + if k < rest { 1 } else { 0 };
            self.nations[i].cap_bonus += add;
            if i == self.player {
                mine = add;
            }
        }
        if mine > 0 {
            let p = self.player;
            let cap = self.nations[p].worker_cap();
            let t = tf!("{}の滅亡で、民・兵の上限がそれぞれ +{}（いま{}）", "With the fall of {}, our worker and soldier limits each rise by {} (now {})", "因{}灭亡，民与兵的上限各+{}（现为{}）", tx(self.nations[n].name), mine, cap);
            self.push_msg(t, [0.7, 1.0, 0.8]);
        }
    }

    // ------------------------------------------------------------ 更新
    pub fn step(&mut self) {
        let dt = DT;
        self.time += dt;
        let spy = self.sec_per_year();
        self.years += dt / spy;
        if self.game_over.is_some() {
            return;
        }
        if self.era == Era::Spring && START_BC - self.years <= WARRING_BC {
            self.begin_warring();
        }
        self.stat_t -= dt;
        if self.stat_t <= 0.0 {
            self.stat_t = 0.25;
            self.update_stats();
        }
        self.economy(dt);
        self.update_trade(dt);
        self.update_buildings(dt);
        self.rebuild_city_list();
        self.rebuild_grid();
        for i in 0..self.units.len() {
            if self.units[i].alive {
                self.update_unit(i, dt);
            }
        }
        // 押し合いは1ステップおきに、2ステップ分の強さで（見た目は変わらず計算は半分）
        self.sep_tick = !self.sep_tick;
        if self.sep_tick {
            self.separate(dt * 2.0);
        }
        self.process_paths();
        for s in self.shots.iter_mut() {
            s.t -= dt;
        }
        self.shots.retain(|s| s.t > 0.0);
        if self.territory_dirty {
            self.recompute_territory();
        }
        if let Some(b) = &mut self.banner {
            b.t -= dt;
            if b.t <= 0.0 {
                self.banner = self.banner_queue.pop_front();
            }
        }
        self.year_tick += dt;
        if self.year_tick >= spy {
            self.year_tick -= spy;
            self.yearly();
        }
        crate::ai::update_all(self, dt);
    }

    fn economy(&mut self, dt: f32) {
        let em = self.era_mult();
        // 城邑からの税収
        for b in self.blds.iter() {
            if b.alive && b.kind.city() && b.done() {
                let n = &mut self.nations[b.nation as usize];
                let m = n.bias * n.eco_mul * n.tm.eco * em * if b.kind == BKind::Capital { 1.3 } else { 1.0 };
                let add = [0.55 * m * dt, 0.25 * m * dt, 0.12 * m * dt];
                for k in 0..3 {
                    n.res[k] += add[k];
                    n.earned[k] += add[k];
                }
            }
        }
        // 軍の維持費（存在するだけで糧を消費）
        let spy = self.sec_per_year();
        let mut starving = vec![false; self.nations.len()];
        for (i, n) in self.nations.iter_mut().enumerate() {
            if !n.alive {
                continue;
            }
            n.res[0] -= n.upkeep * dt;
            if n.res[0] < 0.0 {
                n.res[0] = 0.0;
                starving[i] = true;
            }
            n.aggression = (n.aggression - dt * 2.0 / spy).max(0.0);
        }
        for u in self.units.iter_mut() {
            if u.alive && u.hero != NONE {
                // 英霊は傷が癒える
                u.hp = (u.hp + u.max_hp() * 0.015 * dt).min(u.max_hp());
            } else if u.alive && u.kind.military() && !u.divine && starving[u.nation as usize] {
                u.hp -= u.max_hp() * 0.01 * dt;
            }
        }
        for i in 0..self.units.len() {
            if self.units[i].alive && self.units[i].hp <= 0.0 {
                self.kill_unit(i);
            }
        }
        self.income_t += dt;
        if self.income_t >= 5.0 {
            for n in self.nations.iter_mut() {
                for k in 0..3 {
                    let r = n.earned[k] / self.income_t;
                    n.income[k] = n.income[k] * 0.5 + r * 0.5;
                    n.earned[k] = 0.0;
                }
                let r = n.coin_earned / self.income_t;
                n.coin_income = n.coin_income * 0.5 + r * 0.5;
                n.coin_earned = 0.0;
            }
            self.income_t = 0.0;
        }
    }

    fn update_buildings(&mut self, dt: f32) {
        for bi in 0..self.blds.len() {
            if !self.blds[bi].alive || !self.blds[bi].done() {
                continue;
            }
            // 訓練
            if let Some(&k) = self.blds[bi].queue.front() {
                self.blds[bi].queue_t += dt;
                if self.blds[bi].queue_t >= k.train_time() {
                    let h = self.bh(bi);
                    if let Some(sp) = self.spawn_point(h) {
                        let b = &mut self.blds[bi];
                        b.queue.pop_front();
                        b.queue_t = 0.0;
                        let (ni, rally) = (b.nation as usize, b.rally);
                        let uh = self.spawn_unit(ni, k, sp, 1.0);
                        if let Some(r) = rally {
                            self.order_move(&[uh], r, true);
                        } else if k == UnitKind::Worker {
                            self.auto_gather(uh);
                        }
                        if k.military() {
                            self.nations[ni].soldiers += 1;
                            self.nations[ni].q_soldiers = self.nations[ni].q_soldiers.saturating_sub(1);
                        } else {
                            self.nations[ni].workers += 1;
                            self.nations[ni].q_workers = self.nations[ni].q_workers.saturating_sub(1);
                        }
                    }
                }
            }
            // 城邑・要塞の防衛射撃
            let fort = self.blds[bi].kind == BKind::Fort;
            if self.blds[bi].kind.city() || fort {
                let b = &mut self.blds[bi];
                b.cooldown -= dt;
                if b.cooldown <= 0.0 {
                    let (c, ni) = (b.center(), b.nation as usize);
                    let (range, cd, dmg) = match (fort, self.era == Era::Spring) {
                        (true, true) => (FORT_RANGE, FORT_COOLDOWN, 14.0),
                        (true, false) => (FORT_RANGE, FORT_COOLDOWN, 22.0),
                        (false, true) => (6.0, 2.0, 10.0),
                        (false, false) => (6.0, 2.0, 16.0),
                    };
                    b.cooldown = cd;
                    if let Some(t) = self.nearest_enemy_unit(ni, c, range) {
                        let tp = self.units[t].pos;
                        self.shots.push(Shot { a: c, b: tp, t: 0.25 });
                        self.damage_unit(t, dmg, None, ni);
                    }
                }
            }
        }
    }

    /// 新しい民に仕事を割り当てる
    pub fn auto_gather(&mut self, h: Handle) {
        let Some(u) = self.unit(h) else { return };
        let ni = u.nation as usize;
        let p = u.pos;
        // 未完成で作業者のいない建物があれば最優先で建設に向かう
        if let Some(bh) = self.unfinished_bld(ni, p) {
            self.order_build(&[h], bh);
            return;
        }
        // 空いている田
        if let Some(f) = self.free_farm(ni, p) {
            self.order_gather(&[h], Src::Farm(f));
            return;
        }
        let res = self.nations[ni].res;
        if res[2] < res[1] * 0.6 {
            if let Some(m) = self.nearest_mine_for(Some(ni), p, 16.0) {
                self.order_gather(&[h], Src::Mine(m));
                return;
            }
        }
        if let Some((x, y)) = self.nearest_forest_for(Some(ni), p, 14) {
            self.order_gather(&[h], Src::Forest(x, y));
        }
    }

    pub fn bld_builders(&self, bh: Handle) -> usize {
        self.units.iter().filter(|u| u.alive && u.order == Order::Build(bh)).count()
    }

    pub fn unfinished_bld(&self, ni: usize, p: V2) -> Option<Handle> {
        let mut best: Option<(f32, Handle)> = None;
        for (i, b) in self.blds.iter().enumerate() {
            if b.alive && b.nation as usize == ni && !b.done() {
                let h = self.bh(i);
                if self.bld_builders(h) == 0 {
                    let d = b.center().dist(p);
                    if best.map_or(true, |x| d < x.0) {
                        best = Some((d, h));
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    /// 建物へ接近する際の通行可能な目標地点を返す（2×2の通行不可タイル内部ではなく隣接する通行可能タイル上の点）
    pub fn bld_approach_point(&self, bh: Handle, from: V2) -> V2 {
        let Some(b) = self.bld(bh) else { return from };
        let c = b.center();
        if !b.kind.blocks() && self.passable_at(c) {
            return c;
        }
        let mut best: Option<(f32, V2)> = None;
        for dy in -1..=2 {
            for dx in -1..=2 {
                if dx >= 0 && dx <= 1 && dy >= 0 && dy <= 1 {
                    continue;
                }
                let (x, y) = (b.tx + dx, b.ty + dy);
                if !Map::inb(x, y) || !self.cost[Map::idx(x, y)].is_finite() {
                    continue;
                }
                let tc = v2(x as f32 + 0.5, y as f32 + 0.5);
                let bx = tc.x.clamp(b.tx as f32, b.tx as f32 + 2.0);
                let by = tc.y.clamp(b.ty as f32, b.ty as f32 + 2.0);
                let ap = tc + (v2(bx, by) - tc) * 0.35;
                let corner_pen = if (dx == -1 || dx == 2) && (dy == -1 || dy == 2) { 0.45 } else { 0.0 };
                let score = from.dist(ap) + corner_pen;
                if best.map_or(true, |bs| score < bs.0) {
                    best = Some((score, ap));
                }
            }
        }
        best.map(|bs| bs.1).unwrap_or(c)
    }

    pub fn farm_users(&self, f: Handle) -> usize {
        self.units.iter().filter(|u| u.alive && matches!(u.order, Order::Gather { src: Src::Farm(h), .. } if h == f)).count()
    }

    pub fn free_farm(&self, ni: usize, p: V2) -> Option<Handle> {
        // 田ごとに全ユニットを数えると田の数×ユニット数になるので、先に使われている田を集める
        let mut used = vec![false; self.blds.len()];
        for u in self.units.iter() {
            if let (true, Order::Gather { src: Src::Farm(h), .. }) = (u.alive, u.order) {
                if let Some(x) = used.get_mut(h.idx as usize) {
                    if self.blds[h.idx as usize].gen == h.gen {
                        *x = true;
                    }
                }
            }
        }
        let mut best: Option<(f32, Handle)> = None;
        for (i, b) in self.blds.iter().enumerate() {
            if b.alive && b.nation as usize == ni && b.kind == BKind::Farm && b.done() {
                let h = self.bh(i);
                if !used[i] {
                    let d = b.center().dist(p);
                    if best.map_or(true, |x| d < x.0) {
                        best = Some((d, h));
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    // ---- 移動
    fn request_path(&mut self, i: usize, goal: (i32, i32)) {
        let u = &mut self.units[i];
        u.path_goal = goal;
        u.path_done = false;
        if !u.path_pending {
            u.path_pending = true;
            let h = Handle { idx: i as u32, gen: u.gen };
            self.path_queue.push_back(h);
        }
    }

    fn process_paths(&mut self) {
        // 1ステップで展開してよいマスの数。実時間で区切ると機械の速さで結果が変わるので、仕事量で区切る
        const BUDGET: usize = 30_000;
        let mut n = 0;
        let mut spent = 0;
        // 同じステップで、隣り合う地点から同じ目的地へ求めた経路は使い回す（部隊でまとめて動かすとき）
        let mut memo: Vec<((i32, i32), (i32, i32), Vec<(i16, i16)>)> = Vec::new();
        while let Some(h) = self.path_queue.pop_front() {
            let i = h.idx as usize;
            if !(self.units[i].alive && self.units[i].gen == h.gen && self.units[i].path_pending) {
                continue;
            }
            let start = self.units[i].pos.tile();
            let goal = self.units[i].path_goal;
            let pos = self.units[i].pos;
            let reuse = memo.iter().find(|(s, g, p)| {
                *g == goal && !p.is_empty() && (s.0 - start.0).abs().max((s.1 - start.1).abs()) <= 1 && self.line_clear(pos, v2(s.0 as f32 + 0.5, s.1 as f32 + 0.5))
            });
            let path = if let Some((_, _, p)) = reuse {
                p.clone()
            } else {
                let r = if Map::inb(goal.0, goal.1) && self.cost[Map::idx(goal.0, goal.1)].is_finite() { 0 } else { 2 };
                let p = self.pf.find(&self.cost, start, goal, r, 160000, crate::path::UNIT_WEIGHT);
                spent += self.pf.last_exp;
                memo.push((start, goal, p.clone()));
                p
            };
            let u = &mut self.units[i];
            u.path = path;
            u.path_i = 0;
            u.path_pending = false;
            u.path_done = u.path.is_empty();
            n += 1;
            if n >= 4 && spent > BUDGET {
                break;
            }
        }
    }

    fn step_towards(&mut self, i: usize, to: V2, dt: f32) {
        let u = &self.units[i];
        let sp = u.kind.speed(self.era);
        let (tx, ty) = u.pos.tile();
        let tc = if Map::inb(tx, ty) { self.map.terrain[Map::idx(tx, ty)].move_cost() } else { 1.0 };
        let sp = sp / tc.clamp(1.0, 3.0).sqrt();
        let d = to - u.pos;
        let l = d.len();
        if l < 1e-4 {
            return;
        }
        let mv = (sp * dt).min(l);
        let np = u.pos + d * (mv / l);
        let dir = d * (1.0 / l);
        let ok = self.passable_at(np) || !self.passable_at(u.pos);
        let u = &mut self.units[i];
        u.dir = dir;
        if ok {
            u.pos = np;
        } else {
            let a = v2(np.x, u.pos.y);
            let b = v2(u.pos.x, np.y);
            if self.passable_at(a) {
                self.units[i].pos = a;
            } else if self.passable_at(b) {
                self.units[i].pos = b;
            }
        }
    }

    /// 目的地へ移動。到着（または到達不能）なら true
    fn nav(&mut self, i: usize, goal: V2, r: f32, dt: f32) -> bool {
        let pos = self.units[i].pos;
        let dist = pos.dist(goal);
        if dist <= r {
            self.units[i].path.clear();
            return true;
        }
        if dist < 4.0 && self.line_clear(pos, goal) {
            self.units[i].path.clear();
            self.step_towards(i, goal, dt);
            return false;
        }
        let gt = goal.tile();
        let pg = self.units[i].path_goal;
        let same = if self.units[i].path_done {
            pg == gt
        } else {
            (pg.0 - gt.0).abs().max((pg.1 - gt.1).abs()) <= 1
        };
        if !same {
            self.units[i].path.clear();
            self.request_path(i, gt);
            return false;
        }
        let u = &self.units[i];
        if u.path_i < u.path.len() {
            let (wx, wy) = u.path[u.path_i];
            let wp = v2(wx as f32 + 0.5, wy as f32 + 0.5);
            if pos.dist(wp) < 0.45 {
                let u = &mut self.units[i];
                u.path_i += 1;
                if u.path_i >= u.path.len() {
                    u.path.clear();
                    u.path_i = 0;
                    u.path_done = true;
                }
            } else {
                // 先読み：次の点が直接見えるなら飛ばす
                let ni = u.path_i + 1;
                if ni < u.path.len() {
                    let (nx, ny) = u.path[ni];
                    let np = v2(nx as f32 + 0.5, ny as f32 + 0.5);
                    if pos.dist(wp) < 1.2 && self.line_clear(pos, np) {
                        self.units[i].path_i = ni;
                    }
                }
                self.step_towards(i, wp, dt);
            }
            return false;
        }
        if u.path_pending {
            return false;
        }
        if u.path_done {
            // 経路の終端に着いた：直進できなければ到達不能
            if dist < 6.0 && self.line_clear(pos, goal) {
                self.step_towards(i, goal, dt);
                return false;
            }
            return true;
        }
        self.request_path(i, gt);
        false
    }

    fn update_unit(&mut self, i: usize, dt: f32) {
        {
            let u = &mut self.units[i];
            u.cooldown -= dt;
            u.flash = (u.flash - dt).max(0.0);
            u.scan_t -= dt;
        }
        let order = self.units[i].order;
        let ni = self.units[i].nation as usize;
        let kind = self.units[i].kind;
        match order {
            Order::Idle => {
                if kind.military() && self.units[i].scan_t <= 0.0 {
                    self.units[i].scan_t = 0.5;
                    if let Some(t) = self.nearest_enemy_unit(ni, self.units[i].pos, kind.sight()) {
                        let th = self.uh(t);
                        self.units[i].order = Order::Attack(Target::Unit(th));
                    }
                }
                // 自国領内で待機中は回復
                let (x, y) = self.units[i].pos.tile();
                if Map::inb(x, y) && self.owner[Map::idx(x, y)] as usize == ni {
                    let u = &mut self.units[i];
                    u.hp = (u.hp + u.max_hp() * 0.01 * dt).min(u.max_hp());
                }
            }
            Order::Move(g) => {
                if self.nav(i, g, 0.25, dt) {
                    self.units[i].order = Order::Idle;
                }
            }
            Order::AttackMove(g) => {
                if self.units[i].scan_t <= 0.0 {
                    self.units[i].scan_t = 0.4;
                    let p = self.units[i].pos;
                    let found = self
                        .nearest_enemy_unit(ni, p, kind.sight())
                        .map(|t| Target::Unit(self.uh(t)))
                        .or_else(|| self.nearest_enemy_bld(ni, p, kind.sight()).map(|b| Target::Bld(self.bh(b))));
                    if let Some(t) = found {
                        self.units[i].order = Order::Attack(t);
                        self.units[i].resume = Some(g);
                        return;
                    }
                }
                if self.nav(i, g, 0.6, dt) {
                    self.units[i].order = Order::Idle;
                }
            }
            Order::Attack(t) => {
                let info = self.target_info(t);
                let valid = match info {
                    Some((_, _, tn)) => self.at_war(ni, tn),
                    None => false,
                };
                if !valid {
                    let r = self.units[i].resume.take();
                    self.units[i].order = match r {
                        Some(g) => Order::AttackMove(g),
                        None => Order::Idle,
                    };
                    self.units[i].scan_t = 0.0;
                    return;
                }
                let (tp, rad, _) = info.unwrap();
                let pos = self.units[i].pos;
                let range = kind.range() + rad;
                let d = pos.dist(tp);
                // 進軍中なら深追いしない
                if let (Some(_), Target::Unit(_)) = (self.units[i].resume, t) {
                    if d > kind.sight() * 1.8 {
                        let g = self.units[i].resume.take().unwrap();
                        self.units[i].order = Order::AttackMove(g);
                        return;
                    }
                }
                // 攻撃中も近くに敵兵がいれば建物より優先
                if let Target::Bld(_) = t {
                    if self.units[i].scan_t <= 0.0 {
                        self.units[i].scan_t = 0.6;
                        if let Some(e) = self.nearest_enemy_unit(ni, pos, kind.sight() * 0.8) {
                            if self.units[e].kind.military() {
                                if self.units[i].resume.is_none() {
                                    self.units[i].resume = Some(tp);
                                }
                                self.units[i].order = Order::Attack(Target::Unit(self.uh(e)));
                                return;
                            }
                        }
                    }
                }
                let in_range = match t {
                    Target::Bld(bh) => d <= range || self.bld(bh).map_or(false, |b| b.dist_to_footprint(pos) <= kind.range() + 0.35),
                    Target::Unit(_) => d <= range,
                };
                if in_range {
                    self.units[i].dir = (tp - pos).norm();
                    if self.units[i].cooldown <= 0.0 {
                        self.units[i].cooldown = kind.cooldown();
                        let atk = kind.base_atk() * self.units[i].power * self.nations[ni].mil_mul * self.nations[ni].tm.atk[kind as usize] * self.rng.range(0.85, 1.15);
                        let me = self.uh(i);
                        if kind == UnitKind::Archer {
                            self.shots.push(Shot { a: pos, b: tp, t: 0.3 });
                        }
                        match t {
                            Target::Unit(h) => {
                                let ti = h.idx as usize;
                                let tk = self.units[ti].kind;
                                let (tx, ty) = self.units[ti].pos.tile();
                                let hill = Map::inb(tx, ty) && self.map.terrain[Map::idx(tx, ty)] == Terrain::Hill;
                                let dmg = atk * kind.bonus(tk) * if hill { 0.8 } else { 1.0 };
                                self.damage_unit(ti, dmg, Some(me), ni);
                            }
                            Target::Bld(h) => {
                                let dmg = atk * kind.vs_building() * 0.8;
                                self.damage_bld(h.idx as usize, dmg, ni);
                            }
                        }
                    }
                } else {
                    let (nav_goal, nav_r) = match t {
                        Target::Bld(bh) if kind.range() < 1.5 => (self.bld_approach_point(bh, pos), 0.45),
                        _ => (tp, range * 0.9),
                    };
                    if self.nav(i, nav_goal, nav_r, dt) {
                        let unreachable = match t {
                            Target::Bld(bh) => d > range + 0.5 && self.bld(bh).map_or(true, |b| b.dist_to_footprint(self.units[i].pos) > kind.range() + 0.65),
                            Target::Unit(_) => d > range + 0.5,
                        };
                        if unreachable {
                            self.units[i].order = Order::Idle;
                        }
                    }
                }
            }
            Order::Gather { src, phase } => self.update_gather(i, src, phase, dt),
            Order::Build(bh) => {
                let Some(b) = self.bld(bh) else {
                    if !self.next_queued_build(i) {
                        self.units[i].order = Order::Idle;
                    }
                    return;
                };
                if b.done() || b.nation as usize != ni {
                    let (kind_b, own) = (b.kind, b.nation as usize == ni);
                    // 予約があれば次の建設へ
                    if self.next_queued_build(i) {
                        return;
                    }
                    if kind_b == BKind::Farm && own && self.farm_users(bh) == 0 {
                        self.units[i].order = Order::Gather { src: Src::Farm(bh), phase: 0 };
                    } else {
                        self.units[i].order = Order::Idle;
                    }
                    return;
                }
                let c = b.center();
                let pos = self.units[i].pos;
                let near = if b.kind.blocks() {
                    b.dist_to_footprint(pos) <= 1.05
                } else {
                    pos.dist(c) <= 1.15 || b.dist_to_footprint(pos) <= 0.45
                };
                if near {
                    self.units[i].path.clear();
                    let d = (c - pos).norm();
                    if d.len() > 0.0 {
                        self.units[i].dir = d;
                    }
                    let pw = self.units[i].power;
                    let b = self.bld_mut(bh).unwrap();
                    let inc = dt * pw / b.kind.build_time();
                    b.progress += inc;
                    b.hp = (b.hp + b.kind.max_hp() * 0.9 * inc).min(b.kind.max_hp());
                    if b.progress >= 1.0 {
                        b.progress = 1.0;
                        if b.kind.city() {
                            self.territory_dirty = true;
                            if self.nations[ni].capital.and_then(|h| self.bld(h)).is_none() {
                                let b = self.bld_mut(bh).unwrap();
                                b.kind = BKind::Capital;
                                self.nations[ni].capital = Some(bh);
                            }
                        }
                        if ni == self.player {
                            let nm = self.bld(bh).unwrap().kind.name();
                            self.push_msg(tf!("{}が完成した", "{} completed", "{}建成了", nm), [0.7, 0.9, 1.0]);
                        }
                    }
                } else {
                    let ap = self.bld_approach_point(bh, pos);
                    let nr = if b.kind.blocks() { 0.45 } else { 0.85 };
                    if self.nav(i, ap, nr, dt) {
                        let reached = self.bld(bh).map_or(false, |b| {
                            if b.kind.blocks() {
                                b.dist_to_footprint(self.units[i].pos) <= 1.25
                            } else {
                                self.units[i].pos.dist(b.center()) <= 1.5 || b.dist_to_footprint(self.units[i].pos) <= 0.75
                            }
                        });
                        if !reached {
                            // 辿り着けなければ次の予約へ
                            if !self.next_queued_build(i) {
                                self.units[i].order = Order::Idle;
                            }
                        }
                    }
                }
            }
        }
    }

    fn update_gather(&mut self, i: usize, src: Src, phase: u8, dt: f32) {
        let ni = self.units[i].nation as usize;
        let res = src.res();
        // 資源の位置と有効性
        let (sp, valid, sr) = match src {
            Src::Farm(h) => match self.bld(h) {
                Some(b) if b.nation as usize == ni && b.done() => (b.center(), true, 1.0),
                _ => (v2(0.0, 0.0), false, 0.0),
            },
            Src::Forest(x, y) => (v2(x as f32 + 0.5, y as f32 + 0.5), Map::inb(x, y) && self.map.terrain[Map::idx(x, y)] == Terrain::Forest, 1.2),
            Src::Mine(m) => {
                let mm = &self.map.mines[m];
                (v2(mm.x as f32 + 0.5, mm.y as f32 + 0.5), mm.amount > 0.0, 1.0)
            }
        };
        let cap = 10.0 * self.units[i].power;
        if phase != 2 && !valid {
            // 近くの同種資源へ
            let p = self.units[i].pos;
            let next = match src {
                Src::Forest(..) => self.nearest_forest(p, 8).map(|(x, y)| Src::Forest(x, y)),
                Src::Mine(_) => self.nearest_mine(p, 10.0).map(Src::Mine),
                Src::Farm(_) => self.free_farm(ni, p).map(Src::Farm),
            };
            match next {
                Some(s) => self.units[i].order = Order::Gather { src: s, phase: 0 },
                None => {
                    self.units[i].order = if self.units[i].carry > 0.0 { Order::Gather { src, phase: 2 } } else { Order::Idle };
                }
            }
            return;
        }
        match phase {
            0 => {
                if self.units[i].carry_kind != res {
                    self.units[i].carry = 0.0;
                    self.units[i].carry_kind = res;
                }
                if self.units[i].carry >= cap {
                    self.units[i].order = Order::Gather { src, phase: 2 };
                    return;
                }
                if self.nav(i, sp, sr, dt) {
                    if self.units[i].pos.dist(sp) <= sr + 0.6 {
                        self.units[i].order = Order::Gather { src, phase: 1 };
                    } else {
                        self.units[i].order = Order::Idle;
                    }
                }
            }
            1 => {
                let base = [0.9, 0.8, 0.5][res];
                let n = &self.nations[ni];
                let amt = base * self.units[i].power * n.bias * self.era_mult() * dt;
                let mut take = amt;
                match src {
                    Src::Forest(x, y) => {
                        let ti = Map::idx(x, y);
                        take = take.min(self.map.wood[ti]);
                        self.map.wood[ti] -= take;
                        if self.map.wood[ti] <= 0.0 {
                            self.map.terrain[ti] = Terrain::Plain;
                            if self.bmap[ti] == 0 {
                                self.cost[ti] = 1.0;
                            }
                            self.terrain_dirty.push((x, y));
                        }
                    }
                    Src::Mine(m) => {
                        take = take.min(self.map.mines[m].amount);
                        self.map.mines[m].amount -= take;
                    }
                    Src::Farm(_) => {}
                }
                let sp_dir = (sp - self.units[i].pos).norm();
                let u = &mut self.units[i];
                if sp_dir.len() > 0.0 {
                    u.dir = sp_dir;
                }
                u.carry += take;
                if u.carry >= cap {
                    u.carry = cap;
                    u.order = Order::Gather { src, phase: 2 };
                }
            }
            _ => {
                let p = self.units[i].pos;
                let Some(di) = self.nearest_dropsite(ni, p) else {
                    self.units[i].order = Order::Idle;
                    return;
                };
                let dh = self.bh(di);
                if self.blds[di].dist_to_footprint(p) <= 1.15 {
                    self.units[i].path.clear();
                    let carry = self.units[i].carry * self.nations[ni].eco_mul * self.nations[ni].tm.eco;
                    let k = self.units[i].carry_kind;
                    self.nations[ni].res[k] += carry;
                    self.nations[ni].earned[k] += carry;
                    self.units[i].carry = 0.0;
                    self.units[i].order = Order::Gather { src, phase: 0 };
                } else {
                    let ap = self.bld_approach_point(dh, p);
                    if self.nav(i, ap, 0.5, dt) && self.blds[di].dist_to_footprint(self.units[i].pos) > 1.35 {
                        self.units[i].order = Order::Idle;
                    }
                }
            }
        }
    }

    fn separate(&mut self, dt: f32) {
        // 押し合いは 0.55 以内のユニット同士だけ。格子（4マス）のうち、その範囲にかかる升（たいてい1つ、多くて4つ）だけを見る
        const R: f32 = 0.55;
        let mut pushes: Vec<(usize, V2)> = Vec::new();
        for a in 0..self.units.len() {
            let ua = &self.units[a];
            if !ua.alive {
                continue;
            }
            let p = ua.pos;
            let g0x = ((p.x - R) as i32 / GCELL).clamp(0, GW - 1);
            let g1x = ((p.x + R) as i32 / GCELL).clamp(0, GW - 1);
            let g0y = ((p.y - R) as i32 / GCELL).clamp(0, GH - 1);
            let g1y = ((p.y + R) as i32 / GCELL).clamp(0, GH - 1);
            let mut push = v2(0.0, 0.0);
            for gy in g0y..=g1y {
                for gx in g0x..=g1x {
                    for &b in self.grid[(gy * GW + gx) as usize].iter() {
                        let b = b as usize;
                        if a == b {
                            continue;
                        }
                        let ub = &self.units[b];
                        if !ub.alive {
                            continue;
                        }
                        let d = p - ub.pos;
                        let l2 = d.x * d.x + d.y * d.y;
                        if l2 < R * R {
                            let l = l2.sqrt();
                            let dir = if l > 1e-4 { d * (1.0 / l) } else { v2(((a * 7) % 5) as f32 - 2.0, ((a * 3) % 5) as f32 - 2.0).norm() };
                            push = push + dir * (R - l);
                        }
                    }
                }
            }
            if push.x != 0.0 || push.y != 0.0 {
                pushes.push((a, push));
            }
        }
        for (i, p) in pushes {
            let np = self.units[i].pos + p * (dt * 3.0).min(0.5);
            if self.passable_at(np) {
                self.units[i].pos = np;
            }
        }
    }

    // ------------------------------------------------------------ 年次イベント
    fn yearly(&mut self) {
        let yb = START_BC - self.years;
        // 連合の終了
        let mut ended = Vec::new();
        for (k, c) in self.coalitions.iter().enumerate() {
            if self.years >= c.end || !self.nations[c.target].alive {
                ended.push(k);
            }
        }
        for k in ended.into_iter().rev() {
            let c = self.coalitions.remove(k);
            for &m in c.members.iter() {
                if self.nations[m].alive && self.nations[c.target].alive {
                    self.propose_peace(m, c.target, 8.0, t!("連合の解散", "the coalition disbanded", "联军解散"));
                }
            }
            let name = if c.kind == CoalKind::Hegemon { t!("覇者の呼びかけによる討伐", "The Hegemon's campaign", "霸主号召的讨伐") } else { t!("合従軍", "The Grand Alliance", "合纵军") };
            let t = tf!("{}（{}討伐）が解散した", "{} against {} has disbanded", "{}（讨伐{}）已解散", name, tx(self.nations[c.target].name));
            self.push_msg(t, [0.9, 0.9, 0.6]);
        }
        // 覇者の呼びかけ（春秋）
        if self.era == Era::Spring {
            let mut cand = None;
            for (i, n) in self.nations.iter().enumerate() {
                if n.alive && n.aggression >= 100.0 && n.hegemon_cd <= self.years && !self.coalitions.iter().any(|c| c.target == i) {
                    cand = Some(i);
                    break;
                }
            }
            if let Some(t) = cand {
                self.start_coalition(CoalKind::Hegemon, t);
            }
        } else if self.years >= self.hezong_next {
            // 合従軍（戦国）
            let mut ranks: Vec<(f32, usize)> = self.nations.iter().enumerate().filter(|(_, n)| n.alive).map(|(i, n)| (n.strength, i)).collect();
            ranks.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            if ranks.len() >= 3 && ranks[0].0 > ranks[1].0 * 1.2 && !self.coalitions.iter().any(|c| c.kind == CoalKind::Hezong) {
                self.start_coalition(CoalKind::Hezong, ranks[0].1);
                self.hezong_next = self.years + 30.0;
            } else {
                self.hezong_next = self.years + 4.0;
            }
        }
        self.run_history();
        self.yearly_trade();
        self.refresh_traits();
        // 終了判定
        let alive: Vec<usize> = (0..NATIONS.len()).filter(|&i| self.nations[i].alive).collect();
        if self.descent.is_none() {
            if alive.len() == 1 && alive[0] == self.player {
                self.game_over = Some((true, tf!("{} は天下を統一した！", "{} has unified the realm!", "{}统一了天下！", tx(self.nations[self.player].name))));
            } else if yb <= END_BC {
                // 史実の統一の年までに天下が定まらなければ、上帝が秦に降臨する
                self.begin_descent();
            }
        } else {
            self.yearly_descent();
        }
    }

    fn start_coalition(&mut self, kind: CoalKind, target: usize) {
        self.start_coalition_ex(kind, target, true);
    }

    /// 連合を結成する。announce が偽なら通知は呼び出し側に任せる
    fn start_coalition_ex(&mut self, kind: CoalKind, target: usize, announce: bool) -> bool {
        let tc = self.nations[target].home;
        let reach = if kind == CoalKind::Hegemon { 140.0 } else { 240.0 };
        let mut members: Vec<usize> = (0..NATIONS.len())
            .filter(|&i| i != target && self.nations[i].alive && self.nations[i].home.dist(tc) <= reach)
            .collect();
        // 盟主：最も強い参加国（覇者）
        let mut leader = None;
        let mut best = -1.0;
        for (i, n) in self.nations.iter().enumerate() {
            if i != target && n.alive && n.strength > best && (members.contains(&i) || kind == CoalKind::Hegemon) {
                best = n.strength;
                leader = Some(i);
            }
        }
        let Some(leader) = leader else { return false };
        if !members.contains(&leader) {
            members.push(leader);
        }
        if members.len() < 2 {
            self.nations[target].hegemon_cd = self.years + 10.0;
            return false;
        }
        // 参加国同士は休戦し、一斉に宣戦
        for a in 0..members.len() {
            for b in a + 1..members.len() {
                let (ma, mb) = (members[a], members[b]);
                if self.war[ma][mb] {
                    self.propose_peace(ma, mb, 15.0, t!("連合への参加", "joining a coalition", "加入联军"));
                }
            }
        }
        for &m in members.iter() {
            self.declare_war(m, target, true);
        }
        let dur = 15.0;
        let (title, text) = match kind {
            CoalKind::Hegemon => (
                t!("覇者の呼びかけ", "The Hegemon's Call", "霸主号召").to_string(),
                tf!("{} が諸侯に呼びかけた。{} を討つべく {} か国が兵を挙げる（{}年間）", "{0} has called on the lords. {2} states take up arms against {1} ({3} years)", "{}号召诸侯，{2}国起兵讨伐{1}（{3}年）", tx(self.nations[leader].name), tx(self.nations[target].name), members.len(), dur as i32),
            ),
            CoalKind::Hezong => (
                t!("合従軍", "Grand Alliance", "合纵军").to_string(),
                tf!("最強国 {} に対し、{} を盟主として {} か国が合従した（{}年間）", "{2} states led by {1} have allied against {0}, the strongest power ({3} years)", "以{1}为盟主的{2}国合纵，对抗最强国{0}（{3}年）", tx(self.nations[target].name), tx(self.nations[leader].name), members.len(), dur as i32),
            ),
        };
        if announce {
            self.push_msg(format!("【{}】{}", title, text), [1.0, 0.85, 0.3]);
            self.queue_banner(Banner { title, text, t: 7.0 });
        }
        self.nations[target].aggression = 0.0;
        self.nations[target].hegemon_cd = self.years + 40.0;
        self.coalitions.push(Coalition { kind, target, leader, members, end: self.years + dur });
        true
    }

    /// 三家分晋と戦国時代の到来
    fn begin_warring(&mut self) {
        self.era = Era::Warring;
        self.hezong_next = self.years + 15.0;
        let mut text = String::from(t!("周王室の権威は失墜し、七雄が覇を競う時代となった。", "The authority of the Zhou kings has collapsed, and the Seven Powers contend for supremacy. ", "周王室权威扫地，七雄争霸的时代到来了。"));
        if self.nations[JIN].alive {
            text.push_str(t!("晋は韓・趙・魏の三家に分割された。", "Jin has been divided among the Han, Zhao and Wei clans. ", "晋被韩、赵、魏三家瓜分。"));
            let seats = [HAN, ZHAO, WEI];
            let seat_pos: Vec<V2> = seats.iter().map(|&s| self.nations[s].home).collect();
            let near = |p: V2| -> usize {
                let mut b = 0;
                for k in 1..3 {
                    if seat_pos[k].dist(p) < seat_pos[b].dist(p) {
                        b = k;
                    }
                }
                seats[b]
            };
            let res = self.nations[JIN].res;
            for &s in seats.iter() {
                let n = &mut self.nations[s];
                n.alive = true;
                n.appeared = true;
                n.res = [res[0] / 3.0 + 150.0, res[1] / 3.0 + 150.0, res[2] / 3.0 + 80.0];
                n.capital = None;
            }
            for i in 0..self.blds.len() {
                if self.blds[i].alive && self.blds[i].nation as usize == JIN {
                    let s = near(self.blds[i].center());
                    self.blds[i].nation = s as u8;
                    if self.blds[i].kind == BKind::Capital {
                        self.blds[i].kind = BKind::Town;
                    }
                }
            }
            for i in 0..self.units.len() {
                if self.units[i].alive && self.units[i].nation as usize == JIN {
                    let s = near(self.units[i].pos);
                    self.units[i].nation = s as u8;
                    self.units[i].order = Order::Idle;
                }
            }
            // 晋の戦争状態を継承
            for o in 0..NATIONS.len() {
                if self.war[JIN][o] {
                    for &s in seats.iter() {
                        self.war[s][o] = true;
                        self.war[o][s] = true;
                    }
                }
                self.war[JIN][o] = false;
                self.war[o][JIN] = false;
            }
            for &s in seats.iter() {
                // 都を定める（城邑が無ければ新たに築く）
                let mut cap = None;
                let mut best = f32::MAX;
                for (i, b) in self.blds.iter().enumerate() {
                    if b.alive && b.nation as usize == s && b.kind.city() {
                        let d = b.center().dist(self.nations[s].home);
                        if d < best {
                            best = d;
                            cap = Some(i);
                        }
                    }
                }
                if cap.is_none() {
                    let d = &NATIONS[s];
                    if let Some(h) = self.found_city_free(s, d.lon, d.lat) {
                        cap = Some(h.idx as usize);
                    }
                }
                if let Some(ci) = cap {
                    self.blds[ci].kind = BKind::Capital;
                    let h = self.bh(ci);
                    self.nations[s].capital = Some(h);
                    self.nations[s].home = self.blds[ci].center();
                    let cnt = self.units.iter().filter(|u| u.alive && u.nation as usize == s).count();
                    if cnt < 4 {
                        for k in [UnitKind::Worker, UnitKind::Worker, UnitKind::Infantry, UnitKind::Infantry] {
                            if let Some(sp) = self.spawn_point(h) {
                                self.spawn_unit(s, k, sp, 1.0);
                            }
                        }
                    }
                } else {
                    self.nations[s].alive = false;
                    for i in 0..self.units.len() {
                        if self.units[i].alive && self.units[i].nation as usize == s {
                            self.kill_unit(i);
                        }
                    }
                }
            }
            // 三家同士は当面休戦
            for a in seats {
                for b in seats {
                    if a != b {
                        self.truce[a][b] = self.years + 20.0;
                    }
                }
            }
            if self.player == JIN {
                let mut best = (0, WEI);
                for &s in seats.iter() {
                    let c = self.blds.iter().filter(|b| b.alive && b.nation as usize == s && b.kind.city()).count();
                    if c > best.0 || (c == best.0 && s == WEI) {
                        best = (c, s);
                    }
                }
                self.player = best.1;
                text.push_str(&tf!("あなたは {} を率いる。", "You now lead {}.", "你将率领{}。", tx(self.nations[best.1].name)));
            }
            self.nations[JIN].alive = false;
            self.nations[JIN].capital = None;
            self.territory_dirty = true;
        }
        self.push_msg(tf!("【戦国時代】{}", "[Warring States] {}", "【战国时代】{}", text), [1.0, 0.85, 0.3]);
        self.queue_banner(Banner { title: t!("戦国時代の幕開け", "The Warring States Begin", "战国时代的开幕").into(), text, t: 8.0 });
        self.update_stats();
    }

    fn found_city_free(&mut self, nation: usize, lon: f32, lat: f32) -> Option<Handle> {
        let (fx, fy) = ll_to_tile(lon, lat);
        let (cx, cy) = (fx as i32 - 1, fy as i32 - 1);
        for r in 0..12i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (tx, ty) = (cx + dx, cy + dy);
                    if !self.footprint_free(tx, ty) {
                        continue;
                    }
                    let c = v2(tx as f32 + 1.0, ty as f32 + 1.0);
                    if self.blds.iter().any(|b| b.alive && b.kind.city() && b.center().dist(c) < 6.0) {
                        continue;
                    }
                    return Some(self.spawn_building(nation, BKind::Capital, tx, ty, true));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 年表は降順で、国の番号と国姓表が国家データと揃っている
    #[test]
    fn history_table() {
        assert_eq!(INITIAL_HOUSE.len(), NATIONS.len());
        for w in HISTORY.windows(2) {
            assert!(w[0].bc >= w[1].bc, "{} の後に {}", w[0].title, w[1].title);
        }
        for e in HISTORY {
            assert!(e.nation < NATIONS.len());
            assert!(e.bc <= START_BC as i32 && e.bc > END_BC as i32);
        }
    }

    /// 秦（雍）から蜀（成都）へ、秦嶺・大巴山の古道を通って行ける
    #[test]
    fn qin_shu_road() {
        let w = World::new(12345);
        let a = w.nations[2].home.tile();
        let b = w.nations[23].home.tile();
        let mut pf = PathFinder::new();
        let path = pf.find(&w.cost, a, b, 3, 200000, crate::path::UNIT_WEIGHT);
        let end = *path.last().unwrap();
        let reach = (end.0 as i32 - b.0).abs().max((end.1 as i32 - b.1).abs());
        println!("秦→蜀 経路長 {} 終点の距離 {}", path.len(), reach);
        assert!(reach <= 3, "蜀に届かない");
        assert!(path.len() < 110 * MAP_SCALE, "遠回りしている: {}", path.len());
    }

    /// 前221年に上帝が秦に降臨し、10年で天下を統一して終わる（滅んだ秦も蘇る）
    #[test]
    fn descent_unifies() {
        let mut w = World::new(12345);
        w.player = HAN;
        // 秦を先に滅ぼしておく
        w.eliminate(2, 1);
        w.game_over = None;
        w.years = START_BC - END_BC - 0.5;
        let mut steps = 0;
        while w.game_over.is_none() && steps < 200_000 {
            w.step();
            steps += 1;
            if let Some((_, t)) = &w.game_over {
                if !t.contains("天下を統一した") {
                    // 自国が滅んでも観戦を続ける
                    w.game_over = None;
                }
            }
        }
        let d = w.descent.as_ref().expect("上帝が降臨していない");
        assert!(d.revived, "滅んだ秦が蘇っていない");
        let alive: Vec<&str> = (0..NATIONS.len()).filter(|&i| w.nations[i].alive).map(|i| w.nations[i].name).collect();
        println!("降臨 BC{} → 終了 BC{} 残存 {:?} {:?}", (START_BC - d.start) as i32, w.year_bc(), alive, w.game_over);
        assert_eq!(alive, vec!["秦"]);
        assert!(w.years - d.start <= DESCENT_YEARS + 1.01);
        assert!(matches!(&w.game_over, Some((false, _))));
    }

    /// 英霊：民を生贄に呼び、能力は生贄の合計×倍率。一国一体。上帝の兵を傷つけられるのは英霊だけ
    #[test]
    fn hero_summon_rules() {
        let mut w = World::new(12345);
        let qi = 4;
        let workers: Vec<Handle> = (0..w.units.len()).filter(|&i| w.units[i].alive && w.units[i].nation as usize == qi && w.units[i].kind == UnitKind::Worker).map(|i| w.uh(i)).collect();
        assert!(w.summon_hero(qi, 3, &workers).is_err(), "英霊モードでなければ呼べない");
        w.heroic = true;
        let (v, p) = w.sacrifice_power(qi, &workers);
        let before = w.units.iter().filter(|u| u.alive && u.nation as usize == qi && u.kind == UnitKind::Worker).count();
        let h = w.summon_hero(qi, 3, &workers).expect("召喚できない");
        let hero = w.unit(h).unwrap();
        assert_eq!(hero.hero, 3);
        assert!((hero.power - p * HEROES[3].mult).abs() < 1e-3);
        let after = w.units.iter().filter(|u| u.alive && u.nation as usize == qi && u.kind == UnitKind::Worker).count();
        assert_eq!(after, before - v.len(), "生贄の民が残っている");
        // 一国一体、同じ英霊は二国に現れない
        let more: Vec<Handle> = (0..w.units.len()).filter(|&i| w.units[i].alive && w.units[i].nation as usize == qi).map(|i| w.uh(i)).collect();
        assert!(w.can_summon(qi, 5, &more).is_err());
        let chu = 3;
        let cw: Vec<Handle> = (0..w.units.len()).filter(|&i| w.units[i].alive && w.units[i].nation as usize == chu && w.units[i].kind == UnitKind::Worker).map(|i| w.uh(i)).collect();
        assert!(w.can_summon(chu, 3, &cw).is_err(), "黄帝が二国に現れた");
        assert!(w.can_summon(chu, 4, &cw).is_ok());
        // 上帝の兵：普通の兵では傷つかず、英霊なら傷つく
        let pos = w.unit(h).unwrap().pos;
        let g = w.spawn_unit(2, UnitKind::Infantry, pos, 12.0);
        w.unit_mut(g).unwrap().divine = true;
        let gi = g.idx as usize;
        let hp0 = w.units[gi].hp;
        let soldier = (0..w.units.len()).find(|&i| w.units[i].alive && w.units[i].nation as usize == chu && w.units[i].kind.military()).unwrap();
        let sh = w.uh(soldier);
        w.damage_unit(gi, 100.0, Some(sh), chu);
        assert_eq!(w.units[gi].hp, hp0);
        w.damage_unit(gi, 100.0, Some(h), qi);
        assert!(w.units[gi].hp < hp0);
        // 英霊の近くの城邑は守られる
        let cap = w.nations[qi].capital.unwrap().idx as usize;
        let c = w.blds[cap].center();
        w.unit_mut(h).unwrap().pos = c;
        assert!(w.hero_guarded(cap));
    }

    /// 英霊モード：上帝降臨から10年後、版図が上帝の秦より広ければ勝利（秦を滅ぼした場合）
    #[test]
    fn heroic_win_by_territory() {
        let mut w = World::new(12345);
        w.heroic = true;
        w.player = 4;
        w.era = Era::Warring;
        w.years = START_BC - 221.0;
        w.begin_descent();
        // 英霊の軍が秦を滅ぼした
        w.eliminate(2, 4);
        w.game_over = None;
        w.descent.as_mut().unwrap().start = w.years - DESCENT_YEARS - 0.5;
        w.yearly_descent();
        assert!(matches!(&w.game_over, Some((true, _))), "{:?}", w.game_over);
        // 英霊モードでなければ、同じ状況でも勝利にはならない
        let mut w = World::new(12345);
        w.player = 4;
        w.era = Era::Warring;
        w.years = START_BC - 221.0;
        w.begin_descent();
        w.descent.as_mut().unwrap().start = w.years - DESCENT_YEARS - 0.5;
        w.yearly_descent();
        assert!(matches!(&w.game_over, Some((false, _))) || !w.nations[4].alive);
    }

    /// 建設の予約：民は今の建設を終えてから、予約した順に建てていく
    #[test]
    fn build_queue_in_order() {
        let mut w = World::new(12345);
        let n = 4;
        w.player = n;
        w.nations[n].res = [99999.0; 3];
        let home = w.nations[n].home;
        let mut spots: Vec<(i32, i32)> = Vec::new();
        for r in 3..10 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let (tx, ty) = (home.x as i32 + dx, home.y as i32 + dy);
                    if spots.len() < 3 && w.can_place(n, BKind::Farm, tx, ty).is_ok() && !spots.iter().any(|&(x, y)| (x - tx).abs() < 3 && (y - ty).abs() < 3) {
                        spots.push((tx, ty));
                    }
                }
            }
        }
        assert_eq!(spots.len(), 3);
        // 民を1人だけ使う
        let wk = (0..w.units.len()).find(|&i| w.units[i].alive && w.units[i].nation as usize == n && w.units[i].kind == UnitKind::Worker).unwrap();
        let h = w.uh(wk);
        let bs: Vec<Handle> = spots.iter().map(|&(x, y)| w.place_building(n, BKind::Farm, x, y).unwrap()).collect();
        // 1つ目は即座に、2つ目以降は予約
        for &b in bs.iter() {
            w.queue_build(&[h], b);
        }
        assert_eq!(w.unit(h).unwrap().order, Order::Build(bs[0]));
        assert_eq!(w.unit(h).unwrap().build_queue.len(), 2);
        // 2つ目の建設が始まる前に1つ目が完成している
        let mut done_order = Vec::new();
        for _ in 0..(120.0 / DT) as usize {
            w.step();
            for (k, &b) in bs.iter().enumerate() {
                if w.bld(b).map_or(false, |x| x.done()) && !done_order.contains(&k) {
                    done_order.push(k);
                }
            }
            if done_order.len() == 3 {
                break;
            }
        }
        assert_eq!(done_order, vec![0, 1, 2]);
        // ほかの命令を出すと予約は取り消される
        let more: Vec<Handle> = (0..2).map(|_| bs[0]).collect();
        w.unit_mut(h).unwrap().build_queue.extend(more);
        w.order_move(&[h], home, false);
        assert!(w.unit(h).unwrap().build_queue.is_empty());
    }

    /// 要塞：数に上限があり、近づく敵兵を射る
    #[test]
    fn fort_cap_and_shoots() {
        let mut w = World::new(12345);
        let n = 4;
        w.nations[n].res = [99999.0; 3];
        let cap = w.nations[n].fort_cap();
        let home = w.nations[n].home;
        // 自国領内の空き地に上限まで築く
        let mut spots = Vec::new();
        for r in 3..12 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let (tx, ty) = (home.x as i32 + dx, home.y as i32 + dy);
                    if w.can_place(n, BKind::Fort, tx, ty).is_ok() && !spots.iter().any(|&(x, y): &(i32, i32)| (x - tx).abs() < 3 && (y - ty).abs() < 3) {
                        spots.push((tx, ty));
                    }
                }
            }
        }
        assert!(spots.len() > cap as usize);
        let mut hs = Vec::new();
        for &(tx, ty) in spots.iter().take(cap as usize) {
            hs.push(w.place_building(n, BKind::Fort, tx, ty).unwrap());
        }
        let (tx, ty) = spots[cap as usize];
        assert!(w.place_building(n, BKind::Fort, tx, ty).is_err(), "上限を超えて築けてしまう");
        // 完成した要塞は射程内の敵兵を射る
        let b = hs[0].idx as usize;
        w.blds[b].progress = 1.0;
        w.blds[b].hp = BKind::Fort.max_hp();
        let c = w.blds[b].center();
        let e = 5;
        w.declare_war(n, e, true);
        let uh = w.spawn_unit(e, UnitKind::Infantry, c + v2(FORT_RANGE - 1.5, 0.0), 1.0);
        let hp0 = w.unit(uh).unwrap().hp;
        w.rebuild_grid();
        for _ in 0..30 {
            w.update_buildings(DT);
        }
        assert!(w.unit(uh).map_or(true, |u| u.hp < hp0), "要塞が射たない");
    }

    /// 訓練・教化の費用は蓄えの多い資源から払う
    #[test]
    fn upgrade_cost_follows_stock() {
        let mut w = World::new(12345);
        let i = (0..w.units.len()).find(|&i| w.units[i].alive && w.units[i].nation == 4 && w.units[i].kind.military()).unwrap();
        let p = w.units[i].power;
        let total = (40.0 * p * w.nations[4].tm.upgrade).round();
        // 木が余っていれば木で多く払う
        w.nations[4].res = [100.0, 5000.0, 10.0];
        let c = w.upgrade_cost(&w.units[i]);
        assert_eq!(c[0] + c[1] + c[2], total);
        assert!(c[1] > c[0] && c[1] > c[2] && c[2] <= 10.0, "{:?}", c);
        // 銅がなくても訓練できる
        let h = w.uh(i);
        let x = w.nations[4].capital.and_then(|b| w.bld(b)).unwrap().center();
        w.units[i].pos = x;
        assert!(w.upgrade_unit(h).is_ok());
        // 蓄えが乏しければ従来どおり糧と銅
        w.nations[4].res = [1.0, 1.0, 1.0];
        let c = w.upgrade_cost(&w.units[i]);
        assert!(c[1] == 0.0 && c[0] > c[2] && c[2] > 0.0, "{:?}", c);
    }

    /// 国が滅ぶと、その国の人口上限が生き残った国に分けられる
    #[test]
    fn fallen_cap_is_shared() {
        let mut w = World::new(12345);
        w.player = 4;
        let total = |w: &World| (0..NATIONS.len()).filter(|&i| w.nations[i].alive).map(|i| w.nations[i].cap_bonus).sum::<u32>();
        assert_eq!(total(&w), 0);
        let cap0 = w.nations[4].worker_cap();
        // 斉（4）が莒（26）を滅ぼすと、斉に1つ以上分けられる
        w.eliminate(26, 4);
        assert_eq!(total(&w), BASE_CAP);
        assert!(w.nations[4].cap_bonus >= 1);
        assert_eq!(w.nations[4].worker_cap(), cap0 + w.nations[4].cap_bonus);
        assert_eq!(w.nations[4].mil_cap(), w.nations[4].worker_cap());
        // 分けられた分も、その国が滅べばまた分けられる（合計は減らない）
        w.nations[5].cap_bonus += 6;
        let before = total(&w);
        w.eliminate(5, 4);
        assert_eq!(total(&w), before + BASE_CAP);
        // 上限25を超えても加わる
        w.nations[4].cities = 20;
        assert_eq!(w.nations[4].worker_cap(), 25 + w.nations[4].cap_bonus);
    }

    /// 和睦はプレイヤーに選ばせる：拒否すれば交戦が続き、受諾すれば休戦する
    #[test]
    fn peace_needs_player_answer() {
        let mut w = World::new(12345);
        w.player = 4;
        w.declare_war(4, 5, false);
        // 連合の解散などによる和睦は、プレイヤーの返答を待つ
        assert!(!w.propose_peace(5, 4, 8.0, "連合の解散"));
        assert!(w.war[4][5] && w.peace_asks.len() == 1);
        // 同じ国から重ねて申し入れない
        w.propose_peace(4, 5, 8.0, "連合の解散");
        assert_eq!(w.peace_asks.len(), 1);
        w.answer_peace(false);
        assert!(w.war[4][5] && w.peace_asks.is_empty() && w.offer_cd[5] > w.years);
        w.propose_peace(5, 4, 8.0, "連合の解散");
        w.answer_peace(true);
        assert!(!w.war[4][5] && w.truce[4][5] > w.years + 7.0);
        // プレイヤーが当事者でなければ即座に和睦する
        w.declare_war(6, 7, false);
        assert!(w.propose_peace(6, 7, 8.0, "連合の解散") && !w.war[6][7]);
    }

    /// 全 AI で長期間シミュレーションし、国家の推移とイベントを確認する
    #[test]
    fn long_sim() {
        let seed = std::env::var("SIM_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(12345u32);
        let mut w = World::new(seed);
        w.player = 999usize.min(w.nations.len() - 1);
        // プレイヤー不在扱い：最後の国（韓、休眠）をプレイヤーにする
        w.player = HAN;
        w.heroic = std::env::var("SIM_HEROIC").is_ok();
        let t0 = std::time::Instant::now();
        let years = std::env::var("SIM_YEARS").ok().and_then(|s| s.parse().ok()).unwrap_or(560.0f32);
        let mut next_report = 0.0;
        let mut seen_t = -1.0f32;
        let mut steps = 0usize;
        while w.years < years + 1.0 {
            steps += 1;
            // 観戦用：見ている国が滅んでも最後まで進める
            if w.game_over.is_some() && w.year_bc() > END_BC as i32 {
                w.game_over = None;
            }
            w.step();
            if w.years >= next_report {
                next_report += 25.0;
                let alive: Vec<String> = w
                    .nations
                    .iter()
                    .filter(|n| n.alive)
                    .map(|n| format!("{}(c{}+{} w{} s{} p{:.0} 糧{:.0}木{:.0}銅{:.0})", n.name, n.cities, n.cap_bonus, n.workers, n.soldiers, n.strength, n.res[0], n.res[1], n.res[2]))
                    .collect();
                let units = w.units.iter().filter(|u| u.alive).count();
                let forts = w.blds.iter().filter(|b| b.alive && b.kind == BKind::Fort && b.done()).count();
                println!("BC{} t={:.1}s units={} forts={} alive={} :: {}", w.year_bc(), t0.elapsed().as_secs_f32(), units, forts, alive.len(), alive.join(" "));
                let mut coins: Vec<(f32, usize)> = (0..NATIONS.len()).filter(|&i| w.nations[i].alive).map(|i| (w.nations[i].coin_income, i)).collect();
                coins.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                let top: Vec<String> = coins.iter().take(6).map(|&(c, i)| format!("{}{:.2}/s({:.0})", w.nations[i].name, c, w.nations[i].coin)).collect();
                let tr = &w.trade;
                println!(
                    "  [trade] 商人{}/{} 西域{}/{} 部族隊商{} 略奪団{} 傭兵{} 銭収入: {}",
                    tr.count(|k| k == CaravanKind::Merchant),
                    tr.target,
                    tr.count(|k| matches!(k, CaravanKind::Western(_))),
                    tr.target_w,
                    tr.count(|k| matches!(k, CaravanKind::Tribe(_))),
                    tr.bands.len(),
                    w.units.iter().filter(|u| u.alive && u.merc_until > 0.0).count(),
                    top.join(" ")
                );
                for (t, st) in tr.tribes.iter().enumerate() {
                    if tribe_active(t, w.year_bc()) && !st.last.is_empty() {
                        println!("    {}：{}", TRIBES[t].name, st.last);
                    }
                }
            }
            for m in w.msgs.iter().filter(|m| m.time > seen_t) {
                println!("  [msg] {}", m.text);
            }
            seen_t = w.time;
            if w.game_over.is_some() && w.years > years {
                break;
            }
        }
        println!("elapsed {:.1}s for {} steps", t0.elapsed().as_secs_f32(), steps);
    }

    #[test]
    fn resume_interrupted_build() {
        let mut w = World::new(12345);
        let pl = 0;
        w.player = pl;
        w.nations[pl].res = [900.0, 900.0, 900.0];
        let home = w.nations[pl].home;
        let (hx, hy) = home.tile();
        let worker = (0..w.units.len())
            .find(|&i| w.units[i].alive && w.units[i].nation as usize == pl && w.units[i].kind == UnitKind::Worker)
            .map(|i| w.uh(i))
            .unwrap();

        for kind in [BKind::Barracks, BKind::Farm] {
            let mut placed = None;
            for dy in -5..=5 {
                for dx in -5..=5 {
                    if let Ok(bh) = w.place_building(pl, kind, hx + dx, hy + dy) {
                        placed = Some(bh);
                        break;
                    }
                }
                if placed.is_some() {
                    break;
                }
            }
            let bh = placed.expect("should place building");
            let c = w.bld(bh).unwrap().center();

            // 1) 東側（x + 3.5）から建設開始 → 途中で移動中断 → 南東・南・北西から再開して完成するか検証
            for &(ox, oy) in &[(3.5f32, 0.5f32), (2.2, 2.2), (0.5, 3.6), (-1.8, -1.8)] {
                let start_pos = v2(c.x + ox, c.y + oy);
                if w.passable_at(start_pos) {
                    w.unit_mut(worker).unwrap().pos = start_pos;
                }
                w.order_build(&[worker], bh);
                for _ in 0..80 {
                    w.step();
                }
                let prog_before = w.bld(bh).unwrap().progress;
                assert!(prog_before > 0.02, "{:?} progress should advance from offset {:?}, got {}", kind, (ox, oy), prog_before);
                if w.bld(bh).unwrap().done() {
                    break;
                }
                // 中断（別の場所へ移動命令＋停止）
                w.order_move(&[worker], start_pos, false);
                for _ in 0..15 {
                    w.step();
                }
                w.unit_mut(worker).unwrap().order = Order::Idle;
                // 再開
                w.order_build(&[worker], bh);
            }
            for _ in 0..900 {
                if w.bld(bh).unwrap().done() {
                    break;
                }
                w.step();
            }
            assert!(w.bld(bh).unwrap().done(), "{:?} should complete after resuming construction", kind);
        }
    }
}

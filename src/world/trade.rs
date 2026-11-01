//! 交易：中原の商人・西域の商人、山岳民と遊牧民（いずれも国ではない人々）

use super::*;

/// 部族の兵が属する仮の枠（国としては扱わない。外交・連合・終了判定には現れない）
pub const TRIBE: usize = NATIONS.len();

/// 税率（軽税・中税・重税）：税として取る割合と、商人の集まりやすさ
/// 税率の名前（0:軽税 1:中税 2:重税）
pub fn tax_name(t: usize) -> &'static str {
    match t {
        0 => t!("軽税", "Light tax", "轻税"),
        1 => t!("中税", "Normal tax", "中税"),
        _ => t!("重税", "Heavy tax", "重税"),
    }
}
pub const TAX_RATE: [f32; 3] = [0.6, 1.0, 1.5];
pub const TAX_ATTRACT: [f32; 3] = [1.5, 1.0, 0.55];

/// 市場の値（銭）：100単位あたり
pub const MARKET_PRICE: [f32; 3] = [40.0, 50.0, 80.0];
pub const MARKET_LOT: f32 = 100.0;

/// 傭兵の契約年数と人数
pub const MERC_YEARS: f32 = 10.0;
pub const MERC_N: usize = 3;

// ---------------------------------------------------------------- 山岳民・遊牧民
pub struct TribeDef {
    pub name: &'static str,
    pub lon: f32,
    pub lat: f32,
    /// 活動する年代（紀元前）
    pub from_bc: i32,
    pub to_bc: i32,
    /// 遊牧民（騎馬）か山岳民（歩兵・弓）か
    pub nomad: bool,
    /// 略奪に向かいやすさ（0〜1）
    pub host: f32,
    pub desc: &'static str,
}

macro_rules! td {
    ($n:expr, $lon:expr, $lat:expr, $f:expr, $t:expr, $nomad:expr, $h:expr, $d:expr) => {
        TribeDef { name: $n, lon: $lon, lat: $lat, from_bc: $f, to_bc: $t, nomad: $nomad, host: $h, desc: $d }
    };
}

pub const TRIBES: &[TribeDef] = &[
    td!("犬戎", 104.9, 35.4, 770, 660, true, 0.75, "前771年に鎬京を陥れ西周を滅ぼした西戎。隴山の西に拠る"),
    td!("山戎", 118.6, 41.6, 770, 640, true, 0.65, "燕の北、燕山の外の戎。前663年に斉の桓公が討った"),
    td!("群蛮・百濮", 110.4, 29.9, 770, 480, false, 0.45, "楚の西南の山地の諸族。前611年、庸に従って楚を脅かした"),
    td!("百越", 119.4, 27.9, 770, 221, false, 0.40, "江南・閩の山地に住む越人の諸族"),
    td!("陸渾の戎", 111.9, 33.95, 638, 525, false, 0.50, "秦・晋によって伊川に移された戎。前525年に晋に滅ぼされた"),
    td!("西戎（綿諸・烏氏）", 105.2, 34.95, 660, 330, true, 0.55, "秦の穆公が覇を唱えた西方の戎の諸部"),
    td!("林胡", 110.2, 40.3, 560, 300, true, 0.60, "オルドスの遊牧民。趙の武霊王が胡服騎射で破った"),
    td!("東胡", 120.6, 41.9, 500, 221, true, 0.60, "燕の北東の遊牧民。燕の将・秦開に千里退けられた"),
    td!("西南夷（滇・夜郎）", 103.4, 26.2, 400, 221, false, 0.35, "雲貴高原の諸族。楚の荘蹻が滇に入った"),
    td!("月氏", 102.6, 38.4, 400, 221, true, 0.10, "河西の遊牧民。和田の玉を東へ運んだ（禺氏の玉）"),
    td!("匈奴", 110.6, 41.95, 318, 221, true, 0.80, "前318年に初めて史書に現れる北方の騎馬民。李牧が大破した"),
];

/// 部族の状態
#[derive(Clone)]
pub struct TribeState {
    /// 次に動く年（ゲーム内経過年）
    pub next: f32,
    /// 国ごとの雇用契約の満了年（その間はその国を襲わない）
    pub contract: Vec<f32>,
    /// 最近の動き
    pub last: String,
}

/// 略奪に出た部族の一団
pub struct Band {
    pub tribe: usize,
    pub target: usize,
    pub city: Handle,
    pub units: Vec<Handle>,
    /// 0:略奪へ 1:引き上げ
    pub phase: u8,
    pub until: f32,
    pub home: V2,
}

// ---------------------------------------------------------------- 西域
pub struct SilkRoute {
    pub name: &'static str,
    /// 地図上の入口（経度・緯度）
    pub entry: (f32, f32),
    pub from_bc: i32,
    /// 商人が訪れる地域（経度 lon0..lon1, 緯度 lat0..lat1）
    pub region: (f32, f32, f32, f32),
    pub desc: &'static str,
}

pub const ROUTES: &[SilkRoute] = &[
    SilkRoute { name: "河西回廊（玉の道）", entry: (102.1, 37.0), from_bc: 770, region: (102.0, 115.2, 33.4, 38.6), desc: "隴西から関中・河東・洛邑・鄭へ。斉・魯・宋や江南には来ない" },
    SilkRoute { name: "草原の道", entry: (102.1, 40.4), from_bc: 650, region: (102.0, 117.6, 37.4, 42.5), desc: "オルドスから晋陽・代・中山・燕へ" },
    SilkRoute { name: "蜀身毒道（西南の道）", entry: (102.05, 24.95), from_bc: 400, region: (102.0, 109.0, 24.0, 32.6), desc: "滇から蜀・巴へ（張騫が大夏で蜀布を見た道）" },
];

// ---------------------------------------------------------------- 隊商
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CaravanKind {
    /// 中原の商人
    Merchant,
    /// 西域の商人（道の番号）
    Western(u8),
    /// 部族の隊商（部族の番号）
    Tribe(u8),
}

pub struct Caravan {
    pub kind: CaravanKind,
    pub pos: V2,
    pub dir: V2,
    pub(crate) path: Vec<(i16, i16)>,
    pub(crate) path_i: usize,
    pub(crate) need_path: bool,
    pub(crate) dest: Option<Handle>,
    pub(crate) goal: V2,
    /// 市で商い中の残り秒
    pub stay: f32,
    pub(crate) trips: u8,
    pub(crate) max_trips: u8,
    /// 帰途（西域・部族）
    pub leaving: bool,
    /// 去った（取り除く）
    pub(crate) gone: bool,
    pub value: f32,
}

/// 交易全体の状態
pub struct Trade {
    pub caravans: Vec<Caravan>,
    pub tribes: Vec<TribeState>,
    pub bands: Vec<Band>,
    /// 中原の商人の目標数（中原全体の生産量に比例）
    pub target: usize,
    pub target_w: usize,
    pub(crate) spawn_t: f32,
    pub(crate) band_t: f32,
}

impl Trade {
    pub fn new(rng: &mut Rng) -> Trade {
        let tribes = TRIBES
            .iter()
            .map(|_| TribeState { next: rng.range(12.0, 26.0), contract: vec![0.0; NATIONS.len()], last: String::new() })
            .collect();
        Trade { caravans: Vec::new(), tribes, bands: Vec::new(), target: 8, target_w: 1, spawn_t: 0.0, band_t: 0.0 }
    }
    pub fn count(&self, f: impl Fn(CaravanKind) -> bool) -> usize {
        self.caravans.iter().filter(|c| f(c.kind)).count()
    }
}

/// 部族がその年に活動しているか
pub fn tribe_active(t: usize, yb: i32) -> bool {
    yb <= TRIBES[t].from_bc && yb >= TRIBES[t].to_bc
}

pub fn route_open(r: usize, yb: i32) -> bool {
    yb <= ROUTES[r].from_bc
}

fn in_region(r: usize, p: V2) -> bool {
    let (lon, lat) = tile_to_ll(p.x, p.y);
    let (a, b, c, d) = ROUTES[r].region;
    lon >= a && lon <= b && lat >= c && lat <= d
}

fn ll_v2(lon: f32, lat: f32) -> V2 {
    let (x, y) = ll_to_tile(lon, lat);
    v2(x, y)
}

impl World {
    /// 国の生産力（商人を引き寄せる力）
    pub fn production(&self, n: usize) -> f32 {
        let nn = &self.nations[n];
        nn.income[0] + nn.income[1] + nn.income[2]
    }

    /// 部族の本拠（通行できる地点）
    pub fn tribe_home(&self, t: usize) -> V2 {
        self.passable_near(ll_v2(TRIBES[t].lon, TRIBES[t].lat))
    }

    fn passable_near(&self, p: V2) -> V2 {
        let (cx, cy) = p.tile();
        for r in 0..28i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (x, y) = (cx + dx, cy + dy);
                    if Map::inb(x, y) && self.cost[Map::idx(x, y)].is_finite() && self.map.terrain[Map::idx(x, y)] != Terrain::River {
                        return v2(x as f32 + 0.5, y as f32 + 0.5);
                    }
                }
            }
        }
        p
    }

    /// 傭兵の値（銭）
    pub fn merc_cost(&self) -> f32 {
        (150.0 + self.years * 0.3).round()
    }

    /// 部族の兵の能力
    fn tribe_power(&self) -> f32 {
        (1.0 + 2.2 * self.years / (START_BC - END_BC)).min(self.power_cap())
    }

    /// その国が部族から傭兵を雇えるか（雇えない理由）
    pub fn can_hire(&self, n: usize, t: usize) -> Result<(), &'static str> {
        if !tribe_active(t, self.year_bc()) {
            return Err(t!("この部族は今はいない", "This people is not active now", "该部族目前不在"));
        }
        let home = self.tribe_home(t);
        let near = self.blds.iter().any(|b| b.alive && b.nation as usize == n && b.kind.city() && b.done() && b.center().dist(home) < 150.0);
        if !near {
            return Err(t!("自国の城邑から遠すぎる", "Too far from your cities", "离己方城邑太远"));
        }
        if self.trade.bands.iter().any(|b| b.tribe == t && b.target == n && b.phase == 0) {
            return Err(t!("この部族は自国を襲っている", "This people is raiding you", "该部族正在劫掠我国"));
        }
        if self.nations[n].coin < self.merc_cost() {
            return Err(t!("銭が足りない", "Not enough coin", "钱不足"));
        }
        Ok(())
    }

    /// 傭兵を雇う：部族に最も近い自国の城邑に現れ、契約年数のあいだ仕える
    pub fn hire_mercs(&mut self, n: usize, t: usize) -> Result<(), &'static str> {
        self.can_hire(n, t)?;
        let home = self.tribe_home(t);
        let mut best: Option<(f32, usize)> = None;
        for (i, b) in self.blds.iter().enumerate() {
            if b.alive && b.nation as usize == n && b.kind.city() && b.done() {
                let d = b.center().dist(home);
                if best.map_or(true, |x| d < x.0) {
                    best = Some((d, i));
                }
            }
        }
        let Some((_, bi)) = best else { return Err(t!("城邑がない", "No city", "没有城邑")) };
        let cost = self.merc_cost();
        self.nations[n].coin -= cost;
        let bh = self.bh(bi);
        let power = (self.tribe_power() * 1.15).min(self.power_cap());
        let until = self.years + MERC_YEARS;
        for k in 0..MERC_N {
            let kind = if TRIBES[t].nomad {
                UnitKind::Chariot
            } else if k == 0 {
                UnitKind::Archer
            } else {
                UnitKind::Infantry
            };
            if let Some(sp) = self.spawn_point(bh) {
                let h = self.spawn_unit(n, kind, sp, power);
                let u = self.unit_mut(h).unwrap();
                u.tribe = t as u8;
                u.merc_until = until;
            }
        }
        self.trade.tribes[t].contract[n] = until;
        self.trade.tribes[t].last = tf!("{}に傭兵として仕える", "Serving {} as mercenaries", "受雇于{}为佣兵", tx(self.nations[n].name));
        if n == self.player {
            let s = tf!("{}の兵{}を傭兵として雇った（{}銭・{}年）", "Hired {1} {0} warriors as mercenaries ({2} coin, {3} years)", "雇佣了{}兵{}名为佣兵（{}钱・{}年）", tx(TRIBES[t].name), MERC_N, cost, MERC_YEARS as i32);
            self.push_msg(s, [0.6, 0.9, 0.6]);
        }
        Ok(())
    }

    /// 市場で資源を買う
    pub fn buy_resource(&mut self, n: usize, k: usize) -> Result<(), &'static str> {
        let price = self.market_price(n, k);
        let nn = &mut self.nations[n];
        if nn.coin < price {
            return Err(t!("銭が足りない", "Not enough coin", "钱不足"));
        }
        nn.coin -= price;
        nn.res[k] += MARKET_LOT;
        Ok(())
    }

    // ------------------------------------------------------------ 毎フレーム
    pub(super) fn update_trade(&mut self, dt: f32) {
        let em = self.era_mult();
        // ---- 隊商の移動と税
        let mut paths = 0;
        let mut arrivals: Vec<usize> = Vec::new();
        for i in 0..self.trade.caravans.len() {
            let c = &self.trade.caravans[i];
            let (x, y) = c.pos.tile();
            if Map::inb(x, y) {
                let o = self.owner[Map::idx(x, y)] as usize;
                if o < NATIONS.len() && self.nations[o].alive {
                    let n = &mut self.nations[o];
                    let add = c.value * 0.45 * em * TAX_RATE[n.tax as usize] * if c.stay > 0.0 { 2.5 } else { 1.0 } * dt;
                    n.coin += add;
                    n.coin_earned += add;
                }
            }
            if c.gone {
                continue;
            }
            if c.stay > 0.0 {
                let c = &mut self.trade.caravans[i];
                c.stay -= dt;
                if c.stay <= 0.0 {
                    arrivals.push(i);
                }
                continue;
            }
            if c.need_path {
                if paths >= 2 {
                    continue;
                }
                paths += 1;
                let start = c.pos.tile();
                let goal = c.goal.tile();
                let path = self.pf.find(&self.cost, start, goal, 2, 160000, crate::path::CARAVAN_WEIGHT);
                let c = &mut self.trade.caravans[i];
                c.path = path;
                c.path_i = 0;
                c.need_path = false;
                if c.path.is_empty() && c.pos.dist(c.goal) > 3.2 {
                    // 行き着けない：あきらめる
                    c.gone = true;
                }
                continue;
            }
            // 経路に沿って進む
            let sp = if matches!(c.kind, CaravanKind::Tribe(_)) { 2.6 } else { 2.0 };
            let (tx, ty) = c.pos.tile();
            let tc = if Map::inb(tx, ty) { self.map.terrain[Map::idx(tx, ty)].move_cost() } else { 1.0 };
            let mut mv = sp / tc.clamp(1.0, 3.0).sqrt() * dt;
            let c = &mut self.trade.caravans[i];
            while mv > 0.0 {
                let to = if c.path_i < c.path.len() {
                    let (px, py) = c.path[c.path_i];
                    v2(px as f32 + 0.5, py as f32 + 0.5)
                } else {
                    c.goal
                };
                let d = to - c.pos;
                let l = d.len();
                if l <= mv {
                    c.pos = to;
                    mv -= l;
                    if c.path_i < c.path.len() {
                        c.path_i += 1;
                    } else {
                        break;
                    }
                } else {
                    c.dir = d * (1.0 / l);
                    c.pos = c.pos + d * (mv / l);
                    mv = 0.0;
                }
            }
            if c.path_i >= c.path.len() {
                if c.pos.dist(c.goal) < 3.2 {
                    c.stay = if c.leaving { 0.01 } else { self.rng.range(6.0, 12.0) };
                } else {
                    c.need_path = true;
                }
            }
        }
        for i in arrivals {
            self.caravan_next(i);
        }
        // 帰り着いた・去った隊商
        self.trade.caravans.retain(|c| !c.gone);

        // ---- 隊商を送り出す
        self.trade.spawn_t -= dt;
        if self.trade.spawn_t <= 0.0 {
            self.trade.spawn_t = 0.8;
            self.spawn_caravans();
        }
        // ---- 略奪の一団
        self.trade.band_t -= dt;
        if self.trade.band_t <= 0.0 {
            self.trade.band_t = 1.0;
            self.update_bands();
        }
    }

    /// 都市を選ぶ：生産力が高く税が軽い国ほど、近いほど選ばれやすい
    fn pick_city(&mut self, from: V2, filter: impl Fn(&World, V2) -> bool, avoid: Option<Handle>) -> Option<Handle> {
        let mut cands: Vec<(f32, usize)> = Vec::new();
        let mut total = 0.0;
        for (i, b) in self.blds.iter().enumerate() {
            if !b.alive || !b.kind.city() || !b.done() || (b.nation as usize) >= NATIONS.len() {
                continue;
            }
            if avoid.map_or(false, |h| h.idx as usize == i) || !filter(self, b.center()) {
                continue;
            }
            let ni = b.nation as usize;
            let n = &self.nations[ni];
            let prod = (self.production(ni) + 0.5) / n.cities.max(1) as f32;
            let mut wgt = prod.powf(1.2) * TAX_ATTRACT[n.tax as usize] * n.tm.attract * if b.kind == BKind::Capital { 1.6 } else { 1.0 };
            if (0..NATIONS.len()).any(|o| self.at_war(ni, o)) {
                wgt *= 0.6;
            }
            wgt *= (-b.center().dist(from) / 140.0).exp();
            if wgt > 0.0 {
                total += wgt;
                cands.push((wgt, i));
            }
        }
        if cands.is_empty() {
            return None;
        }
        let mut r = self.rng.f32() * total;
        for (wgt, i) in cands.iter() {
            r -= wgt;
            if r <= 0.0 {
                return Some(self.bh(*i));
            }
        }
        Some(self.bh(cands.last().unwrap().1))
    }

    fn new_caravan(&mut self, kind: CaravanKind, pos: V2, value: f32, max_trips: u8) -> Caravan {
        Caravan {
            kind,
            pos,
            dir: v2(1.0, 0.0),
            path: Vec::new(),
            path_i: 0,
            need_path: false,
            dest: None,
            goal: pos,
            stay: self.rng.range(0.5, 2.0),
            trips: 0,
            max_trips,
            leaving: false,
            gone: false,
            value,
        }
    }

    /// 市での商いを終えた隊商の次の行き先
    fn caravan_next(&mut self, i: usize) {
        let (kind, pos, dest, trips, max_trips, leaving) = {
            let c = &self.trade.caravans[i];
            (c.kind, c.pos, c.dest, c.trips, c.max_trips, c.leaving)
        };
        if leaving {
            self.trade.caravans[i].gone = true;
            return;
        }
        let retire = match kind {
            CaravanKind::Merchant => {
                let n = self.trade.count(|k| k == CaravanKind::Merchant);
                n > self.trade.target && self.rng.chance(0.5)
            }
            _ => trips >= max_trips,
        };
        let next = if retire {
            None
        } else {
            match kind {
                CaravanKind::Merchant => self.pick_city(pos, |_, _| true, dest),
                CaravanKind::Western(r) => self.pick_city(pos, move |_, p| in_region(r as usize, p), dest),
                CaravanKind::Tribe(_) => None,
            }
        };
        let c = &mut self.trade.caravans[i];
        c.trips += 1;
        c.stay = 0.0;
        match next {
            Some(h) => {
                c.dest = Some(h);
                c.goal = self.blds[h.idx as usize].center();
                c.need_path = true;
            }
            None => match kind {
                CaravanKind::Merchant => c.gone = true,
                CaravanKind::Western(r) => {
                    let (lo, la) = ROUTES[r as usize].entry;
                    c.leaving = true;
                    c.goal = ll_v2(lo, la);
                    c.need_path = true;
                }
                CaravanKind::Tribe(t) => {
                    c.leaving = true;
                    c.goal = ll_v2(TRIBES[t as usize].lon, TRIBES[t as usize].lat);
                    c.need_path = true;
                }
            },
        }
        if c.leaving && c.need_path {
            let g = self.passable_near(self.trade.caravans[i].goal);
            self.trade.caravans[i].goal = g;
        }
    }

    fn spawn_caravans(&mut self) {
        let total: f32 = (0..NATIONS.len()).filter(|&i| self.nations[i].alive).map(|i| self.production(i)).sum();
        // 中原全体の生産量に比例して商人が増える
        self.trade.target = ((total * 0.16) as usize).clamp(6, 90);
        let yb = self.year_bc();
        let open: Vec<usize> = (0..ROUTES.len()).filter(|&r| route_open(r, yb)).collect();
        self.trade.target_w = if open.is_empty() { 0 } else { ((total * 0.012) as usize + open.len()).clamp(1, 12) };
        let nm = self.trade.count(|k| k == CaravanKind::Merchant);
        if nm < self.trade.target {
            let c = self.v2_center();
            if let Some(h) = self.pick_city(c, |_, _| true, None) {
                let p = self.blds[h.idx as usize].center();
                let off = v2(self.rng.range(-2.0, 2.0), self.rng.range(1.5, 2.5));
                let p = self.passable_near(p + off);
                let mut cv = self.new_caravan(CaravanKind::Merchant, p, 1.0, 255);
                cv.dest = Some(h);
                self.trade.caravans.push(cv);
            }
        }
        let nw = self.trade.count(|k| matches!(k, CaravanKind::Western(_)));
        if nw < self.trade.target_w && self.rng.chance(0.25) {
            // 道ごとの訪問先の生産力で選ぶ
            let mut best: Vec<(f32, usize)> = Vec::new();
            for &r in open.iter() {
                let mut s = 0.0;
                for b in self.blds.iter() {
                    if b.alive && b.kind.city() && b.done() && (b.nation as usize) < NATIONS.len() && in_region(r, b.center()) {
                        let ni = b.nation as usize;
                        s += (self.production(ni) + 0.5) / self.nations[ni].cities.max(1) as f32;
                    }
                }
                if s > 0.0 {
                    best.push((s, r));
                }
            }
            let tot: f32 = best.iter().map(|b| b.0).sum();
            let mut x = self.rng.f32() * tot;
            let mut pick = None;
            for (s, r) in best {
                x -= s;
                pick = Some(r);
                if x <= 0.0 {
                    break;
                }
            }
            if let Some(r) = pick {
                let (lo, la) = ROUTES[r].entry;
                let p = self.passable_near(ll_v2(lo, la));
                let trips = 2 + (self.rng.f32() * 3.0) as u8;
                let mut cv = self.new_caravan(CaravanKind::Western(r as u8), p, 3.0, trips);
                if let Some(h) = self.pick_city(p, move |_, q| in_region(r, q), None) {
                    cv.dest = Some(h);
                    cv.goal = self.blds[h.idx as usize].center();
                    cv.need_path = true;
                    cv.stay = 0.0;
                    self.trade.caravans.push(cv);
                    if self.blds[h.idx as usize].nation as usize == self.player && self.rng.chance(0.5) {
                        let s = tf!("西域の商人が{}を通ってやって来た", "Western merchants arrived by the {}", "西域商人经{}而来", tx(ROUTES[r].name));
                        self.push_msg(s, [0.85, 0.75, 1.0]);
                    }
                }
            }
        }
    }

    fn v2_center(&self) -> V2 {
        v2(MAP_W as f32 / 2.0, MAP_H as f32 / 2.0)
    }

    // ------------------------------------------------------------ 毎年：部族の動き・傭兵の満了
    pub(super) fn yearly_trade(&mut self) {
        let yb = self.year_bc();
        // 傭兵の契約満了
        let mut done = Vec::new();
        for (i, u) in self.units.iter().enumerate() {
            if u.alive && u.merc_until > 0.0 && self.years >= u.merc_until {
                done.push(i);
            }
        }
        if done.iter().any(|&i| self.units[i].nation as usize == self.player) {
            let t = self.units[done[0]].tribe as usize;
            let s = tf!("{}の傭兵は契約を終えて去った", "The {} mercenaries finished their contract and left", "{}佣兵合约期满离去", TRIBES.get(t).map_or(t!("部族", "tribal", "部族"), |d| tx(d.name)));
            self.push_msg(s, [0.85, 0.85, 0.7]);
        }
        for i in done {
            self.kill_unit(i);
        }
        for t in 0..TRIBES.len() {
            if !tribe_active(t, yb) || self.years < self.trade.tribes[t].next {
                continue;
            }
            self.trade.tribes[t].next = self.years + self.rng.range(7.0, 15.0);
            self.tribe_act(t);
        }
    }

    /// 部族が動く：近くの国へ略奪に出るか、交易に来る
    fn tribe_act(&mut self, t: usize) {
        let home = self.tribe_home(t);
        let mut cands: Vec<(f32, usize)> = Vec::new();
        for (i, b) in self.blds.iter().enumerate() {
            if b.alive && b.kind.city() && b.done() && (b.nation as usize) < NATIONS.len() {
                let d = b.center().dist(home);
                if d < 100.0 {
                    cands.push((d, i));
                }
            }
        }
        if cands.is_empty() {
            return;
        }
        cands.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        cands.truncate(4);
        let k = (self.rng.f32() * self.rng.f32() * cands.len() as f32) as usize;
        let bi = cands[k.min(cands.len() - 1)].1;
        let target = self.blds[bi].nation as usize;
        let city = self.bh(bi);
        let size = (2.0 + self.years / 90.0).round() as usize;
        let power = self.tribe_power();
        // 守りが薄い国ほど襲われる。雇われている国は襲わない
        let defense = self.nations[target].mil_power + 1.0;
        let raid_power = size as f32 * power;
        let mut p = TRIBES[t].host * (raid_power / (defense * 0.6)).clamp(0.3, 1.6);
        if self.trade.tribes[t].contract[target] > self.years || (target == self.player && self.protect_player) {
            p = 0.0;
        }
        if self.trade.bands.iter().any(|b| b.tribe == t) {
            p = 0.0;
        }
        let tname = tx(TRIBES[t].name);
        let nname = tx(self.nations[target].name);
        if self.rng.chance(p.min(0.9)) {
            let mut hs = Vec::new();
            for k in 0..size {
                let kind = if TRIBES[t].nomad {
                    if k % 3 == 2 {
                        UnitKind::Archer
                    } else {
                        UnitKind::Chariot
                    }
                } else if k % 2 == 1 {
                    UnitKind::Archer
                } else {
                    UnitKind::Infantry
                };
                let off = v2(self.rng.range(-1.5, 1.5), self.rng.range(-1.5, 1.5));
                let pos = self.passable_near(home + off);
                let h = self.spawn_unit(TRIBE, kind, pos, power);
                self.unit_mut(h).unwrap().tribe = t as u8;
                hs.push(h);
            }
            let goal = self.blds[bi].center();
            self.order_move(&hs, goal, true);
            self.trade.bands.push(Band { tribe: t, target, city, units: hs, phase: 0, until: self.years + 8.0, home });
            self.trade.tribes[t].last = tf!("{}へ略奪に向かった", "Set out to raid {}", "前往劫掠{}", nname);
            if target == self.player {
                let s = tf!("{}の騎兵・兵{}が我が国へ略奪に向かってくる！", "{} riders and warriors ({}) are coming to raid us!", "{}的骑兵、士兵{}名正前来劫掠我国！", tname, size);
                self.push_msg(s.clone(), [1.0, 0.4, 0.3]);
                let text = tf!("{}（{}）。城邑を守れ。傭兵として雇えば襲われなくなる。", "{} ({}). Defend your cities. Hiring them as mercenaries stops their raids.", "{}（{}）。守住城邑。雇其为佣兵即可免遭劫掠。", s, tx(TRIBES[t].desc));
                self.queue_banner(Banner { title: tf!("{}の来寇", "{} Raid", "{}来犯", tname), text, t: 9.0 });
            } else if self.nations[target].home.dist(self.nations[self.player].home) < 80.0 {
                self.push_msg(tf!("{}が{}へ略奪に向かった", "The {} set out to raid {}", "{}前往劫掠{}", tname, nname), [0.95, 0.7, 0.5]);
            }
        } else {
            let mut cv = self.new_caravan(CaravanKind::Tribe(t as u8), home, 1.6, 1);
            cv.dest = Some(city);
            cv.goal = self.blds[bi].center();
            cv.need_path = true;
            cv.stay = 0.0;
            self.trade.caravans.push(cv);
            self.trade.tribes[t].last = tf!("{}へ馬・毛皮を商いに来た", "Came to {} to trade horses and furs", "前往{}贩卖马匹、毛皮", nname);
            if target == self.player {
                self.push_msg(tf!("{}が馬や毛皮を商いにやって来る", "The {} are coming to trade horses and furs", "{}前来贩卖马匹、毛皮", tname), [0.8, 0.9, 0.7]);
            }
        }
    }

    /// 略奪の一団：城を目指し、略奪したら（または時間切れで）引き上げる
    fn update_bands(&mut self) {
        let mut k = 0;
        while k < self.trade.bands.len() {
            let alive: Vec<Handle> = self.trade.bands[k].units.iter().copied().filter(|&h| self.unit(h).is_some()).collect();
            self.trade.bands[k].units = alive.clone();
            let b = &self.trade.bands[k];
            let (t, target, phase, home, city, until) = (b.tribe, b.target, b.phase, b.home, b.city, b.until);
            if alive.is_empty() {
                if phase == 0 {
                    self.trade.tribes[t].last = tf!("{}に撃退された", "Driven off by {}", "被{}击退", tx(self.nations[target].name));
                    if target == self.player {
                        self.push_msg(tf!("{}の来寇を撃退した", "We drove off the {} raid", "击退了{}的来犯", tx(TRIBES[t].name)), [0.5, 1.0, 0.5]);
                    }
                }
                self.trade.bands.remove(k);
                continue;
            }
            if phase == 0 {
                let city_ok = self.bld(city).map_or(false, |c| c.nation as usize == target);
                if self.years > until || !city_ok || !self.nations[target].alive {
                    self.trade.bands[k].phase = 1;
                    self.trade.bands[k].until = self.years + 15.0;
                    self.order_move(&alive, home, false);
                } else {
                    let idle: Vec<Handle> = alive.iter().copied().filter(|&h| self.unit(h).map_or(false, |u| u.order == Order::Idle)).collect();
                    if !idle.is_empty() {
                        self.order_attack(&idle, Target::Bld(city));
                    }
                }
            } else {
                let mut stay = Vec::new();
                for &h in alive.iter() {
                    let u = self.unit(h).unwrap();
                    if u.pos.dist(home) < 4.0 || self.years > until {
                        let i = h.idx as usize;
                        self.kill_unit(i);
                    } else if u.order == Order::Idle {
                        stay.push(h);
                    }
                }
                if !stay.is_empty() {
                    self.order_move(&stay, home, false);
                }
            }
            k += 1;
        }
    }

    /// 部族が城邑を陥とした：占領はせず、蓄えを奪って引き上げる
    pub(super) fn plunder_city(&mut self, bi: usize) {
        let vn = self.blds[bi].nation as usize;
        {
            let b = &mut self.blds[bi];
            b.hp = b.kind.max_hp() * 0.35;
        }
        let n = &mut self.nations[vn];
        let mut lost = [0.0; 3];
        for k in 0..3 {
            lost[k] = n.res[k] * 0.35 * n.tm.plunder;
            n.res[k] -= lost[k];
        }
        n.coin *= 1.0 - 0.3 * n.tm.plunder;
        let mut who = t!("部族", "Raiders", "部族");
        for b in self.trade.bands.iter_mut() {
            if b.target == vn && b.phase == 0 {
                b.phase = 1;
                b.until = self.years + 15.0;
                who = tx(TRIBES[b.tribe].name);
                self.trade.tribes[b.tribe].last = tf!("{}を略奪して引き上げた", "Raided {} and withdrew", "劫掠{}后撤回", tx(self.nations[vn].name));
            }
        }
        let backs: Vec<(Vec<Handle>, V2)> = self.trade.bands.iter().filter(|b| b.target == vn && b.phase == 1).map(|b| (b.units.clone(), b.home)).collect();
        for (hs, home) in backs {
            self.order_move(&hs, home, false);
        }
        if vn == self.player {
            let s = tf!("{}に城邑を略奪された（糧{:.0}・木{:.0}・銅{:.0}と銭の三割を失った）", "The {} raided one of our cities (lost food {:.0}, wood {:.0}, bronze {:.0} and 30% of our coin)", "{}劫掠了我国城邑（损失粮{:.0}、木{:.0}、铜{:.0}及三成钱）", who, lost[0], lost[1], lost[2]);
            self.push_msg(s, [1.0, 0.35, 0.3]);
        } else if self.nations[vn].home.dist(self.nations[self.player].home) < 80.0 {
            self.push_msg(tf!("{}が{}の城邑を略奪した", "The {} raided a city of {}", "{}劫掠了{}的城邑", who, tx(self.nations[vn].name)), [0.95, 0.7, 0.5]);
        }
    }

    /// AI：銭で足りない資源を買い、戦時には傭兵を雇う
    pub fn ai_trade(&mut self, n: usize, rng: &mut Rng) {
        let res = self.nations[n].res;
        let coin = self.nations[n].coin;
        let at_war = (0..NATIONS.len()).any(|o| self.at_war(n, o)) || self.trade.bands.iter().any(|b| b.target == n && b.phase == 0);
        if at_war && coin > self.merc_cost() * 1.2 && rng.chance(0.15) {
            let mut ts: Vec<usize> = (0..TRIBES.len()).filter(|&t| self.can_hire(n, t).is_ok()).collect();
            ts.sort_by(|&a, &b| self.tribe_home(a).dist(self.nations[n].home).partial_cmp(&self.tribe_home(b).dist(self.nations[n].home)).unwrap());
            if let Some(&t) = ts.first() {
                let _ = self.hire_mercs(n, t);
                return;
            }
        }
        let keep = if at_war { 0.0 } else { 80.0 };
        if coin - keep >= self.market_price(n, 2) {
            // 必要量に対して最も乏しい資源
            let need = [200.0, 150.0, 120.0];
            let mut k = 0;
            for j in 1..3 {
                if res[j] / need[j] < res[k] / need[k] {
                    k = j;
                }
            }
            if res[k] < need[k] * 2.0 {
                let _ = self.buy_resource(n, k);
            }
        }
    }
}

//! 記録（セーブ・ロード）：World の状態を読み書きする。
//!
//! 地図は乱数の種から作り直し、その後の変化（伐採・採掘）だけを上書きする。
//! 経路（path）は保存せず、読み込み後に引き直す。

use super::history::Fx;
use super::*;
use crate::ai::AiState;
use crate::save::{RResult, Reader, Writer};

/// 記録形式の版（中身の並びを変えたら上げる）
pub const SAVE_VERSION: u32 = 3;

fn bad(what: &str) -> String {
    tf!("記録が壊れている（{}）", "The save is damaged ({})", "存档已损坏（{}）", what)
}

// ------------------------------------------------------------ 列挙型 ⇔ 数値
fn terrain_u8(t: Terrain) -> u8 {
    match t {
        Terrain::Sea => 0,
        Terrain::Lake => 1,
        Terrain::River => 2,
        Terrain::Plain => 3,
        Terrain::Steppe => 4,
        Terrain::Forest => 5,
        Terrain::Hill => 6,
        Terrain::Mountain => 7,
    }
}
fn terrain_of(v: u8) -> RResult<Terrain> {
    Ok(match v {
        0 => Terrain::Sea,
        1 => Terrain::Lake,
        2 => Terrain::River,
        3 => Terrain::Plain,
        4 => Terrain::Steppe,
        5 => Terrain::Forest,
        6 => Terrain::Hill,
        7 => Terrain::Mountain,
        _ => return Err(bad(t!("地形", "terrain", "地形"))),
    })
}
fn kind_of(v: u8) -> RResult<UnitKind> {
    Ok(match v {
        0 => UnitKind::Worker,
        1 => UnitKind::Infantry,
        2 => UnitKind::Archer,
        3 => UnitKind::Chariot,
        _ => return Err(bad(t!("兵種", "unit type", "兵种"))),
    })
}
fn bkind_u8(k: BKind) -> u8 {
    match k {
        BKind::Capital => 0,
        BKind::Town => 1,
        BKind::Barracks => 2,
        BKind::Farm => 3,
        BKind::Fort => 4,
    }
}
fn bkind_of(v: u8) -> RResult<BKind> {
    Ok(match v {
        0 => BKind::Capital,
        1 => BKind::Town,
        2 => BKind::Barracks,
        3 => BKind::Farm,
        4 => BKind::Fort,
        _ => return Err(bad(t!("建物", "building", "建筑"))),
    })
}

fn w_v2(w: &mut Writer, p: V2) {
    w.f32(p.x);
    w.f32(p.y);
}
fn r_v2(r: &mut Reader) -> RResult<V2> {
    Ok(v2(r.f32()?, r.f32()?))
}
fn w_h(w: &mut Writer, h: Handle) {
    w.u32(h.idx);
    w.u32(h.gen);
}
fn r_h(r: &mut Reader) -> RResult<Handle> {
    Ok(Handle { idx: r.u32()?, gen: r.u32()? })
}
fn w_oh(w: &mut Writer, h: Option<Handle>) {
    w.bool(h.is_some());
    if let Some(h) = h {
        w_h(w, h);
    }
}
fn r_oh(r: &mut Reader) -> RResult<Option<Handle>> {
    Ok(if r.bool()? { Some(r_h(r)?) } else { None })
}
fn w_ov2(w: &mut Writer, p: Option<V2>) {
    w.bool(p.is_some());
    if let Some(p) = p {
        w_v2(w, p);
    }
}
fn r_ov2(r: &mut Reader) -> RResult<Option<V2>> {
    Ok(if r.bool()? { Some(r_v2(r)?) } else { None })
}
fn w_col(w: &mut Writer, c: [f32; 3]) {
    for k in c {
        w.f32(k);
    }
}
fn r_col(r: &mut Reader) -> RResult<[f32; 3]> {
    Ok([r.f32()?, r.f32()?, r.f32()?])
}

fn w_target(w: &mut Writer, t: Target) {
    match t {
        Target::Unit(h) => {
            w.u8(0);
            w_h(w, h);
        }
        Target::Bld(h) => {
            w.u8(1);
            w_h(w, h);
        }
    }
}
fn r_target(r: &mut Reader) -> RResult<Target> {
    Ok(match r.u8()? {
        0 => Target::Unit(r_h(r)?),
        1 => Target::Bld(r_h(r)?),
        _ => return Err(bad(t!("攻撃目標", "attack target", "攻击目标"))),
    })
}
fn w_order(w: &mut Writer, o: Order) {
    match o {
        Order::Idle => w.u8(0),
        Order::Move(p) => {
            w.u8(1);
            w_v2(w, p);
        }
        Order::AttackMove(p) => {
            w.u8(2);
            w_v2(w, p);
        }
        Order::Attack(t) => {
            w.u8(3);
            w_target(w, t);
        }
        Order::Gather { src, phase } => {
            w.u8(4);
            match src {
                Src::Farm(h) => {
                    w.u8(0);
                    w_h(w, h);
                }
                Src::Forest(x, y) => {
                    w.u8(1);
                    w.i32(x);
                    w.i32(y);
                }
                Src::Mine(m) => {
                    w.u8(2);
                    w.usize(m);
                }
            }
            w.u8(phase);
        }
        Order::Build(h) => {
            w.u8(5);
            w_h(w, h);
        }
    }
}
fn r_order(r: &mut Reader, mines: usize) -> RResult<Order> {
    Ok(match r.u8()? {
        0 => Order::Idle,
        1 => Order::Move(r_v2(r)?),
        2 => Order::AttackMove(r_v2(r)?),
        3 => Order::Attack(r_target(r)?),
        4 => {
            let src = match r.u8()? {
                0 => Src::Farm(r_h(r)?),
                1 => Src::Forest(r.i32()?, r.i32()?),
                2 => {
                    let m = r.usize()?;
                    if m >= mines {
                        return Err(bad(t!("銅山", "mine", "铜矿")));
                    }
                    Src::Mine(m)
                }
                _ => return Err(bad(t!("採集", "gathering", "采集"))),
            };
            Order::Gather { src, phase: r.u8()? }
        }
        5 => Order::Build(r_h(r)?),
        _ => return Err(bad(t!("命令", "order", "命令"))),
    })
}

/// 史実イベントに現れる名前（君主家・人物）を 'static な文字列に戻す
fn static_house(s: &str) -> RResult<&'static str> {
    if let Some(h) = INITIAL_HOUSE.iter().find(|h| **h == s) {
        return Ok(h);
    }
    for e in HISTORY {
        for fx in e.fx {
            if let Fx::House(h) = fx {
                if *h == s {
                    return Ok(h);
                }
            }
        }
    }
    if s.is_empty() {
        return Ok("");
    }
    Err(bad(t!("君主家", "ruling house", "君主家")))
}
fn static_figure(s: &str) -> RResult<&'static str> {
    for e in HISTORY {
        for fx in e.fx {
            if let Fx::Figure(n, ..) = fx {
                if *n == s {
                    return Ok(n);
                }
            }
        }
    }
    Err(bad(t!("人物", "figure", "人物")))
}

fn w_banner(w: &mut Writer, b: &Banner) {
    w.str(&b.title);
    w.str(&b.text);
    w.f32(b.t);
}
fn r_banner(r: &mut Reader) -> RResult<Banner> {
    Ok(Banner { title: r.str()?, text: r.str()?, t: r.f32()? })
}

impl World {
    /// World の状態を書き出す
    pub fn save_to(&self, w: &mut Writer) {
        let n = MAP_W * MAP_H;
        // ---- 地図（地形・森の残量・銅山の残量）
        let terr: Vec<u8> = self.map.terrain.iter().map(|&t| terrain_u8(t)).collect();
        w.rle(&terr);
        let woods: Vec<usize> = (0..n).filter(|&i| self.map.terrain[i] == Terrain::Forest && self.map.wood[i] != 120.0).collect();
        w.usize(woods.len());
        for i in woods {
            w.usize(i);
            w.f32(self.map.wood[i]);
        }
        w.usize(self.map.mines.len());
        for m in self.map.mines.iter() {
            w.i32(m.x);
            w.i32(m.y);
            w.f32(m.amount);
        }
        // ---- ユニット（死んだ枠も番号を保つために書く）
        w.usize(self.units.len());
        for u in self.units.iter() {
            w.u32(u.gen);
            w.bool(u.alive);
            if !u.alive {
                continue;
            }
            w.u8(u.nation);
            w.u8(u.kind as u8);
            w_v2(w, u.pos);
            w.f32(u.hp);
            w.f32(u.power);
            w_order(w, u.order);
            w_ov2(w, u.resume);
            w.f32(u.cooldown);
            w.usize(u.carry_kind);
            w.f32(u.carry);
            w.f32(u.scan_t);
            w_v2(w, u.dir);
            w_oh(w, u.last_attacker);
            w.u8(u.tribe);
            w.f32(u.merc_until);
            w.bool(u.divine);
            w.u8(u.hero);
            w.usize(u.build_queue.len());
            for &h in u.build_queue.iter() {
                w_h(w, h);
            }
        }
        w.usize(self.free_units.len());
        for &i in self.free_units.iter() {
            w.u32(i);
        }
        // ---- 建物
        w.usize(self.blds.len());
        for b in self.blds.iter() {
            w.u32(b.gen);
            w.bool(b.alive);
            if !b.alive {
                continue;
            }
            w.u8(b.nation);
            w.u8(bkind_u8(b.kind));
            w.i32(b.tx);
            w.i32(b.ty);
            w.f32(b.hp);
            w.f32(b.progress);
            w.usize(b.queue.len());
            for &k in b.queue.iter() {
                w.u8(k as u8);
            }
            w.f32(b.queue_t);
            w_ov2(w, b.rally);
            w.f32(b.cooldown);
        }
        w.usize(self.free_blds.len());
        for &i in self.free_blds.iter() {
            w.u32(i);
        }
        // ---- 国
        w.usize(self.nations.len());
        for nt in self.nations.iter() {
            w.bool(nt.alive);
            w.bool(nt.appeared);
            for k in 0..3 {
                w.f32(nt.res[k]);
                w.f32(nt.earned[k]);
                w.f32(nt.income[k]);
            }
            w.f32(nt.aggression);
            w.f32(nt.hegemon_cd);
            w_oh(w, nt.capital);
            w_v2(w, nt.home);
            w.str(nt.house);
            w.usize(nt.figures.len());
            for f in nt.figures.iter() {
                w.str(f.name);
                w.f32(f.eco);
                w.f32(f.mil);
                w.f32(f.until);
            }
            w.f32(nt.coin);
            w.f32(nt.coin_earned);
            w.f32(nt.coin_income);
            w.u8(nt.tax);
            w.u32(nt.cap_bonus);
        }
        // ---- 外交
        let nn = NATIONS.len();
        for a in 0..nn {
            for b in 0..nn {
                w.bool(self.war[a][b]);
                w.f32(self.war_start[a][b]);
                w.f32(self.truce[a][b]);
            }
            w.bool(self.peace_offer[a]);
            w.f32(self.offer_cd[a]);
        }
        w.usize(self.peace_asks.len());
        for q in self.peace_asks.iter() {
            w.usize(q.from);
            w.f32(q.years);
            w.str(&q.why);
        }
        w.usize(self.coalitions.len());
        for c in self.coalitions.iter() {
            w.u8(if c.kind == CoalKind::Hegemon { 0 } else { 1 });
            w.usize(c.target);
            w.usize(c.leader);
            w.usize(c.members.len());
            for &m in c.members.iter() {
                w.usize(m);
            }
            w.f32(c.end);
        }
        w.f32(self.hezong_next);
        // ---- 時間・全体
        w.f32(self.years);
        w.u8(if self.era == Era::Spring { 0 } else { 1 });
        w.usize(self.player);
        w.f32(self.time);
        w.f32(self.stat_t);
        w.f32(self.income_t);
        w.f32(self.year_tick);
        w.u64(self.rng.0);
        w.bool(self.protect_player);
        w.u32(self.player_orders);
        w.bool(self.ai_player);
        w.bool(self.heroic);
        w.bool(self.game_over.is_some());
        if let Some((win, t)) = &self.game_over {
            w.bool(*win);
            w.str(t);
        }
        w.bool(self.descent.is_some());
        if let Some(d) = &self.descent {
            w.f32(d.start);
            w.bool(d.revived);
            w.usize(d.total);
            w.usize(d.taken);
        }
        // ---- 記録・史書
        w.usize(self.msgs.len());
        for m in self.msgs.iter() {
            w.str(&m.text);
            w.f32(m.time);
            w_col(w, m.color);
        }
        w.bool(self.banner.is_some());
        if let Some(b) = &self.banner {
            w_banner(w, b);
        }
        w.usize(self.banner_queue.len());
        for b in self.banner_queue.iter() {
            w_banner(w, b);
        }
        // 史書は年表の先頭から hist_i 件。滅んで起こらなかったかだけを書く
        w.usize(self.hist_i);
        for c in self.chronicle.iter() {
            w.bool(c.skipped);
        }
        // ---- AI
        w.usize(self.ai.len());
        for a in self.ai.iter() {
            w.u64(a.rng.0);
            w.f32(a.think_t);
            w.f32(a.war_t);
            w.f32(a.attack_t);
            w.f32(a.aggr);
        }
        // ---- 交易
        let tr = &self.trade;
        w.usize(tr.caravans.len());
        for c in tr.caravans.iter() {
            match c.kind {
                CaravanKind::Merchant => w.u8(0),
                CaravanKind::Western(r) => {
                    w.u8(1);
                    w.u8(r);
                }
                CaravanKind::Tribe(t) => {
                    w.u8(2);
                    w.u8(t);
                }
            }
            w_v2(w, c.pos);
            w_v2(w, c.dir);
            w_oh(w, c.dest);
            w_v2(w, c.goal);
            w.f32(c.stay);
            w.u8(c.trips);
            w.u8(c.max_trips);
            w.bool(c.leaving);
            w.bool(c.gone);
            w.f32(c.value);
        }
        w.usize(tr.tribes.len());
        for t in tr.tribes.iter() {
            w.f32(t.next);
            w.usize(t.contract.len());
            for &c in t.contract.iter() {
                w.f32(c);
            }
            w.str(&t.last);
        }
        w.usize(tr.bands.len());
        for b in tr.bands.iter() {
            w.usize(b.tribe);
            w.usize(b.target);
            w_h(w, b.city);
            w.usize(b.units.len());
            for &h in b.units.iter() {
                w_h(w, h);
            }
            w.u8(b.phase);
            w.f32(b.until);
            w_v2(w, b.home);
        }
        w.usize(tr.target);
        w.usize(tr.target_w);
        w.f32(tr.spawn_t);
        w.f32(tr.band_t);
    }

    /// 記録から World を復元する（地図は種から作り直す）
    pub fn load_from(seed: u32, r: &mut Reader) -> RResult<World> {
        let mut w = World::new(seed);
        let n = MAP_W * MAP_H;
        // ---- 地図
        let terr = r.rle(n)?;
        for i in 0..n {
            let t = terrain_of(terr[i])?;
            w.map.terrain[i] = t;
            w.map.wood[i] = if t == Terrain::Forest { 120.0 } else { 0.0 };
        }
        let nw = r.len(n)?;
        for _ in 0..nw {
            let i = r.usize()?;
            let v = r.f32()?;
            if i >= n {
                return Err(bad(t!("森", "forest", "森林")));
            }
            w.map.wood[i] = v;
        }
        let nm = r.len(100_000)?;
        let mut mines = Vec::with_capacity(nm);
        for _ in 0..nm {
            mines.push(Mine { x: r.i32()?, y: r.i32()?, amount: r.f32()? });
        }
        w.map.mines = mines;
        let nmines = w.map.mines.len();
        let nn_all = w.nations.len();
        // ---- ユニット
        let nu = r.len(1_000_000)?;
        let mut units = Vec::with_capacity(nu);
        for _ in 0..nu {
            let gen = r.u32()?;
            let alive = r.bool()?;
            let mut u = Unit {
                gen,
                alive,
                nation: 0,
                kind: UnitKind::Worker,
                pos: v2(0.0, 0.0),
                hp: 0.0,
                power: 1.0,
                order: Order::Idle,
                resume: None,
                path: Vec::new(),
                path_i: 0,
                path_goal: (-9, -9),
                path_pending: false,
                path_done: false,
                cooldown: 0.0,
                carry_kind: 0,
                carry: 0.0,
                scan_t: 0.0,
                dir: v2(0.0, 1.0),
                flash: 0.0,
                selected: false,
                last_attacker: None,
                tribe: NONE,
                merc_until: 0.0,
                divine: false,
                hero: NONE,
                build_queue: std::collections::VecDeque::new(),
            };
            if alive {
                u.nation = r.u8()?;
                if u.nation as usize >= nn_all {
                    return Err(bad(t!("ユニットの国", "unit state", "单位所属国")));
                }
                u.kind = kind_of(r.u8()?)?;
                u.pos = r_v2(r)?;
                u.hp = r.f32()?;
                u.power = r.f32()?;
                u.order = r_order(r, nmines)?;
                u.resume = r_ov2(r)?;
                u.cooldown = r.f32()?;
                u.carry_kind = r.usize()?.min(2);
                u.carry = r.f32()?;
                u.scan_t = r.f32()?;
                u.dir = r_v2(r)?;
                u.last_attacker = r_oh(r)?;
                u.tribe = r.u8()?;
                u.merc_until = r.f32()?;
                u.divine = r.bool()?;
                u.hero = r.u8()?;
                let k = r.len(1000)?;
                for _ in 0..k {
                    u.build_queue.push_back(r_h(r)?);
                }
            }
            units.push(u);
        }
        w.units = units;
        let nf = r.len(1_000_000)?;
        w.free_units = (0..nf).map(|_| r.u32()).collect::<RResult<_>>()?;
        // ---- 建物
        let nb = r.len(1_000_000)?;
        let mut blds = Vec::with_capacity(nb);
        for _ in 0..nb {
            let gen = r.u32()?;
            let alive = r.bool()?;
            let mut b = Building {
                gen,
                alive,
                nation: 0,
                kind: BKind::Farm,
                tx: 0,
                ty: 0,
                hp: 0.0,
                progress: 1.0,
                queue: std::collections::VecDeque::new(),
                queue_t: 0.0,
                rally: None,
                cooldown: 0.0,
            };
            if alive {
                b.nation = r.u8()?;
                if b.nation as usize >= nn_all {
                    return Err(bad(t!("建物の国", "building state", "建筑所属国")));
                }
                b.kind = bkind_of(r.u8()?)?;
                b.tx = r.i32()?;
                b.ty = r.i32()?;
                if !Map::inb(b.tx, b.ty) || !Map::inb(b.tx + 1, b.ty + 1) {
                    return Err(bad(t!("建物の位置", "building position", "建筑位置")));
                }
                b.hp = r.f32()?;
                b.progress = r.f32()?;
                let q = r.len(64)?;
                for _ in 0..q {
                    b.queue.push_back(kind_of(r.u8()?)?);
                }
                b.queue_t = r.f32()?;
                b.rally = r_ov2(r)?;
                b.cooldown = r.f32()?;
            }
            blds.push(b);
        }
        w.blds = blds;
        let nf = r.len(1_000_000)?;
        w.free_blds = (0..nf).map(|_| r.u32()).collect::<RResult<_>>()?;
        // ---- 国
        let nnat = r.len(1000)?;
        if nnat != w.nations.len() {
            return Err(t!("記録の国の数が合わない（別の版の記録）", "The number of states does not match (save from another version)", "存档中的国家数量不符（其他版本的存档）").into());
        }
        for i in 0..nnat {
            let alive = r.bool()?;
            let appeared = r.bool()?;
            let mut res = [0.0; 3];
            let mut earned = [0.0; 3];
            let mut income = [0.0; 3];
            for k in 0..3 {
                res[k] = r.f32()?;
                earned[k] = r.f32()?;
                income[k] = r.f32()?;
            }
            let aggression = r.f32()?;
            let hegemon_cd = r.f32()?;
            let capital = r_oh(r)?;
            let home = r_v2(r)?;
            let house = static_house(&r.str()?)?;
            let nfig = r.len(1000)?;
            let mut figures = Vec::with_capacity(nfig);
            for _ in 0..nfig {
                let name = static_figure(&r.str()?)?;
                figures.push(Person { name, eco: r.f32()?, mil: r.f32()?, until: r.f32()? });
            }
            let nt = &mut w.nations[i];
            nt.alive = alive;
            nt.appeared = appeared;
            nt.res = res;
            nt.earned = earned;
            nt.income = income;
            nt.aggression = aggression;
            nt.hegemon_cd = hegemon_cd;
            nt.capital = capital;
            nt.home = home;
            nt.house = house;
            nt.figures = figures;
            nt.coin = r.f32()?;
            nt.coin_earned = r.f32()?;
            nt.coin_income = r.f32()?;
            nt.tax = r.u8()?.min(2);
            nt.cap_bonus = r.u32()?.min(10_000);
        }
        // ---- 外交
        let nn = NATIONS.len();
        for a in 0..nn {
            for b in 0..nn {
                w.war[a][b] = r.bool()?;
                w.war_start[a][b] = r.f32()?;
                w.truce[a][b] = r.f32()?;
            }
            w.peace_offer[a] = r.bool()?;
            w.offer_cd[a] = r.f32()?;
        }
        let np = r.len(1000)?;
        w.peace_asks.clear();
        for _ in 0..np {
            let from = r.usize()?;
            if from >= nn {
                return Err(bad(t!("和睦の申し入れ", "peace offer", "议和请求")));
            }
            w.peace_asks.push_back(PeaceAsk { from, years: r.f32()?, why: r.str()? });
        }
        let nc = r.len(1000)?;
        let mut coalitions = Vec::with_capacity(nc);
        for _ in 0..nc {
            let kind = if r.u8()? == 0 { CoalKind::Hegemon } else { CoalKind::Hezong };
            let target = r.usize()?;
            let leader = r.usize()?;
            let k = r.len(1000)?;
            let members: Vec<usize> = (0..k).map(|_| r.usize()).collect::<RResult<_>>()?;
            if target >= nn || leader >= nn || members.iter().any(|&m| m >= nn) {
                return Err(bad(t!("連合", "coalition", "联军")));
            }
            coalitions.push(Coalition { kind, target, leader, members, end: r.f32()? });
        }
        w.coalitions = coalitions;
        w.hezong_next = r.f32()?;
        // ---- 時間・全体
        w.years = r.f32()?;
        w.era = if r.u8()? == 0 { Era::Spring } else { Era::Warring };
        w.player = r.usize()?;
        if w.player >= nn {
            return Err(bad(t!("自国", "player state", "己方国家")));
        }
        w.time = r.f32()?;
        w.stat_t = r.f32()?;
        w.income_t = r.f32()?;
        w.year_tick = r.f32()?;
        w.rng.0 = r.u64()?;
        w.protect_player = r.bool()?;
        w.player_orders = r.u32()?;
        w.ai_player = r.bool()?;
        w.heroic = r.bool()?;
        w.game_over = if r.bool()? { Some((r.bool()?, r.str()?)) } else { None };
        w.descent = if r.bool()? { Some(Descent { start: r.f32()?, revived: r.bool()?, total: r.usize()?, taken: r.usize()? }) } else { None };
        // ---- 記録・史書
        let nmsg = r.len(10_000)?;
        let mut msgs = Vec::with_capacity(nmsg);
        for _ in 0..nmsg {
            msgs.push(Msg { text: r.str()?, time: r.f32()?, color: r_col(r)? });
        }
        w.msgs = msgs;
        w.banner = if r.bool()? { Some(r_banner(r)?) } else { None };
        let nq = r.len(10_000)?;
        w.banner_queue.clear();
        for _ in 0..nq {
            w.banner_queue.push_back(r_banner(r)?);
        }
        w.hist_i = r.len(HISTORY.len())?;
        w.chronicle.clear();
        for e in HISTORY.iter().take(w.hist_i) {
            let skipped = r.bool()?;
            w.chronicle.push(Chron { bc: e.bc, nation: e.nation, kind: e.kind, title: e.title, text: e.text, skipped });
        }
        // ---- AI
        let na = r.len(1000)?;
        if na != w.ai.len() {
            return Err(bad("AI"));
        }
        for i in 0..na {
            let mut a = AiState::new(0);
            a.rng.0 = r.u64()?;
            a.think_t = r.f32()?;
            a.war_t = r.f32()?;
            a.attack_t = r.f32()?;
            a.aggr = r.f32()?;
            w.ai[i] = a;
        }
        // ---- 交易
        let ncv = r.len(100_000)?;
        let mut caravans = Vec::with_capacity(ncv);
        for _ in 0..ncv {
            let kind = match r.u8()? {
                0 => CaravanKind::Merchant,
                1 => CaravanKind::Western(r.u8()?),
                2 => CaravanKind::Tribe(r.u8()?),
                _ => return Err(bad(t!("隊商", "caravan", "商队"))),
            };
            let pos = r_v2(r)?;
            let dir = r_v2(r)?;
            let dest = r_oh(r)?;
            let goal = r_v2(r)?;
            let stay = r.f32()?;
            let trips = r.u8()?;
            let max_trips = r.u8()?;
            let leaving = r.bool()?;
            let gone = r.bool()?;
            let value = r.f32()?;
            // 移動中の隊商は経路を引き直す
            caravans.push(Caravan { kind, pos, dir, path: Vec::new(), path_i: 0, need_path: stay <= 0.0, dest, goal, stay, trips, max_trips, leaving, gone, value });
        }
        w.trade.caravans = caravans;
        let nt = r.len(1000)?;
        if nt != w.trade.tribes.len() {
            return Err(bad(t!("部族", "tribe", "部族")));
        }
        for t in 0..nt {
            let next = r.f32()?;
            let k = r.len(1000)?;
            let contract: Vec<f32> = (0..k).map(|_| r.f32()).collect::<RResult<_>>()?;
            let last = r.str()?;
            w.trade.tribes[t] = TribeState { next, contract, last };
        }
        let nbd = r.len(10_000)?;
        let mut bands = Vec::with_capacity(nbd);
        for _ in 0..nbd {
            let tribe = r.usize()?;
            let target = r.usize()?;
            if tribe >= TRIBES.len() || target >= nn {
                return Err(bad(t!("略奪団", "raiders", "劫掠团")));
            }
            let city = r_h(r)?;
            let k = r.len(10_000)?;
            let units: Vec<Handle> = (0..k).map(|_| r_h(r)).collect::<RResult<_>>()?;
            bands.push(Band { tribe, target, city, units, phase: r.u8()?, until: r.f32()?, home: r_v2(r)? });
        }
        w.trade.bands = bands;
        w.trade.target = r.usize()?;
        w.trade.target_w = r.usize()?;
        w.trade.spawn_t = r.f32()?;
        w.trade.band_t = r.f32()?;

        // ---- 導出される状態を作り直す
        w.shots.clear();
        w.path_queue.clear();
        w.terrain_dirty.clear();
        for i in 0..n {
            w.cost[i] = w.map.terrain[i].move_cost();
            w.bmap[i] = 0;
        }
        for (bi, b) in w.blds.iter().enumerate() {
            if !b.alive {
                continue;
            }
            for dy in 0..2 {
                for dx in 0..2 {
                    let i = Map::idx(b.tx + dx, b.ty + dy);
                    w.bmap[i] = bi as u32 + 1;
                    if b.kind.blocks() {
                        w.cost[i] = f32::INFINITY;
                    }
                }
            }
        }
        w.recompute_territory();
        w.refresh_mods();
        w.refresh_traits();
        w.update_stats();
        w.rebuild_grid();
        Ok(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 長く進めた世界を書き出して読み戻すと、同じ状態から同じように進む
    #[test]
    fn save_roundtrip() {
        let seed = 777;
        let mut a = World::new(seed);
        a.player = 4;
        while a.years < 120.0 {
            a.step();
            if a.game_over.is_some() {
                a.game_over = None;
            }
        }
        let mut wr = Writer::new();
        a.save_to(&mut wr);
        let bytes = wr.buf;
        println!("記録の大きさ {} バイト（ユニット {}・建物 {}）", bytes.len(), a.units.iter().filter(|u| u.alive).count(), a.blds.iter().filter(|b| b.alive).count());
        let mut b = World::load_from(seed, &mut Reader::new(&bytes)).expect("読み込めない");
        // 書き戻すと同じ内容
        let mut wr2 = Writer::new();
        b.save_to(&mut wr2);
        assert_eq!(bytes, wr2.buf, "読み戻した世界を書き直すと内容が変わる");
        assert_eq!(a.year_bc(), b.year_bc());
        assert_eq!(a.chronicle.len(), b.chronicle.len());
        for i in 0..NATIONS.len() {
            assert_eq!(a.nations[i].alive, b.nations[i].alive);
            assert_eq!(a.nations[i].house, b.nations[i].house);
            assert_eq!(a.nations[i].cities, b.nations[i].cities);
        }
        assert_eq!(a.owner, b.owner);
        // 読み込んだ世界がそのまま進む
        for _ in 0..3000 {
            b.step();
        }
        assert!(b.years > a.years);
        // 壊れた記録は読み込みを拒む
        assert!(World::load_from(seed, &mut Reader::new(&bytes[..bytes.len() / 2])).is_err());
    }
}

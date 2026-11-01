//! コンピュータ国家の思考

use crate::geo::{Map, NATIONS};
use crate::rng::Rng;
use crate::world::*;

pub struct AiState {
    pub rng: Rng,
    pub(crate) think_t: f32,
    pub(crate) war_t: f32,
    pub(crate) attack_t: f32,
    /// 好戦性（国ごとの性格）
    pub aggr: f32,
}

impl AiState {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let aggr = rng.range(0.6, 1.4);
        let think_t = rng.range(0.0, 1.0);
        AiState { rng, think_t, war_t: 10.0 + think_t * 5.0, attack_t: 0.0, aggr }
    }
}

pub fn update_all(w: &mut World, dt: f32) {
    let mut ais = std::mem::take(&mut w.ai);
    for n in 0..ais.len() {
        if (n == w.player && !w.ai_player) || !w.nations[n].alive {
            continue;
        }
        let a = &mut ais[n];
        a.think_t -= dt;
        a.war_t -= dt;
        a.attack_t -= dt;
        if a.think_t <= 0.0 {
            a.think_t = 1.0 + a.rng.f32() * 0.4;
            think(w, n, a);
        }
    }
    w.ai = ais;
}

fn find_spot(w: &World, n: usize, kind: BKind, center: V2, rmin: f32, rmax: f32, rng: &mut Rng) -> Option<(i32, i32)> {
    for k in 0..60 {
        let ang = rng.range(0.0, std::f32::consts::TAU);
        // 見つからなければ範囲を広げる
        let d = rng.range(rmin, rmax + (k / 20) as f32 * 3.0);
        let tx = (center.x + ang.cos() * d) as i32 - 1;
        let ty = (center.y + ang.sin() * d) as i32 - 1;
        if w.can_place(n, kind, tx, ty).is_ok() {
            return Some((tx, ty));
        }
    }
    None
}

fn think(w: &mut World, n: usize, a: &mut AiState) {
    let res = w.nations[n].res;
    let home = w.nations[n].home;
    let mut workers: Vec<(Handle, Order, V2)> = Vec::new();
    let mut soldiers: Vec<(Handle, Order, V2, f32)> = Vec::new();
    let mut gathering = [0usize; 3];
    for (i, u) in w.units.iter().enumerate() {
        if !u.alive || u.nation as usize != n {
            continue;
        }
        let h = Handle { idx: i as u32, gen: u.gen };
        if u.kind.military() {
            soldiers.push((h, u.order, u.pos, u.power));
        } else {
            if let Order::Gather { src, .. } = u.order {
                gathering[src.res()] += 1;
            }
            workers.push((h, u.order, u.pos));
        }
    }
    let mut cities = Vec::new();
    let mut barracks = Vec::new();
    let mut unfinished = Vec::new();
    let mut farms = 0;
    for (i, b) in w.blds.iter().enumerate() {
        if !b.alive || b.nation as usize != n {
            continue;
        }
        let h = Handle { idx: i as u32, gen: b.gen };
        if !b.done() {
            unfinished.push(h);
        }
        match b.kind {
            BKind::Capital | BKind::Town => {
                if b.done() {
                    cities.push(h)
                }
            }
            BKind::Barracks => {
                if b.done() {
                    barracks.push(h)
                }
            }
            BKind::Farm => farms += 1,
            BKind::Fort => {}
        }
    }
    if cities.is_empty() {
        return;
    }
    let nat = &w.nations[n];
    let (wc, mc) = (nat.worker_cap(), nat.mil_cap());
    let (nw, nm) = (nat.workers + nat.q_workers, nat.soldiers + nat.q_soldiers);
    let at_war_any = (0..NATIONS.len()).any(|o| w.at_war(n, o));
    w.ai_trade(n, &mut a.rng);

    // ---- 民の訓練
    if nw < wc && res[0] >= 50.0 + if at_war_any { 40.0 } else { 0.0 } {
        if let Some(&c) = cities.iter().find(|&&c| w.bld(c).map_or(false, |b| b.queue.is_empty())) {
            let _ = w.enqueue(c, UnitKind::Worker);
        }
    }

    // ---- 未完成の建物に建設者を
    for &bh in unfinished.iter() {
        let builders = workers.iter().filter(|x| x.1 == Order::Build(bh)).count();
        if builders == 0 {
            let bc = w.bld(bh).unwrap().center();
            let mut best: Option<(f32, usize)> = None;
            for (k, x) in workers.iter().enumerate() {
                if matches!(x.1, Order::Build(_)) {
                    continue;
                }
                let d = x.2.dist(bc);
                if best.map_or(true, |b| d < b.0) {
                    best = Some((d, k));
                }
            }
            if let Some((_, k)) = best {
                w.order_build(&[workers[k].0], bh);
                workers[k].1 = Order::Build(bh);
            }
        }
    }

    // ---- 兵営
    let res = w.nations[n].res;
    let has_barracks_site = w.blds.iter().any(|b| b.alive && b.nation as usize == n && b.kind == BKind::Barracks);
    if !has_barracks_site && res[1] >= 150.0 && res[2] >= 30.0 {
        if let Some((tx, ty)) = find_spot(w, n, BKind::Barracks, home, 3.0, 7.0, &mut a.rng) {
            if let Ok(bh) = w.place_building(n, BKind::Barracks, tx, ty) {
                if let Some(x) = workers.iter_mut().find(|x| !matches!(x.1, Order::Build(_))) {
                    w.order_build(&[x.0], bh);
                    x.1 = Order::Build(bh);
                }
            }
        }
    }

    // ---- 資源配分
    let total = workers.len().max(1) as f32;
    let mut want = [0.45, 0.32, 0.23];
    let res = w.nations[n].res;
    for k in 0..3 {
        if res[k] > 700.0 {
            want[k] *= 0.4;
        }
    }
    if w.nations[n].upkeep > w.nations[n].income[0] * 0.5 {
        want[0] += 0.15;
    }
    let sumw: f32 = want.iter().sum();
    // 田が足りなければ作る
    let food_target = (total * want[0] / sumw).round() as usize;
    if farms < food_target.max(1) && res[1] >= 60.0 && unfinished.len() < 3 {
        let c = cities[a.rng.below(cities.len() as u32) as usize];
        let cc = w.bld(c).unwrap().center();
        if let Some((tx, ty)) = find_spot(w, n, BKind::Farm, cc, 2.5, 6.0, &mut a.rng) {
            if let Ok(bh) = w.place_building(n, BKind::Farm, tx, ty) {
                let bc = w.bld(bh).unwrap().center();
                let mut best: Option<(f32, usize)> = None;
                for (k, x) in workers.iter().enumerate() {
                    if matches!(x.1, Order::Build(_)) {
                        continue;
                    }
                    let d = x.2.dist(bc);
                    if best.map_or(true, |b| d < b.0) {
                        best = Some((d, k));
                    }
                }
                if let Some((_, k)) = best {
                    w.order_build(&[workers[k].0], bh);
                    workers[k].1 = Order::Build(bh);
                }
            }
        }
    }
    for x in workers.iter() {
        if x.1 != Order::Idle {
            continue;
        }
        // 不足している資源
        let mut best_k = 0;
        let mut best_def = f32::MIN;
        for k in 0..3 {
            let def = want[k] / sumw * total - gathering[k] as f32;
            if def > best_def {
                best_def = def;
                best_k = k;
            }
        }
        let p = x.2;
        let src = match best_k {
            0 => w.free_farm(n, p).map(Src::Farm),
            2 => w.nearest_mine_for(Some(n), p, 22.0).map(Src::Mine),
            _ => None,
        }
        .or_else(|| w.nearest_forest_for(Some(n), p, 16).map(|(x, y)| Src::Forest(x, y)))
        .or_else(|| w.nearest_mine_for(Some(n), p, 30.0).map(Src::Mine));
        if let Some(s) = src {
            gathering[s.res()] += 1;
            w.order_gather(&[x.0], s);
        }
    }

    // ---- 要塞：資源が余っているか戦時なら、城邑の近くに築く
    let res = w.nations[n].res;
    let fort_c = BKind::Fort.cost();
    let rich = res[0] >= fort_c[0] * 2.0 && res[1] >= fort_c[1] * 2.0 && res[2] >= fort_c[2] * 2.0;
    let can_pay = res[0] >= fort_c[0] && res[1] >= fort_c[1] && res[2] >= fort_c[2];
    let forts_building = unfinished.iter().any(|&h| w.bld(h).map_or(false, |b| b.kind == BKind::Fort));
    if can_pay && (rich || at_war_any) && !forts_building && w.fort_count(n) < w.nations[n].fort_cap() && a.rng.chance(0.15) {
        let c = cities[a.rng.below(cities.len() as u32) as usize];
        let cc = w.bld(c).unwrap().center();
        if let Some((tx, ty)) = find_spot(w, n, BKind::Fort, cc, 4.0, 8.0, &mut a.rng) {
            if let Ok(bh) = w.place_building(n, BKind::Fort, tx, ty) {
                if let Some(x) = workers.iter_mut().find(|x| !matches!(x.1, Order::Build(_))) {
                    w.order_build(&[x.0], bh);
                    x.1 = Order::Build(bh);
                }
            }
        }
    }

    // ---- 拡張（築城）
    let res = w.nations[n].res;
    let max_cities = if w.era == Era::Spring { 3 } else { 5 } + if w.nations[n].bias > 1.2 { 2 } else { 0 };
    let towns_building = unfinished.iter().any(|&h| w.bld(h).map_or(false, |b| b.kind.city()));
    if cities.len() < max_cities && !towns_building && res[1] >= 250.0 && res[0] >= 100.0 && res[2] >= 80.0 && a.rng.chance(0.3) {
        let c = cities[a.rng.below(cities.len() as u32) as usize];
        let cc = w.bld(c).unwrap().center();
        if let Some((tx, ty)) = find_spot(w, n, BKind::Town, cc, 11.0, 21.0, &mut a.rng) {
            if let Ok(bh) = w.place_building(n, BKind::Town, tx, ty) {
                let mut hs = Vec::new();
                for x in workers.iter_mut() {
                    if hs.len() >= 2 {
                        break;
                    }
                    if !matches!(x.1, Order::Build(_)) {
                        hs.push(x.0);
                        x.1 = Order::Build(bh);
                    }
                }
                w.order_build(&hs, bh);
            }
        }
    }

    // ---- 軍の訓練
    let nat = &w.nations[n];
    let res = nat.res;
    let food_ok = res[0] > 350.0 || nat.income[0] > nat.upkeep * 1.3 + 0.3;
    if nm < mc && res[0] >= 100.0 && food_ok && !barracks.is_empty() {
        let b = barracks[a.rng.below(barracks.len() as u32) as usize];
        if w.bld(b).map_or(false, |x| x.queue.len() < 2) {
            let r = a.rng.f32();
            let k = if r < 0.45 {
                UnitKind::Infantry
            } else if r < 0.75 {
                UnitKind::Archer
            } else {
                UnitKind::Chariot
            };
            if w.enqueue(b, k).is_err() {
                let _ = w.enqueue(b, UnitKind::Infantry);
            }
        }
    }

    // ---- 能力の鍛錬（上限に達したら質で補う）
    let res = w.nations[n].res;
    if res[0] > 300.0 && res[2] > 150.0 && (nm >= mc || res[0] > 700.0) {
        if let Some(s) = soldiers.iter().min_by(|x, y| x.3.partial_cmp(&y.3).unwrap()) {
            let _ = w.upgrade_unit(s.0);
        }
    }
    if res[0] > 350.0 && res[1] > 250.0 && nw >= wc {
        if let Some(x) = workers.iter().min_by(|x, y| w.unit(x.0).unwrap().power.partial_cmp(&w.unit(y.0).unwrap().power).unwrap()) {
            let _ = w.upgrade_unit(x.0);
        }
    }

    // ---- 外交
    if a.war_t <= 0.0 {
        a.war_t = 6.0 + a.rng.f32() * 4.0;
        diplomacy(w, n, a, soldiers.len());
    }

    // ---- 軍の指揮
    command_army(w, n, a, &soldiers);

    // ---- 英霊召喚（民を生贄にするので最後に）
    w.ai_summon(n, &mut a.rng);
}

fn diplomacy(w: &mut World, n: usize, a: &mut AiState, nsold: usize) {
    let my = w.nations[n].strength;
    let years = w.years;
    // 和睦
    for e in 0..NATIONS.len() {
        if !w.at_war(n, e) || w.coalition_locked(n, e) {
            continue;
        }
        let dur = years - w.war_start[n][e];
        let theirs = w.nations[e].strength;
        if dur > 10.0 && (my < theirs * 0.8 || a.rng.chance(0.12)) {
            if e == w.player && !w.ai_player {
                if !w.peace_offer[n] && w.offer_cd[n] <= years {
                    w.peace_offer[n] = true;
                    w.propose_peace(n, e, 10.0, t!("戦の長期化", "a long war", "战事旷日持久"));
                }
            } else {
                w.make_peace(n, e, 10.0);
            }
        }
    }
    // 宣戦
    let wars = (0..NATIONS.len()).filter(|&o| w.at_war(n, o)).count();
    if wars > 0 || years < 12.0 || nsold < 5 {
        return;
    }
    if !a.rng.chance(0.22 * a.aggr) {
        return;
    }
    let home = w.nations[n].home;
    let mut best: Option<(f32, usize)> = None;
    for o in 0..NATIONS.len() {
        let on = &w.nations[o];
        if o == n || !on.alive || w.truce[n][o] > years || w.coalition_locked(n, o) || (o == w.player && w.protect_player) {
            continue;
        }
        let d = on.home.dist(home);
        if d > 90.0 {
            continue;
        }
        if on.strength < my * 0.75 {
            let score = on.strength + d * 2.0;
            if best.map_or(true, |b| score < b.0) {
                best = Some((score, o));
            }
        }
    }
    if let Some((_, o)) = best {
        w.declare_war(n, o, false);
    }
}

fn command_army(w: &mut World, n: usize, a: &mut AiState, soldiers: &[(Handle, Order, V2, f32)]) {
    if soldiers.is_empty() {
        return;
    }
    let home = w.nations[n].home;
    // 領内の侵入者
    let mut intruder: Option<V2> = None;
    for u in w.units.iter() {
        if !u.alive || !w.at_war(n, u.nation as usize) {
            continue;
        }
        let (x, y) = u.pos.tile();
        if Map::inb(x, y) && w.owner[Map::idx(x, y)] as usize == n {
            if intruder.map_or(true, |p| p.dist(home) > u.pos.dist(home)) {
                intruder = Some(u.pos);
            }
        }
    }
    let idle: Vec<Handle> = soldiers.iter().filter(|s| s.1 == Order::Idle).map(|s| s.0).collect();
    if let Some(p) = intruder {
        // 防衛：手の空いている兵と近くの兵を向かわせる
        let hs: Vec<Handle> = soldiers
            .iter()
            .filter(|s| s.1 == Order::Idle || (matches!(s.1, Order::AttackMove(_)) && s.2.dist(home) < 40.0))
            .map(|s| s.0)
            .collect();
        if !hs.is_empty() {
            w.order_move(&hs, p, true);
        }
        return;
    }
    // 攻撃目標：連合の標的を優先
    let mut target_nation = None;
    for c in w.coalitions.iter() {
        if c.members.contains(&n) && w.nations[c.target].alive {
            target_nation = Some(c.target);
        }
    }
    if target_nation.is_none() {
        let mut best: Option<(f32, usize)> = None;
        for o in 0..NATIONS.len() {
            if w.at_war(n, o) {
                let d = w.nations[o].home.dist(home);
                if best.map_or(true, |b| d < b.0) {
                    best = Some((d, o));
                }
            }
        }
        target_nation = best.map(|b| b.1);
    }
    match target_nation {
        Some(t) => {
            if a.attack_t > 0.0 {
                return;
            }
            let need = ((soldiers.len() as f32) * 0.6).ceil().max(3.0) as usize;
            if idle.len() >= need.min(soldiers.len()) {
                // 最寄りの敵城邑
                let mut best: Option<(f32, V2)> = None;
                for b in w.blds.iter() {
                    if b.alive && b.nation as usize == t && b.kind.city() {
                        let d = b.center().dist(home);
                        if best.map_or(true, |x| d < x.0) {
                            best = Some((d, b.center()));
                        }
                    }
                }
                if let Some((_, p)) = best {
                    w.order_move(&idle, p, true);
                    a.attack_t = 8.0;
                }
            }
        }
        None => {
            // 平時：遠くの兵は帰還
            let far: Vec<Handle> = soldiers.iter().filter(|s| s.1 == Order::Idle && s.2.dist(home) > 12.0).map(|s| s.0).collect();
            if !far.is_empty() {
                let g = home + v2(a.rng.range(-3.0, 3.0), a.rng.range(2.0, 4.0));
                w.order_move(&far, g, false);
            }
        }
    }
}

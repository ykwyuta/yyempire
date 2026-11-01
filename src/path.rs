//! グリッド A* 経路探索

use crate::geo::{MAP_H, MAP_W};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Copy, Clone)]
struct Node {
    f: f32,
    idx: u32,
}
impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool {
        self.f == o.f
    }
}
impl Eq for Node {}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.partial_cmp(&self.f).unwrap_or(Ordering::Equal)
    }
}

/// 兵・民の経路の重み（ほぼ最短）
pub const UNIT_WEIGHT: f32 = 1.15;
/// 隊商の経路の重み（最短でなくてよいので、探す量を大きく減らす）
pub const CARAVAN_WEIGHT: f32 = 1.6;

pub struct PathFinder {
    /// 直前の探索で展開したマスの数（1フレームの予算の計算に使う）
    pub last_exp: usize,
    g: Vec<f32>,
    parent: Vec<u32>,
    seen: Vec<u32>,
    closed: Vec<u32>,
    stamp: u32,
    heap: BinaryHeap<Node>,
}

const DIRS: [(i32, i32, f32); 8] = [
    (1, 0, 1.0),
    (-1, 0, 1.0),
    (0, 1, 1.0),
    (0, -1, 1.0),
    (1, 1, 1.41421),
    (1, -1, 1.41421),
    (-1, 1, 1.41421),
    (-1, -1, 1.41421),
];

impl PathFinder {
    pub fn new() -> Self {
        let n = MAP_W * MAP_H;
        PathFinder { last_exp: 0, g: vec![0.0; n], parent: vec![0; n], seen: vec![0; n], closed: vec![0; n], stamp: 0, heap: BinaryHeap::new() }
    }

    /// cost: タイルごとの移動コスト（INF=通行不可）。goal から Chebyshev 距離 goal_r 以内に到達したら終了。
    /// 到達不能時は最も近づけた地点までの経路を返す。
    /// weight: 推定距離の重み（1 なら最短。大きいほど探すマスが減るが、遠回りの経路になりうる）
    pub fn find(&mut self, cost: &[f32], start: (i32, i32), goal: (i32, i32), goal_r: i32, max_exp: usize, weight: f32) -> Vec<(i16, i16)> {
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp == 0 {
            self.seen.fill(0);
            self.closed.fill(0);
            self.stamp = 1;
        }
        let st = self.stamp;
        self.heap.clear();
        let w = MAP_W as i32;
        let h = MAP_H as i32;
        let inb = |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h;
        if !inb(start.0, start.1) {
            return Vec::new();
        }
        let heur = |x: i32, y: i32| {
            let dx = (x - goal.0).abs() as f32;
            let dy = (y - goal.1).abs() as f32;
            let (a, b) = if dx > dy { (dx, dy) } else { (dy, dx) };
            (a - b) + b * 1.41421
        };
        let si = (start.1 * w + start.0) as usize;
        self.g[si] = 0.0;
        self.seen[si] = st;
        self.parent[si] = si as u32;
        self.heap.push(Node { f: heur(start.0, start.1), idx: si as u32 });
        let mut best = (heur(start.0, start.1), si);
        let mut found = None;
        let mut exp = 0;
        while let Some(Node { idx, .. }) = self.heap.pop() {
            let i = idx as usize;
            if self.closed[i] == st {
                continue;
            }
            self.closed[i] = st;
            let x = (i % MAP_W) as i32;
            let y = (i / MAP_W) as i32;
            if (x - goal.0).abs().max((y - goal.1).abs()) <= goal_r {
                found = Some(i);
                break;
            }
            let hh = heur(x, y);
            if hh < best.0 {
                best = (hh, i);
            }
            exp += 1;
            if exp > max_exp {
                break;
            }
            let gi = self.g[i];
            for &(dx, dy, dc) in DIRS.iter() {
                let (nx, ny) = (x + dx, y + dy);
                if !inb(nx, ny) {
                    continue;
                }
                let ni = (ny * w + nx) as usize;
                let c = cost[ni];
                if !c.is_finite() {
                    continue;
                }
                if dx != 0 && dy != 0 {
                    // 角抜け禁止
                    if !cost[(y * w + nx) as usize].is_finite() || !cost[(ny * w + x) as usize].is_finite() {
                        continue;
                    }
                }
                let ng = gi + dc * c;
                if self.seen[ni] != st || ng < self.g[ni] {
                    if self.closed[ni] == st {
                        continue;
                    }
                    self.seen[ni] = st;
                    self.g[ni] = ng;
                    self.parent[ni] = i as u32;
                    self.heap.push(Node { f: ng + heur(nx, ny) * weight, idx: ni as u32 });
                }
            }
        }
        self.last_exp = exp;
        let end = found.unwrap_or(best.1);
        let mut out = Vec::new();
        let mut c = end;
        while c != si {
            out.push(((c % MAP_W) as i16, (c / MAP_W) as i16));
            let p = self.parent[c] as usize;
            if p == c {
                break;
            }
            c = p;
        }
        out.reverse();
        out
    }
}

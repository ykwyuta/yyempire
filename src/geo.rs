//! 地理データ（海岸線・河川・山脈）と国家データ、地形生成

use crate::rng::{fbm, hash2, Rng};

/// 地図の細かさ（1度あたりのタイル数の倍率。2で面積は旧来の4倍）
pub const MAP_SCALE: usize = 2;
pub const MAP_W: usize = 280 * MAP_SCALE;
pub const MAP_H: usize = 240 * MAP_SCALE;
pub const LON0: f32 = 102.0;
pub const LON1: f32 = 124.0;
pub const LAT_TOP: f32 = 42.5;
pub const LAT_BOT: f32 = 24.0;

pub fn ll_to_tile(lon: f32, lat: f32) -> (f32, f32) {
    (
        (lon - LON0) / (LON1 - LON0) * MAP_W as f32,
        (LAT_TOP - lat) / (LAT_TOP - LAT_BOT) * MAP_H as f32,
    )
}
pub fn tile_to_ll(x: f32, y: f32) -> (f32, f32) {
    (
        LON0 + x / MAP_W as f32 * (LON1 - LON0),
        LAT_TOP - y / MAP_H as f32 * (LAT_TOP - LAT_BOT),
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Terrain {
    Sea,
    Lake,
    River,
    Plain,
    Steppe,
    Forest,
    Hill,
    Mountain,
}

impl Terrain {
    pub fn passable(self) -> bool {
        !matches!(self, Terrain::Sea | Terrain::Lake | Terrain::Mountain)
    }
    pub fn move_cost(self) -> f32 {
        match self {
            Terrain::Plain | Terrain::Steppe => 1.0,
            Terrain::Forest => 1.6,
            Terrain::Hill => 1.5,
            Terrain::River => 2.5,
            _ => f32::INFINITY,
        }
    }
    /// 建物を置ける地形
    pub fn buildable(self) -> bool {
        matches!(self, Terrain::Plain | Terrain::Steppe | Terrain::Hill)
    }
}

// ---------------------------------------------------------------- 海岸線（海側ポリゴン, 経度・緯度）
const SEA_POLY: &[(f32, f32)] = &[
    (124.3, 40.0), (123.5, 39.7), (122.6, 39.3), (121.6, 38.85), (121.3, 39.4), (121.9, 40.1),
    (122.2, 40.6), (121.8, 41.0), (121.0, 40.85), (120.4, 40.3), (119.6, 39.9), (119.0, 39.25),
    (118.3, 39.1), (117.8, 38.95), (117.6, 38.4), (118.0, 38.0), (118.9, 37.6), (119.2, 37.15),
    (119.9, 37.3), (120.3, 37.7), (120.9, 37.8), (121.5, 37.5), (122.2, 37.5), (122.6, 37.2),
    (122.4, 36.85), (121.6, 36.75), (120.7, 36.4), (120.3, 36.0), (119.6, 35.5), (119.3, 34.8),
    (120.2, 34.3), (120.6, 33.4), (120.9, 32.6), (121.4, 32.2), (121.9, 31.8), (121.95, 31.2),
    (121.8, 30.85), (121.0, 30.6), (120.5, 30.35), (121.1, 30.2), (121.7, 29.95), (121.9, 29.6),
    (121.9, 29.1), (121.4, 28.4), (120.9, 28.0), (120.5, 27.3), (120.2, 26.7), (119.7, 26.2),
    (119.5, 25.5), (118.9, 25.0), (118.1, 24.5), (117.4, 23.9), (116.8, 23.4), (116.0, 22.9),
    (115.0, 22.6), (130.0, 22.0), (130.0, 40.0),
];

// 河川（幅は太さ 0=細 1=大河）
struct River {
    pts: &'static [(f32, f32)],
    major: bool,
}
const RIVERS: &[River] = &[
    // 黄河（春秋時代の古河道：天津付近へ北流）
    River { major: true, pts: &[(102.0, 36.0), (103.8, 36.1), (105.2, 37.5), (106.3, 38.5), (106.8, 39.3), (107.0, 40.3), (108.0, 40.8), (109.8, 40.6), (111.0, 40.3), (111.2, 39.6), (110.8, 38.6), (110.5, 37.5), (110.4, 36.5), (110.4, 35.5), (110.3, 34.6), (111.2, 34.8), (112.4, 34.85), (113.6, 34.9), (114.3, 35.0), (114.8, 35.9), (115.3, 36.8), (116.0, 37.8), (116.8, 38.5), (117.6, 38.95)] },
    // 長江
    River { major: true, pts: &[(102.0, 28.4), (104.0, 28.8), (105.4, 28.9), (106.6, 29.6), (108.4, 30.8), (109.5, 31.0), (110.4, 31.0), (111.3, 30.7), (112.2, 30.3), (112.9, 29.8), (113.1, 29.4), (114.3, 30.6), (115.0, 30.2), (116.0, 29.8), (117.0, 30.5), (118.4, 31.3), (118.8, 32.1), (119.5, 32.2), (120.5, 32.0), (121.7, 31.7)] },
    // 渭水
    River { major: false, pts: &[(104.5, 34.6), (106.0, 34.4), (107.5, 34.3), (108.9, 34.4), (109.8, 34.5), (110.3, 34.6)] },
    // 汾水
    River { major: false, pts: &[(111.6, 38.8), (112.5, 37.8), (111.8, 36.6), (111.5, 35.9), (110.9, 35.5), (110.45, 35.45)] },
    // 淮水（古淮河は黄海へ）
    River { major: false, pts: &[(113.3, 32.4), (114.5, 32.3), (115.5, 32.5), (116.5, 32.8), (117.4, 33.0), (118.5, 33.3), (119.3, 33.7), (120.3, 33.9)] },
    // 漢水
    River { major: false, pts: &[(106.5, 33.1), (107.0, 33.0), (108.5, 32.7), (109.8, 32.6), (110.8, 32.6), (111.6, 32.4), (112.2, 31.9), (112.5, 31.0), (113.4, 30.6), (114.3, 30.6)] },
    // 岷江
    River { major: false, pts: &[(103.6, 31.4), (104.0, 30.6), (103.8, 29.6), (104.0, 28.8)] },
    // 嘉陵江
    River { major: false, pts: &[(106.0, 32.6), (106.1, 31.0), (106.6, 29.6)] },
    // 湘江
    River { major: false, pts: &[(111.0, 25.6), (112.6, 26.9), (113.0, 28.2), (113.0, 29.1)] },
    // 贛江
    River { major: false, pts: &[(115.0, 25.6), (114.9, 27.1), (115.9, 28.7), (116.2, 29.0)] },
    // 銭塘江
    River { major: false, pts: &[(118.2, 29.2), (119.6, 29.8), (120.4, 30.3)] },
    // 遼河
    River { major: false, pts: &[(122.5, 42.6), (122.2, 41.5), (122.0, 40.95)] },
    // 桑干河・永定河
    River { major: false, pts: &[(112.5, 39.8), (114.5, 40.2), (115.8, 40.2), (116.5, 39.6), (117.5, 39.05)] },
    // 済水
    River { major: false, pts: &[(113.6, 35.0), (115.0, 35.3), (116.3, 36.2), (117.2, 36.8), (118.3, 37.4)] },
    // 泗水
    River { major: false, pts: &[(117.2, 35.6), (117.3, 34.8), (117.6, 34.0), (118.3, 33.3)] },
];

// 湖 (lon, lat, 半径 deg)
const LAKES: &[(f32, f32, f32)] = &[(112.9, 29.1, 0.38), (116.3, 29.0, 0.32), (120.2, 31.2, 0.25), (116.15, 35.35, 0.12)];

// 山脈（ポリライン, 幅 deg, 山岳中心の強さ）
struct Range {
    pts: &'static [(f32, f32)],
    w: f32,
}
const RANGES: &[Range] = &[
    Range { w: 0.25, pts: &[(113.3, 34.9), (113.8, 36.0), (113.9, 37.5), (114.5, 39.0), (115.3, 40.0)] }, // 太行山
    Range { w: 0.35, pts: &[(104.5, 34.0), (106.5, 33.9), (108.0, 33.9), (109.5, 33.8), (111.0, 33.6)] }, // 秦嶺
    Range { w: 0.30, pts: &[(105.6, 32.6), (108.0, 32.0), (110.0, 31.6)] },                              // 大巴山
    Range { w: 0.20, pts: &[(111.0, 36.0), (111.3, 37.0), (111.6, 38.5)] },                              // 呂梁山
    Range { w: 0.22, pts: &[(111.0, 33.4), (112.5, 33.6), (113.0, 33.9)] },                              // 伏牛山
    Range { w: 0.25, pts: &[(114.0, 31.6), (115.5, 31.2), (116.4, 30.8)] },                              // 大別山
    Range { w: 0.40, pts: &[(108.5, 29.3), (110.0, 28.6), (110.5, 27.0)] },                              // 武陵山
    Range { w: 0.30, pts: &[(110.0, 25.3), (112.0, 25.2), (114.0, 25.0), (116.0, 24.6)] },               // 南嶺
    Range { w: 0.30, pts: &[(116.5, 25.0), (117.5, 26.5), (118.0, 27.8), (118.5, 28.6)] },               // 武夷山
    Range { w: 0.28, pts: &[(115.5, 40.65), (117.0, 40.55), (118.5, 40.35), (119.5, 40.65)] },           // 燕山
    Range { w: 0.20, pts: &[(106.2, 35.0), (106.3, 36.2)] },                                              // 六盤山
    Range { w: 0.70, pts: &[(102.4, 25.0), (102.6, 28.0), (102.9, 30.0), (103.3, 32.0), (103.9, 33.6)] }, // 横断山脈
    Range { w: 0.30, pts: &[(108.5, 41.4), (111.0, 41.1), (113.0, 41.4)] },                              // 陰山
    Range { w: 0.20, pts: &[(113.0, 39.0), (114.0, 39.3)] },                                              // 五台・恒山
    Range { w: 0.15, pts: &[(110.6, 34.85), (111.8, 35.2)] },                                             // 中条山
    Range { w: 0.25, pts: &[(110.6, 26.5), (111.4, 28.4)] },                                              // 雪峰山
    Range { w: 0.30, pts: &[(117.5, 29.8), (118.8, 30.1), (119.5, 30.4)] },                              // 黄山・天目山
    Range { w: 0.30, pts: &[(117.2, 35.9), (117.9, 36.2), (118.4, 35.8)] },                              // 泰山・沂蒙
    Range { w: 0.25, pts: &[(112.3, 36.0), (112.8, 36.9)] },                                              // 太岳山
];

/// 山越えの古道（山脈を貫いて通れる幅を残す）
pub struct Road {
    pub name: &'static str,
    pub pts: &'static [(f32, f32)],
}
pub const ROADS: &[Road] = &[
    // 関中→漢中：秦嶺越え
    Road { name: "褒斜道", pts: &[(107.65, 34.30), (107.55, 33.90), (107.35, 33.45), (107.05, 33.12)] },
    Road { name: "故道（陳倉道）", pts: &[(107.15, 34.36), (106.75, 34.00), (106.55, 33.60), (106.80, 33.25), (107.05, 33.12)] },
    // 漢中→蜀：大巴山・剣閣越え（石牛道）
    Road { name: "金牛道", pts: &[(107.05, 33.12), (106.55, 32.75), (106.05, 32.40), (105.60, 32.00), (105.10, 31.50), (104.55, 31.05), (104.06, 30.67)] },
    // 蜀→滇：西南夷の道（蜀身毒道）
    Road { name: "蜀身毒道", pts: &[(104.06, 30.67), (104.30, 29.70), (104.60, 28.80), (104.20, 27.80), (103.60, 26.60), (103.00, 25.50), (102.00, 24.90)] },
];

fn point_in_poly(px: f32, py: f32, poly: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn dist_seg(px: f32, py: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 { (((px - a.0) * dx + (py - a.1) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    let (cx, cy) = (a.0 + dx * t, a.1 + dy * t);
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

fn dist_poly(px: f32, py: f32, pts: &[(f32, f32)]) -> f32 {
    let mut d = f32::MAX;
    for w in pts.windows(2) {
        d = d.min(dist_seg(px, py, w[0], w[1]));
    }
    d
}

// ---------------------------------------------------------------- 国家データ
pub struct NationDef {
    pub name: &'static str,
    pub lon: f32,
    pub lat: f32,
    /// 生産能力バイアス（戦国の七雄は高め）
    pub bias: f32,
    pub color: [f32; 3],
    pub extra: &'static [(f32, f32)],
    /// 三家分晋まで登場しない
    pub dormant: bool,
    pub seven: bool,
    pub desc: &'static str,
}

macro_rules! nd {
    ($n:expr, $lon:expr, $lat:expr, $b:expr, $c:expr, $ex:expr, $d:expr, $s:expr, $desc:expr) => {
        NationDef { name: $n, lon: $lon, lat: $lat, bias: $b, color: $c, extra: $ex, dormant: $d, seven: $s, desc: $desc }
    };
}

pub const NATIONS: &[NationDef] = &[
    nd!("周", 112.45, 34.62, 0.90, [0.92, 0.80, 0.25], &[], false, false, "天子の王畿・洛邑"),
    nd!("晋", 111.35, 35.62, 1.60, [0.25, 0.45, 0.90], &[(112.55, 37.85)], false, true, "絳を都とする北方の大国（後に韓・趙・魏へ）"),
    nd!("秦", 107.40, 34.52, 1.60, [0.30, 0.30, 0.36], &[(108.9, 34.3)], false, true, "雍を都とする西方の雄"),
    nd!("楚", 112.18, 30.42, 1.60, [0.85, 0.18, 0.18], &[(114.3, 30.75)], false, true, "郢を都とする南方の大国"),
    nd!("斉", 118.30, 36.85, 1.60, [0.20, 0.68, 0.32], &[(120.0, 36.4)], false, true, "臨淄を都とする東方の大国"),
    nd!("燕", 116.35, 39.90, 1.60, [0.58, 0.32, 0.75], &[], false, true, "薊を都とする北辺の国"),
    nd!("魯", 117.00, 35.60, 0.95, [0.95, 0.55, 0.20], &[], false, false, "周公の国・曲阜"),
    nd!("宋", 115.65, 34.40, 0.95, [0.55, 0.78, 0.92], &[], false, false, "殷の後裔・商丘"),
    nd!("衛", 114.30, 35.65, 0.90, [0.30, 0.75, 0.75], &[], false, false, "朝歌の国"),
    nd!("鄭", 113.70, 34.40, 0.95, [0.78, 0.52, 0.35], &[], false, false, "新鄭の国"),
    nd!("陳", 114.90, 33.73, 0.85, [0.92, 0.86, 0.52], &[], false, false, "宛丘の国"),
    nd!("蔡", 114.26, 33.26, 0.85, [0.56, 0.66, 0.22], &[], false, false, "上蔡の国"),
    nd!("曹", 115.57, 35.07, 0.80, [0.72, 0.42, 0.62], &[], false, false, "陶丘の国"),
    nd!("許", 113.85, 33.90, 0.80, [0.52, 0.52, 0.80], &[], false, false, "許の国"),
    nd!("呉", 120.60, 31.30, 1.00, [0.18, 0.50, 0.58], &[], false, false, "姑蘇の国"),
    nd!("越", 120.58, 30.00, 1.00, [0.45, 0.72, 0.35], &[], false, false, "会稽の国"),
    nd!("中山", 114.80, 38.30, 0.90, [0.88, 0.62, 0.72], &[], false, false, "白狄の鮮虞"),
    nd!("虢", 111.15, 34.75, 0.75, [0.62, 0.56, 0.40], &[], false, false, "上陽の国"),
    nd!("随", 113.37, 31.70, 0.80, [0.85, 0.45, 0.45], &[], false, false, "漢陽諸姫の雄"),
    nd!("鄧", 112.10, 32.00, 0.80, [0.65, 0.38, 0.30], &[], false, false, "鄧の国"),
    nd!("申", 112.53, 33.00, 0.80, [0.75, 0.70, 0.38], &[], false, false, "南陽の国"),
    nd!("息", 114.74, 32.34, 0.75, [0.48, 0.62, 0.62], &[], false, false, "淮水の小国"),
    nd!("巴", 106.55, 29.56, 0.90, [0.70, 0.55, 0.25], &[], false, false, "江州の国"),
    nd!("蜀", 104.06, 30.67, 0.95, [0.40, 0.58, 0.28], &[], false, false, "成都平原の国"),
    nd!("徐", 118.20, 33.40, 0.85, [0.38, 0.40, 0.68], &[], false, false, "淮夷の大国"),
    nd!("莒", 118.83, 35.58, 0.80, [0.60, 0.68, 0.50], &[], false, false, "東夷の国"),
    nd!("薛", 117.20, 34.80, 0.75, [0.80, 0.62, 0.42], &[], false, false, "薛の国"),
    nd!("邢", 114.50, 37.07, 0.80, [0.50, 0.68, 0.82], &[], false, false, "邢の国"),
    nd!("狄", 113.20, 36.30, 0.85, [0.62, 0.30, 0.22], &[], false, false, "赤狄の部族"),
    nd!("義渠", 107.60, 35.70, 0.90, [0.48, 0.36, 0.28], &[], false, false, "西戎の雄"),
    nd!("庸", 110.20, 32.20, 0.80, [0.42, 0.48, 0.38], &[], false, false, "上庸の国"),
    nd!("舒", 117.30, 31.25, 0.80, [0.70, 0.78, 0.40], &[], false, false, "群舒の国"),
    nd!("梁", 110.45, 35.47, 0.75, [0.55, 0.45, 0.65], &[], false, false, "少梁の国"),
    nd!("孤竹", 118.90, 39.90, 0.75, [0.40, 0.52, 0.50], &[], false, false, "伯夷叔斉の故国"),
    nd!("萊", 120.30, 37.30, 0.80, [0.65, 0.80, 0.68], &[], false, false, "東夷の萊"),
    nd!("郯", 118.35, 34.60, 0.75, [0.78, 0.75, 0.68], &[], false, false, "郯の国"),
    nd!("代", 114.60, 39.80, 0.80, [0.45, 0.42, 0.55], &[], false, false, "北辺の代"),
    nd!("楼煩", 112.40, 39.00, 0.80, [0.65, 0.52, 0.48], &[], false, false, "北狄の騎馬民"),
    nd!("韓", 112.10, 35.25, 1.60, [0.35, 0.62, 0.95], &[], true, true, "三家分晋で成立"),
    nd!("趙", 112.55, 37.85, 1.60, [0.20, 0.32, 0.72], &[], true, true, "三家分晋で成立"),
    nd!("魏", 111.00, 35.10, 1.60, [0.48, 0.70, 0.98], &[], true, true, "三家分晋で成立"),
];

pub const JIN: usize = 1;
pub const HAN: usize = 38;
pub const ZHAO: usize = 39;
pub const WEI: usize = 40;

// ---------------------------------------------------------------- 地図
pub struct Mine {
    pub x: i32,
    pub y: i32,
    pub amount: f32,
}

pub struct Map {
    pub terrain: Vec<Terrain>,
    pub wood: Vec<f32>,
    pub shade: Vec<f32>,
    pub mines: Vec<Mine>,
}

impl Map {
    #[inline]
    pub fn idx(x: i32, y: i32) -> usize {
        y as usize * MAP_W + x as usize
    }
    #[inline]
    pub fn inb(x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < MAP_W as i32 && y < MAP_H as i32
    }
    pub fn t(&self, x: i32, y: i32) -> Terrain {
        if Self::inb(x, y) {
            self.terrain[Self::idx(x, y)]
        } else {
            Terrain::Mountain
        }
    }
}

fn raster_line(map: &mut Map, a: (f32, f32), b: (f32, f32), thick: bool) {
    let (ax, ay) = ll_to_tile(a.0, a.1);
    let (bx, by) = ll_to_tile(b.0, b.1);
    let len = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
    let steps = (len * 4.0).ceil().max(1.0) as i32;
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let x = (ax + (bx - ax) * t).floor() as i32;
        let y = (ay + (by - ay) * t).floor() as i32;
        let pts: &[(i32, i32)] = if thick { &[(0, 0), (1, 0), (0, 1)] } else { &[(0, 0)] };
        for &(dx, dy) in pts {
            let (xx, yy) = (x + dx, y + dy);
            if Map::inb(xx, yy) {
                let i = Map::idx(xx, yy);
                if map.terrain[i] != Terrain::Sea && map.terrain[i] != Terrain::Lake {
                    map.terrain[i] = Terrain::River;
                }
            }
        }
    }
}

pub fn generate_map(seed: u32) -> Map {
    let n = MAP_W * MAP_H;
    let mut map = Map { terrain: vec![Terrain::Plain; n], wood: vec![0.0; n], shade: vec![0.0; n], mines: Vec::new() };

    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let (lon, lat) = tile_to_ll(x as f32 + 0.5, y as f32 + 0.5);
            let i = Map::idx(x, y);
            let nz = fbm(x as f32 * 0.08, y as f32 * 0.08, seed, 4);
            let nz2 = fbm(x as f32 * 0.15, y as f32 * 0.15, seed ^ 0x55aa, 3);
            map.shade[i] = nz;
            if point_in_poly(lon, lat, SEA_POLY) {
                map.terrain[i] = Terrain::Sea;
                continue;
            }
            let mut t = Terrain::Plain;
            // 北方草原・オルドス
            let wlon = lon + (nz - 0.5) * 2.4 + (nz2 - 0.5) * 1.1;
            let wlat = lat + (nz2 - 0.5) * 2.0 + (nz - 0.5) * 0.9;
            let ordos_d = ((wlon - 108.6) / 2.4).powi(2) + ((wlat - 39.4) / 1.4).powi(2);
            let steppe = (wlat > 40.6 && wlon < 116.8) || ordos_d < 1.0 || (wlat > 36.8 && wlon < 106.2);
            if steppe {
                t = Terrain::Steppe;
            }
            // 黄土高原の丘陵
            let loess_d = ((wlon - 107.8) / 3.6).powi(2) + ((wlat - 36.7) / 2.1).powi(2);
            if loess_d < 1.0 && nz2 > 0.54 {
                t = Terrain::Hill;
            }
            // 雲貴高原・嶺南・閩の丘陵
            if (wlon < 109.6 && wlat < 28.6) || (wlat < 26.2) || (wlon > 116.3 && wlat < 28.6) {
                if nz2 > 0.42 {
                    t = Terrain::Hill;
                }
                if nz2 > 0.66 {
                    t = Terrain::Mountain;
                }
            }
            // 山脈
            for r in RANGES {
                let d = dist_poly(lon, lat, r.pts);
                let jitter = (nz2 - 0.5) * r.w * 0.8;
                if d < r.w * 0.55 + jitter {
                    t = Terrain::Mountain;
                    break;
                } else if d < r.w * 1.5 + jitter && t != Terrain::Mountain {
                    t = Terrain::Hill;
                }
            }
            // 森林
            if t == Terrain::Plain || t == Terrain::Steppe {
                let fthr = if lat < 32.5 {
                    0.50
                } else if t == Terrain::Steppe {
                    0.80
                } else {
                    0.63
                };
                if nz > fthr {
                    t = Terrain::Forest;
                }
            }
            map.terrain[i] = t;
        }
    }
    // 湖
    for &(lon, lat, r) in LAKES {
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                let (lo, la) = tile_to_ll(x as f32 + 0.5, y as f32 + 0.5);
                let d = ((lo - lon) / r).powi(2) + ((la - lat) / (r * 0.75)).powi(2);
                if d < 1.0 {
                    map.terrain[Map::idx(x, y)] = Terrain::Lake;
                }
            }
        }
    }
    // 河川
    for r in RIVERS {
        for w in r.pts.windows(2) {
            raster_line(&mut map, w[0], w[1], r.major);
        }
    }
    // 古道：山を丘に削り、通れるようにする
    for r in ROADS {
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                let (lon, lat) = tile_to_ll(x as f32 + 0.5, y as f32 + 0.5);
                let i = Map::idx(x, y);
                if dist_poly(lon, lat, r.pts) < 0.11 && matches!(map.terrain[i], Terrain::Mountain | Terrain::Forest) {
                    map.terrain[i] = Terrain::Hill;
                }
            }
        }
    }
    // 国都の周囲を平地にし、木材・銅を保証
    let mut rng = Rng::new(seed as u64 ^ 0x1234);
    for nd in NATIONS {
        let mut sites = vec![(nd.lon, nd.lat)];
        sites.extend_from_slice(nd.extra);
        for (lon, lat) in sites {
            let (cx, cy) = ll_to_tile(lon, lat);
            let (cx, cy) = (cx as i32, cy as i32);
            for dy in -4..=5i32 {
                for dx in -4..=5i32 {
                    let (x, y) = (cx + dx, cy + dy);
                    if Map::inb(x, y) && dx * dx + dy * dy <= 22 {
                        let i = Map::idx(x, y);
                        if !matches!(map.terrain[i], Terrain::Sea | Terrain::River | Terrain::Lake) || (dx.abs() <= 2 && dy.abs() <= 2 && map.terrain[i] != Terrain::Sea) {
                            map.terrain[i] = Terrain::Plain;
                        }
                    }
                }
            }
            ensure_forest(&mut map, cx, cy, &mut rng);
        }
    }
    for i in 0..n {
        if map.terrain[i] == Terrain::Forest {
            map.wood[i] = 120.0;
        }
    }
    // 銅山
    for y in (2..MAP_H as i32 - 2).step_by(1) {
        for x in 2..MAP_W as i32 - 2 {
            if map.terrain[Map::idx(x, y)] == Terrain::Hill && hash2(x, y, seed ^ 0x77) < 0.035 {
                if map.mines.iter().all(|m| (m.x - x).abs() + (m.y - y).abs() > 10) {
                    map.mines.push(Mine { x, y, amount: 2500.0 });
                }
            }
        }
    }
    for nd in NATIONS {
        let (cx, cy) = ll_to_tile(nd.lon, nd.lat);
        ensure_mine(&mut map, cx as i32, cy as i32, &mut rng);
    }
    connect_regions(&mut map);
    map
}

fn ensure_forest(map: &mut Map, cx: i32, cy: i32, rng: &mut Rng) {
    let mut cnt = 0;
    for dy in -10..=10 {
        for dx in -10..=10 {
            if map.t(cx + dx, cy + dy) == Terrain::Forest {
                cnt += 1;
            }
        }
    }
    if cnt >= 14 {
        return;
    }
    for _ in 0..30 {
        let ang = rng.range(0.0, 6.283);
        let d = rng.range(5.0, 8.0);
        let fx = cx + (ang.cos() * d) as i32;
        let fy = cy + (ang.sin() * d) as i32;
        let mut placed = 0;
        for dy in -2..=2 {
            for dx in -2..=2 {
                if dx * dx + dy * dy > 5 {
                    continue;
                }
                let (x, y) = (fx + dx, fy + dy);
                if Map::inb(x, y) && matches!(map.terrain[Map::idx(x, y)], Terrain::Plain | Terrain::Steppe) {
                    map.terrain[Map::idx(x, y)] = Terrain::Forest;
                    placed += 1;
                }
            }
        }
        if placed > 8 {
            return;
        }
    }
}

fn ensure_mine(map: &mut Map, cx: i32, cy: i32, rng: &mut Rng) {
    if map.mines.iter().any(|m| (m.x - cx).abs().max((m.y - cy).abs()) <= 12) {
        return;
    }
    for _ in 0..40 {
        let ang = rng.range(0.0, 6.283);
        let d = rng.range(6.0, 9.0);
        let x = cx + (ang.cos() * d) as i32;
        let y = cy + (ang.sin() * d) as i32;
        if Map::inb(x, y) && map.terrain[Map::idx(x, y)].passable() && map.terrain[Map::idx(x, y)] != Terrain::River {
            map.terrain[Map::idx(x, y)] = Terrain::Hill;
            map.mines.push(Mine { x, y, amount: 2500.0 });
            return;
        }
    }
}

/// 通行可能領域を一つにつなぐ（孤立した盆地に峠を開ける）
fn connect_regions(map: &mut Map) {
    loop {
        let n = MAP_W * MAP_H;
        let mut comp = vec![u32::MAX; n];
        let mut sizes: Vec<usize> = Vec::new();
        for s in 0..n {
            if comp[s] != u32::MAX || !map.terrain[s].passable() {
                continue;
            }
            let id = sizes.len() as u32;
            let mut stack = vec![s];
            comp[s] = id;
            let mut sz = 0;
            while let Some(c) = stack.pop() {
                sz += 1;
                let (x, y) = ((c % MAP_W) as i32, (c / MAP_W) as i32);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if Map::inb(nx, ny) {
                        let ni = Map::idx(nx, ny);
                        if comp[ni] == u32::MAX && map.terrain[ni].passable() {
                            comp[ni] = id;
                            stack.push(ni);
                        }
                    }
                }
            }
            sizes.push(sz);
        }
        if sizes.len() <= 1 {
            return;
        }
        let main = sizes.iter().enumerate().max_by_key(|(_, s)| **s).unwrap().0 as u32;
        // 一定以上の大きさの孤立領域を本領域へ接続
        let mut target = None;
        for (id, &sz) in sizes.iter().enumerate() {
            if id as u32 != main && sz >= 12 * MAP_SCALE * MAP_SCALE {
                target = Some(id as u32);
                break;
            }
        }
        let Some(tid) = target else {
            // 小さな孤立領域は山にする
            for i in 0..n {
                if comp[i] != u32::MAX && comp[i] != main {
                    map.terrain[i] = Terrain::Mountain;
                }
            }
            return;
        };
        // 孤立領域の一点から最寄りの本領域点へ直線で峠を掘る
        let start = (0..n).find(|&i| comp[i] == tid).unwrap();
        let (sx, sy) = ((start % MAP_W) as i32, (start / MAP_W) as i32);
        let mut best = (i32::MAX, 0usize);
        for i in 0..n {
            if comp[i] == main {
                let (x, y) = ((i % MAP_W) as i32, (i / MAP_W) as i32);
                let d = (x - sx).pow(2) + (y - sy).pow(2);
                if d < best.0 {
                    best = (d, i);
                }
            }
        }
        let (ex, ey) = ((best.1 % MAP_W) as i32, (best.1 / MAP_W) as i32);
        let steps = ((ex - sx).abs().max((ey - sy).abs())).max(1);
        for s in 0..=steps {
            let x = sx + (ex - sx) * s / steps;
            let y = sy + (ey - sy) * s / steps;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1)] {
                if Map::inb(x + dx, y + dy) {
                    let i = Map::idx(x + dx, y + dy);
                    if !map.terrain[i].passable() {
                        map.terrain[i] = if map.terrain[i] == Terrain::Mountain { Terrain::Hill } else { Terrain::River };
                    }
                }
            }
        }
    }
}

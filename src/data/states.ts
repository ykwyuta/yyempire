/**
 * 春秋時代の諸国。都の位置は史料上の比定地のおおよその経緯度。
 * bias: 生産能力の補正（戦国七雄の系譜にある国を高めに設定）
 * aggr: AI の好戦性 (0..1)
 */
export interface CityDef {
  name: string;
  lon: number;
  lat: number;
  level?: number;
}

export interface StateDef {
  key: string;
  name: string;
  color: string;
  bias: number;
  aggr: number;
  major?: boolean;
  /** 戦国時代に登場する国（三家分晋）。初期には存在しない */
  successorOf?: string;
  /** 分割時の拠点 */
  seat?: [number, number];
  desc: string;
  cities: CityDef[];
}

export const STATE_DEFS: StateDef[] = [
  // ---- 戦国七雄の系譜 ----
  {
    key: 'qin', name: '秦', color: '#3a3a3a', bias: 1.45, aggr: 0.8, major: true,
    desc: '西陲の新興国。関中の沃野を背に東を窺う。',
    cities: [{ name: '雍', lon: 107.4, lat: 34.5, level: 2 }, { name: '西垂', lon: 105.2, lat: 34.2 }, { name: '櫟陽', lon: 109.25, lat: 34.65 }],
  },
  {
    key: 'jin', name: '晋', color: '#8e44ad', bias: 1.45, aggr: 0.75, major: true,
    desc: '河東の大国。のちに韓・趙・魏の三家に分かれる。',
    cities: [{ name: '絳', lon: 111.35, lat: 35.62, level: 2 }, { name: '晋陽', lon: 112.55, lat: 37.87 }, { name: '霍', lon: 111.75, lat: 36.55 }],
  },
  {
    key: 'qi', name: '斉', color: '#2e86de', bias: 1.4, aggr: 0.7, major: true,
    desc: '太公望の封国。魚塩の利で富む東方の雄。',
    cities: [{ name: '臨淄', lon: 118.3, lat: 36.85, level: 2 }, { name: '即墨', lon: 120.4, lat: 36.4 }, { name: '高唐', lon: 116.25, lat: 36.85 }],
  },
  {
    key: 'chu', name: '楚', color: '#c0392b', bias: 1.45, aggr: 0.8, major: true,
    desc: '江漢の大国。自ら王を称し中原を窺う。',
    cities: [{ name: '郢', lon: 112.2, lat: 30.45, level: 2 }, { name: '鄂', lon: 114.35, lat: 30.35 }, { name: '羅', lon: 113.15, lat: 28.75 }],
  },
  {
    key: 'yan', name: '燕', color: '#16a085', bias: 1.35, aggr: 0.55, major: true,
    desc: '北辺の古国。山戎と境を接する。',
    cities: [{ name: '薊', lon: 116.4, lat: 39.9, level: 2 }, { name: '易', lon: 115.5, lat: 39.05 }],
  },
  {
    key: 'han', name: '韓', color: '#e67e22', bias: 1.35, aggr: 0.6, successorOf: 'jin', seat: [113.0, 35.4],
    desc: '晋の六卿の一つ。三家分晋で独立。', cities: [],
  },
  {
    key: 'zhao', name: '趙', color: '#d4ac0d', bias: 1.35, aggr: 0.7, successorOf: 'jin', seat: [112.6, 37.6],
    desc: '晋の六卿の一つ。三家分晋で独立。', cities: [],
  },
  {
    key: 'wei', name: '魏', color: '#5b4fcf', bias: 1.35, aggr: 0.7, successorOf: 'jin', seat: [111.0, 35.0],
    desc: '晋の六卿の一つ。三家分晋で独立。', cities: [],
  },
  // ---- 中原・その他の諸侯 ----
  { key: 'zhou', name: '周', color: '#e8d37a', bias: 0.7, aggr: 0.1, desc: '天下の宗主。権威はあれど力は衰えた。', cities: [{ name: '洛邑', lon: 112.45, lat: 34.62, level: 2 }] },
  { key: 'zheng', name: '鄭', color: '#7f8c8d', bias: 0.95, aggr: 0.45, desc: '中原の要衝。春秋初期の小覇。', cities: [{ name: '新鄭', lon: 113.72, lat: 34.4 }] },
  { key: 'wey', name: '衛', color: '#a3cb38', bias: 0.85, aggr: 0.3, desc: '殷の故地を治める古国。', cities: [{ name: '朝歌', lon: 114.2, lat: 35.62 }] },
  { key: 'song', name: '宋', color: '#f8a5c2', bias: 0.95, aggr: 0.4, desc: '殷の末裔の国。', cities: [{ name: '商丘', lon: 115.65, lat: 34.42 }, { name: '彭城', lon: 117.2, lat: 34.27 }] },
  { key: 'lu', name: '魯', color: '#c56cf0', bias: 0.9, aggr: 0.3, desc: '周公旦の封国。礼の国。', cities: [{ name: '曲阜', lon: 117.0, lat: 35.6 }] },
  { key: 'cao', name: '曹', color: '#b8e994', bias: 0.75, aggr: 0.2, desc: '済水のほとりの小国。', cities: [{ name: '陶丘', lon: 115.5, lat: 35.1 }] },
  { key: 'chen', name: '陳', color: '#f3a683', bias: 0.8, aggr: 0.25, desc: '舜の末裔と伝わる国。', cities: [{ name: '宛丘', lon: 114.9, lat: 33.72 }] },
  { key: 'cai', name: '蔡', color: '#78e08f', bias: 0.8, aggr: 0.25, desc: '楚と中原の間で揺れる国。', cities: [{ name: '上蔡', lon: 114.27, lat: 33.25 }] },
  { key: 'xu', name: '許', color: '#e15f41', bias: 0.7, aggr: 0.15, desc: '鄭と楚に挟まれた小国。', cities: [{ name: '許', lon: 113.85, lat: 33.95 }] },
  { key: 'qii', name: '杞', color: '#cf6a87', bias: 0.7, aggr: 0.15, desc: '夏の末裔の小国。', cities: [{ name: '雍丘', lon: 114.75, lat: 34.6 }] },
  { key: 'guo', name: '虢', color: '#596275', bias: 0.75, aggr: 0.25, desc: '周王室を支えた姫姓の国。', cities: [{ name: '上陽', lon: 111.15, lat: 34.75 }] },
  { key: 'ju', name: '莒', color: '#3dc1d3', bias: 0.8, aggr: 0.25, desc: '東夷系の国。', cities: [{ name: '莒', lon: 118.85, lat: 35.6 }] },
  { key: 'lai', name: '莱', color: '#546de5', bias: 0.8, aggr: 0.25, desc: '膠東半島の東夷の国。', cities: [{ name: '莱', lon: 120.25, lat: 37.3 }] },
  { key: 'xing', name: '邢', color: '#63cdda', bias: 0.75, aggr: 0.2, desc: '太行山麓の姫姓国。', cities: [{ name: '邢', lon: 114.5, lat: 37.07 }] },
  { key: 'zhongshan', name: '中山', color: '#574b90', bias: 0.95, aggr: 0.45, desc: '白狄の鮮虞が建てた国。', cities: [{ name: '鮮虞', lon: 114.8, lat: 38.35 }] },
  { key: 'xuu', name: '徐', color: '#f19066', bias: 0.85, aggr: 0.35, desc: '淮水流域の淮夷の雄。', cities: [{ name: '徐', lon: 118.2, lat: 33.4 }] },
  { key: 'shu2', name: '舒', color: '#e77f67', bias: 0.75, aggr: 0.2, desc: '江淮の群舒。', cities: [{ name: '舒', lon: 117.3, lat: 31.15 }] },
  { key: 'huang', name: '黄', color: '#f7d794', bias: 0.75, aggr: 0.2, desc: '淮水上流の嬴姓国。', cities: [{ name: '黄', lon: 115.05, lat: 32.1 }] },
  { key: 'sui', name: '随', color: '#ea8685', bias: 0.8, aggr: 0.25, desc: '漢東の大国。', cities: [{ name: '随', lon: 113.37, lat: 31.7 }] },
  { key: 'shen', name: '申', color: '#786fa6', bias: 0.75, aggr: 0.2, desc: '南陽盆地の姜姓国。', cities: [{ name: '申', lon: 112.55, lat: 33.0 }] },
  { key: 'deng', name: '鄧', color: '#f5cd79', bias: 0.75, aggr: 0.2, desc: '漢水中流の曼姓国。', cities: [{ name: '鄧', lon: 112.05, lat: 32.1 }] },
  { key: 'yong', name: '庸', color: '#a29bfe', bias: 0.8, aggr: 0.25, desc: '漢水上流の山国。', cities: [{ name: '上庸', lon: 110.2, lat: 32.25 }] },
  // ---- 辺境・異民族 ----
  { key: 'wu', name: '呉', color: '#00b894', bias: 1.0, aggr: 0.5, desc: '太伯の建てた江南の国。', cities: [{ name: '姑蘇', lon: 120.55, lat: 31.35 }, { name: '朱方', lon: 119.45, lat: 32.15 }] },
  { key: 'yue', name: '越', color: '#0984e3', bias: 1.0, aggr: 0.45, desc: '会稽の越人の国。', cities: [{ name: '会稽', lon: 120.55, lat: 29.95 }] },
  { key: 'shu', name: '蜀', color: '#fdcb6e', bias: 0.95, aggr: 0.3, desc: '成都平原の古国。', cities: [{ name: '成都', lon: 104.06, lat: 30.67 }] },
  { key: 'ba', name: '巴', color: '#e17055', bias: 0.85, aggr: 0.3, desc: '江州を拠る巴人の国。', cities: [{ name: '江州', lon: 106.55, lat: 29.6 }] },
  { key: 'yiqu', name: '義渠', color: '#b33939', bias: 0.9, aggr: 0.55, desc: '涇水上流の西戎。', cities: [{ name: '義渠', lon: 107.6, lat: 35.95 }] },
  { key: 'chidi', name: '赤狄', color: '#cd6133', bias: 0.85, aggr: 0.5, desc: '上党の狄人。', cities: [{ name: '潞', lon: 113.1, lat: 36.25 }] },
  { key: 'shanrong', name: '山戎', color: '#84817a', bias: 0.8, aggr: 0.5, desc: '燕山の戎。', cities: [{ name: '令支', lon: 118.65, lat: 40.0 }] },
  { key: 'dai', name: '代', color: '#ccae62', bias: 0.8, aggr: 0.35, desc: '北辺の狄の国。', cities: [{ name: '代', lon: 114.6, lat: 39.85 }] },
  { key: 'loufan', name: '楼煩', color: '#aaa69d', bias: 0.85, aggr: 0.5, desc: '北方の騎馬民。', cities: [{ name: '楼煩', lon: 112.2, lat: 39.3 }] },
  { key: 'donghu', name: '東胡', color: '#706fd3', bias: 0.85, aggr: 0.5, desc: '遼西の遊牧民。', cities: [{ name: '東胡', lon: 120.6, lat: 41.6 }] },
  { key: 'minyue', name: '閩越', color: '#34ace0', bias: 0.85, aggr: 0.3, desc: '閩の越人。', cities: [{ name: '東冶', lon: 119.25, lat: 26.1 }] },
  { key: 'nanyue', name: '百越', color: '#33d9b2', bias: 0.85, aggr: 0.3, desc: '嶺南の越人。', cities: [{ name: '番禺', lon: 113.3, lat: 23.2 }] },
  { key: 'xiou', name: '西甌', color: '#218c74', bias: 0.8, aggr: 0.3, desc: '嶺南西部の越人。', cities: [{ name: '布山', lon: 109.6, lat: 23.6 }] },
  { key: 'yelang', name: '夜郎', color: '#ff793f', bias: 0.85, aggr: 0.3, desc: '西南の夷の国。', cities: [{ name: '夜郎', lon: 106.7, lat: 26.6 }] },
];

/** 戦国七雄 */
export const SEVEN_HEROES = ['qin', 'chu', 'qi', 'yan', 'han', 'zhao', 'wei'];

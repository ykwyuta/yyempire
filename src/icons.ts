/**
 * ユニット・建物・資源のアイコン（SVG）。
 * 同じ定義を UI（インライン SVG）と Canvas（画像キャッシュ）で共用する。
 */
import type { BuildingKind, ResKey, SettlementTier, UnitKind } from './config';

export type StructIcon = BuildingKind | SettlementTier;

const INK = '#2a1a0e';
const BONE = '#fbf1d6';
const BRONZE = '#c9a24a';
const WOOD = '#8a5a2b';

const SHIELD = 'M16 1.5 L29 6 V15.5 C29 23.5 23 28.8 16 30.8 C9 28.8 3 23.5 3 15.5 V6 Z';

/** 兵種ごとの図像（32x32, 白地の図に墨線） */
const UNIT_FIG: Record<string, string> = {
  // 農夫: 笠・鍬・籠
  worker: `
    <path d="M7.5 12.5 L16 6.5 L24.5 12.5 Z" fill="${BRONZE}" stroke="${INK}" stroke-width="1.2" stroke-linejoin="round"/>
    <circle cx="16" cy="14.6" r="2.6" fill="${BONE}" stroke="${INK}" stroke-width="1.1"/>
    <path d="M11.5 27 L12.8 18.4 Q16 17 19.2 18.4 L20.5 27 Z" fill="${BONE}" stroke="${INK}" stroke-width="1.2" stroke-linejoin="round"/>
    <path d="M23.5 9.5 L19.6 26.5" stroke="${WOOD}" stroke-width="1.8" stroke-linecap="round"/>
    <path d="M21.3 8.2 L26.4 9.2 L25.9 11.6 L22.2 11 Z" fill="#9aa3a8" stroke="${INK}" stroke-width="1"/>`,
  // 戈兵: 兜・戈・盾
  infantry: `
    <path d="M21.5 4.5 V28" stroke="${WOOD}" stroke-width="1.8" stroke-linecap="round"/>
    <path d="M20.2 5 L21.5 1 L22.8 5 Z" fill="${BRONZE}" stroke="${INK}" stroke-width="0.8" stroke-linejoin="round"/>
    <path d="M21.5 6 L28.2 7.4 L21.5 9.4 Z" fill="${BRONZE}" stroke="${INK}" stroke-width="1" stroke-linejoin="round"/>
    <path d="M12.8 10.6 Q15.5 6.5 18.2 10.6 Z" fill="${BRONZE}" stroke="${INK}" stroke-width="1"/>
    <circle cx="15.5" cy="12" r="2.5" fill="${BONE}" stroke="${INK}" stroke-width="1.1"/>
    <path d="M11.5 27.5 L12.5 16.5 Q15.5 15 18.5 16.5 L19.5 27.5 Z" fill="${BONE}" stroke="${INK}" stroke-width="1.2" stroke-linejoin="round"/>
    <path d="M18.5 18 L21.5 17" stroke="${INK}" stroke-width="1.4" stroke-linecap="round"/>
    <ellipse cx="10" cy="20.5" rx="3.4" ry="5.6" fill="${BRONZE}" stroke="${INK}" stroke-width="1.2"/>
    <path d="M10 16.5 V24.5" stroke="${INK}" stroke-width="0.8"/>`,
  // 戦車: 大車輪・車箱・御者・旗
  chariot: `
    <path d="M24 5 V14" stroke="${INK}" stroke-width="1.1"/>
    <path d="M24 5 L29 6.8 L24 8.6 Z" fill="#d84a3a" stroke="${INK}" stroke-width="0.8"/>
    <circle cx="19" cy="11.5" r="2.2" fill="${BONE}" stroke="${INK}" stroke-width="1"/>
    <path d="M9 13.5 H26 V19 H9 Z" fill="${WOOD}" stroke="${INK}" stroke-width="1.2" stroke-linejoin="round"/>
    <path d="M3 16.5 L9 16.5" stroke="${INK}" stroke-width="1.6" stroke-linecap="round"/>
    <circle cx="17" cy="22" r="6.8" fill="none" stroke="${INK}" stroke-width="2.6"/>
    <circle cx="17" cy="22" r="6.8" fill="none" stroke="${BRONZE}" stroke-width="1.3"/>
    <path d="M17 15.2 V28.8 M10.2 22 H23.8 M12.2 17.2 L21.8 26.8 M21.8 17.2 L12.2 26.8" stroke="${INK}" stroke-width="0.9"/>
    <circle cx="17" cy="22" r="1.6" fill="${INK}"/>`,
  // 弓兵: 弓と矢
  archer: `
    <path d="M12 4.5 Q27.5 16 12 27.5" fill="none" stroke="${WOOD}" stroke-width="2.6" stroke-linecap="round"/>
    <path d="M12 4.5 Q27.5 16 12 27.5" fill="none" stroke="${INK}" stroke-width="0.7"/>
    <path d="M12 4.5 L12 27.5" stroke="${BONE}" stroke-width="1"/>
    <path d="M5 16 H25" stroke="${INK}" stroke-width="1.6" stroke-linecap="round"/>
    <path d="M24 12.8 L29 16 L24 19.2 Z" fill="${BRONZE}" stroke="${INK}" stroke-width="0.9" stroke-linejoin="round"/>
    <path d="M5 16 L3 13.5 M5 16 L3 18.5 M7.5 16 L5.5 13.5 M7.5 16 L5.5 18.5" stroke="${BONE}" stroke-width="1.2" stroke-linecap="round"/>`,
  // 弩兵: 弩
  crossbow: `
    <path d="M4.5 13.5 Q16 4 27.5 13.5" fill="none" stroke="${WOOD}" stroke-width="2.6" stroke-linecap="round"/>
    <path d="M4.5 13.5 Q16 4 27.5 13.5" fill="none" stroke="${INK}" stroke-width="0.7"/>
    <path d="M4.5 13.5 L16 16.5 L27.5 13.5" fill="none" stroke="${BONE}" stroke-width="1"/>
    <path d="M14 7 H18 V28.5 Q16 30 14 28.5 Z" fill="${WOOD}" stroke="${INK}" stroke-width="1.1" stroke-linejoin="round"/>
    <path d="M16 3 V16" stroke="${INK}" stroke-width="1.3"/>
    <path d="M13.8 4.8 L16 1.8 L18.2 4.8 Z" fill="${BRONZE}" stroke="${INK}" stroke-width="0.8"/>
    <path d="M18 19 H20.5 V23" fill="none" stroke="${BRONZE}" stroke-width="1.6"/>`,
  // 騎兵: 馬首と騎手
  cavalry: `
    <path d="M7.5 29 L9.6 20.5 C7.2 17.8 7.8 12.4 11.8 9 L14.6 5.2 L15.9 8.4 C20.6 8.6 25.2 12.2 27 16.8 L25.6 19.4 L21.4 17.6 L18.8 20.6 L19.6 29 Z"
      fill="${WOOD}" stroke="${INK}" stroke-width="1.2" stroke-linejoin="round"/>
    <path d="M11.8 9 C9.8 12.5 9.6 16.4 11.2 19.6" fill="none" stroke="${INK}" stroke-width="2"/>
    <circle cx="19.6" cy="12.4" r="1.1" fill="${INK}"/>
    <path d="M23.5 15.8 L21 18" stroke="${BRONZE}" stroke-width="1"/>
    <circle cx="8" cy="5.5" r="2.2" fill="${BONE}" stroke="${INK}" stroke-width="1"/>
    <path d="M5 13 L6.2 8 Q8 7 9.8 8 L10.8 11.5" fill="${BONE}" stroke="${INK}" stroke-width="1" stroke-linejoin="round"/>
    <path d="M4 2.5 L9.5 12.5" stroke="${INK}" stroke-width="1.1"/>`,
  // 衝車: 屋根付きの車と破城槌
  ram: `
    <path d="M3.5 15 L14.5 7.5 L25.5 15 Z" fill="${WOOD}" stroke="${INK}" stroke-width="1.2" stroke-linejoin="round"/>
    <path d="M6 15 H23 V22 H6 Z" fill="${BRONZE}" stroke="${INK}" stroke-width="1.1"/>
    <path d="M9 15 V22 M13 15 V22 M17 15 V22 M21 15 V22" stroke="${INK}" stroke-width="0.6"/>
    <path d="M16 17 H29.5" stroke="${INK}" stroke-width="3.6" stroke-linecap="round"/>
    <path d="M16 17 H29.5" stroke="${WOOD}" stroke-width="2.2" stroke-linecap="round"/>
    <circle cx="10" cy="24.5" r="3" fill="${BONE}" stroke="${INK}" stroke-width="1.2"/>
    <circle cx="19.5" cy="24.5" r="3" fill="${BONE}" stroke="${INK}" stroke-width="1.2"/>`,
};

/** 建物の図像 */
const BUILDING_FIG: Record<string, string> = {
  // 邑: 木柵に囲まれた茅葺きの家々
  town: `
    <path d="M5 13.5 L10 9.5 L15 13.5 Z" fill="#c9a24a" stroke="${INK}" stroke-width="1" stroke-linejoin="round"/>
    <path d="M6 13.5 H14 V19 H6 Z" fill="#e2cfa0" stroke="${INK}" stroke-width="1"/>
    <path d="M9 19 V16 H11 V19" fill="${INK}"/>
    <path d="M15.5 12 L21.5 7 L27.5 12 Z" fill="#c9a24a" stroke="${INK}" stroke-width="1" stroke-linejoin="round"/>
    <path d="M16.8 12 H26.2 V19 H16.8 Z" fill="#e2cfa0" stroke="${INK}" stroke-width="1"/>
    <path d="M20.5 19 V15 H22.5 V19" fill="${INK}"/>
    <path d="M3 19.5 H29 V27.5 H3 Z" fill="none"/>
    <g fill="${WOOD}" stroke="${INK}" stroke-width="0.8" stroke-linejoin="round">
      <path d="M3 28.5 V21.5 L4.3 19.8 L5.6 21.5 V28.5 Z"/><path d="M6.4 28.5 V21.5 L7.7 19.8 L9 21.5 V28.5 Z"/>
      <path d="M9.8 28.5 V21.5 L11.1 19.8 L12.4 21.5 V28.5 Z"/>
      <path d="M19.6 28.5 V21.5 L20.9 19.8 L22.2 21.5 V28.5 Z"/><path d="M23 28.5 V21.5 L24.3 19.8 L25.6 21.5 V28.5 Z"/>
      <path d="M26.4 28.5 V21.5 L27.7 19.8 L29 21.5 V28.5 Z"/>
    </g>
    <path d="M12.8 28.5 V22.5 H19.2 V28.5" fill="none" stroke="${INK}" stroke-width="1"/>
    <path d="M12.4 22.2 H19.6" stroke="${WOOD}" stroke-width="1.6"/>`,
  // 大城: 二重の城壁と重層の宮殿
  capital: `
    <path d="M2 19 H30 V29.5 H2 Z" fill="#b9a27a" stroke="${INK}" stroke-width="1.2"/>
    <path d="M2 19 V16.8 H4.6 V19 M7 19 V16.8 H9.6 V19 M22.4 19 V16.8 H25 V19 M27.4 19 V16.8 H30 V19" fill="#b9a27a" stroke="${INK}" stroke-width="0.9"/>
    <path d="M5 23 H27" stroke="${INK}" stroke-width="0.6"/>
    <path d="M13 29.5 V24.8 Q16 21.6 19 24.8 V29.5 Z" fill="${INK}"/>
    <path d="M8.5 17 H23.5 V13.5 H8.5 Z" fill="#d8443a" stroke="${INK}" stroke-width="0.9"/>
    <path d="M5 14 Q16 7.6 27 14 Q16 11.2 5 14 Z" fill="${INK}"/>
    <path d="M11 10.5 H21 V7.5 H11 Z" fill="#d8443a" stroke="${INK}" stroke-width="0.8"/>
    <path d="M8 8 Q16 2.2 24 8 Q16 5.8 8 8 Z" fill="${INK}"/>
    <path d="M12 4.6 Q16 1 20 4.6 Q16 3.4 12 4.6 Z" fill="${BRONZE}" stroke="${INK}" stroke-width="0.6"/>
    <path d="M3.3 16.8 V9.5 M28.7 16.8 V9.5" stroke="${INK}" stroke-width="0.9"/>
    <path d="M3.3 9.5 L7 10.6 L3.3 11.8 Z M28.7 9.5 L25 10.6 L28.7 11.8 Z" fill="#d84a3a" stroke="${INK}" stroke-width="0.5"/>`,
  // 砦: 土塁の上の木造の櫓と旗、前面に逆茂木
  fort: `
    <path d="M2.5 29.5 Q4 21 9 20 H23 Q28 21 29.5 29.5 Z" fill="#9c8558" stroke="${INK}" stroke-width="1.1" stroke-linejoin="round"/>
    <path d="M6 24.5 Q16 22.5 26 24.5" fill="none" stroke="${INK}" stroke-width="0.6"/>
    <path d="M10 20 V10 H22 V20" fill="${WOOD}" stroke="${INK}" stroke-width="1.1"/>
    <path d="M10 13.5 H22 M10 17 H22 M14 10 V20 M18 10 V20" stroke="${INK}" stroke-width="0.6"/>
    <path d="M8.5 10.5 H23.5 V8 H8.5 Z" fill="#c9a24a" stroke="${INK}" stroke-width="0.9"/>
    <path d="M8.5 8 V6 H10.5 V8 M13 8 V6 H15 V8 M17 8 V6 H19 V8 M21.5 8 V6 H23.5 V8" fill="#c9a24a" stroke="${INK}" stroke-width="0.8"/>
    <path d="M16 6 V1.5" stroke="${INK}" stroke-width="1"/>
    <path d="M16 1.5 L22 3 L16 4.6 Z" fill="#d84a3a" stroke="${INK}" stroke-width="0.6"/>
    <path d="M4 28 L8 23.5 M7 28 L4.5 24 M24 28 L28 23.5 M27 28 L24.5 24" stroke="${INK}" stroke-width="1.1" stroke-linecap="round"/>`,
  city: `
    <path d="M3 17 H29 V29 H3 Z" fill="#b9a27a" stroke="${INK}" stroke-width="1.2"/>
    <path d="M3 17 V14.5 H6 V17 M9 17 V14.5 H12 V17 M20 17 V14.5 H23 V17 M26 17 V14.5 H29 V17" fill="#b9a27a" stroke="${INK}" stroke-width="1"/>
    <path d="M12.5 29 V23.5 Q16 19.5 19.5 23.5 V29 Z" fill="${INK}"/>
    <path d="M9.5 15 H22.5 V10.5 H9.5 Z" fill="#d8443a" stroke="${INK}" stroke-width="1"/>
    <path d="M6 11 Q16 4 26 11 Q16 8.4 6 11 Z" fill="${INK}"/>
    <path d="M10 6.5 Q16 1.5 22 6.5 Q16 4.6 10 6.5 Z" fill="${INK}"/>
    <path d="M12 7 H20 V10.5 H12 Z" fill="#d8443a" stroke="${INK}" stroke-width="0.8"/>`,
  storehouse: `
    <path d="M3 14 L16 5 L29 14 Z" fill="#8f6a3a" stroke="${INK}" stroke-width="1.2" stroke-linejoin="round"/>
    <path d="M6 14 H26 V28 H6 Z" fill="#c9a970" stroke="${INK}" stroke-width="1.2"/>
    <path d="M13 28 V19 H19 V28" fill="${INK}"/>
    <ellipse cx="9.5" cy="25" rx="2.4" ry="3" fill="#e8d18a" stroke="${INK}" stroke-width="0.9"/>
    <ellipse cx="23" cy="25" rx="2.4" ry="3" fill="#e8d18a" stroke="${INK}" stroke-width="0.9"/>`,
  barracks: `
    <path d="M4 28 L16 9 L28 28 Z" fill="#9a7a4a" stroke="${INK}" stroke-width="1.2" stroke-linejoin="round"/>
    <path d="M13.5 28 L16 20 L18.5 28 Z" fill="${INK}"/>
    <path d="M16 9 V2.5" stroke="${INK}" stroke-width="1.2"/>
    <path d="M16 2.5 L23 4.3 L16 6.4 Z" fill="#d84a3a" stroke="${INK}" stroke-width="0.8"/>
    <path d="M4.5 9 L11 28 M27.5 9 L21 28" stroke="${WOOD}" stroke-width="1.5"/>
    <path d="M3.4 7 L5.6 10.6 L4 11 Z M28.6 7 L26.4 10.6 L28 11 Z" fill="${BRONZE}" stroke="${INK}" stroke-width="0.6"/>`,
  tower: `
    <path d="M10 29 L12 12 H20 L22 29 Z" fill="#b9a27a" stroke="${INK}" stroke-width="1.2" stroke-linejoin="round"/>
    <path d="M10.5 12 H21.5 V8 H10.5 Z" fill="#d8443a" stroke="${INK}" stroke-width="1"/>
    <path d="M7 8.5 Q16 1.5 25 8.5 Q16 5.6 7 8.5 Z" fill="${INK}"/>
    <path d="M14.5 16 H17.5 V19 H14.5 Z M14.5 22 H17.5 V25 H14.5 Z" fill="${INK}"/>`,
  market: `
    <circle cx="16" cy="16" r="12.5" fill="${BRONZE}" stroke="${INK}" stroke-width="1.4"/>
    <circle cx="16" cy="16" r="10" fill="none" stroke="${INK}" stroke-width="0.8"/>
    <path d="M12.5 12.5 H19.5 V19.5 H12.5 Z" fill="${INK}"/>
    <text x="16" y="10.3" font-size="5" text-anchor="middle" fill="${INK}" font-family="serif">半</text>
    <text x="16" y="25.5" font-size="5" text-anchor="middle" fill="${INK}" font-family="serif">両</text>`,
};

const RES_FIG: Record<ResKey, string> = {
  food: `
    <path d="M16 29 V8" stroke="#b08a30" stroke-width="1.6"/>
    <path d="M16 8 Q12 6 13 3 Q17 4 16 8 Z M16 13 Q11 12 11.5 8.5 Q16 9.5 16 13 Z M16 13 Q21 12 20.5 8.5 Q16 9.5 16 13 Z M16 18 Q11 17 11.5 13.5 Q16 14.5 16 18 Z M16 18 Q21 17 20.5 13.5 Q16 14.5 16 18 Z M16 23 Q11 22 11.5 18.5 Q16 19.5 16 23 Z M16 23 Q21 22 20.5 18.5 Q16 19.5 16 23 Z"
      fill="#f0c94a" stroke="${INK}" stroke-width="0.7"/>`,
  wood: `
    <path d="M4 20 L22 10 L28 14 L10 24 Z" fill="${WOOD}" stroke="${INK}" stroke-width="1.1" stroke-linejoin="round"/>
    <ellipse cx="7" cy="22" rx="3.4" ry="2.6" fill="#d9b07a" stroke="${INK}" stroke-width="1"/>
    <path d="M6 26 L22 17 L27 20 L11 29 Z" fill="#a06a32" stroke="${INK}" stroke-width="1.1" stroke-linejoin="round"/>
    <ellipse cx="8.5" cy="27.5" rx="3" ry="2.2" fill="#d9b07a" stroke="${INK}" stroke-width="1"/>`,
  metal: `
    <path d="M5 22 L9 13 H23 L27 22 Z" fill="#c9a24a" stroke="${INK}" stroke-width="1.2" stroke-linejoin="round"/>
    <path d="M9 13 L12 9 H20 L23 13" fill="#e3c370" stroke="${INK}" stroke-width="1.1" stroke-linejoin="round"/>
    <path d="M11 16 H21" stroke="#fff3c0" stroke-width="1.2"/>`,
};

function figFor(kind: UnitKind, warring: boolean): string {
  if (kind === 'archer' && warring) return UNIT_FIG.crossbow;
  return UNIT_FIG[kind];
}

/** ユニットのアイコン SVG（陣営色の盾 / 円の上に図像） */
export function unitSvg(kind: UnitKind, color: string, warring = false, size = 32): string {
  const base = kind === 'worker'
    ? `<circle cx="16" cy="16" r="14.5" fill="${color}" stroke="${INK}" stroke-width="1.6"/><circle cx="16" cy="16" r="12.6" fill="none" stroke="#ffffff55" stroke-width="1"/>`
    : `<path d="${SHIELD}" fill="${color}" stroke="${INK}" stroke-width="1.6" stroke-linejoin="round"/><path d="M16 4 L26.5 7.8 V15.5 C26.5 22 21.6 26.6 16 28.3" fill="none" stroke="#ffffff44" stroke-width="1"/>`;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="${size}" height="${size}">${base}${figFor(kind, warring)}</svg>`;
}

/** 建物アイコン（陣営色の台座付き） */
export function buildingSvg(kind: StructIcon, color: string, size = 32): string {
  const base = `<rect x="1" y="1" width="30" height="30" rx="5" fill="${color}" stroke="${INK}" stroke-width="1.6"/>`;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="${size}" height="${size}">${base}${BUILDING_FIG[kind]}</svg>`;
}

/** 台座なしの図像（ボタン・選択枠用） */
export function plainUnitSvg(kind: UnitKind, warring = false, size = 28): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="${size}" height="${size}">${figFor(kind, warring)}</svg>`;
}
export function plainBuildingSvg(kind: StructIcon, size = 28): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="${size}" height="${size}">${BUILDING_FIG[kind]}</svg>`;
}
export function resSvg(r: ResKey, size = 16): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="${size}" height="${size}" style="vertical-align:-3px">${RES_FIG[r]}</svg>`;
}
export function upgradeSvg(size = 28): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="${size}" height="${size}"><path d="M16 3 L27 15 H20 V29 H12 V15 H5 Z" fill="${BRONZE}" stroke="${INK}" stroke-width="1.4" stroke-linejoin="round"/></svg>`;
}

/** Canvas 描画用の画像キャッシュ */
const cache = new Map<string, HTMLImageElement>();
export function iconImage(key: string, svg: () => string): HTMLImageElement | null {
  let img = cache.get(key);
  if (!img) {
    img = new Image();
    img.src = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(svg());
    cache.set(key, img);
  }
  return img.complete && img.naturalWidth > 0 ? img : null;
}

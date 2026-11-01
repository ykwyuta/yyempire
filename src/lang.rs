//! 表示言語（日本語・英語・中国語）
//!
//! 画面の文字は `t!`（決まった文）・`tf!`（数や名前を差し込む文）で三言語を並べて書き、
//! いまの言語のものを選ぶ。国名・史実の出来事・特徴・英霊などのデータは日本語のまま持ち、
//! 表示するときに `tx` で辞書（`i18n.rs`）から訳を引く。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Ja = 0,
    En = 1,
    Zh = 2,
}

pub const LANGS: [Lang; 3] = [Lang::Ja, Lang::En, Lang::Zh];

static LANG: AtomicU8 = AtomicU8::new(0);

impl Lang {
    /// 言語の名前（その言語で書く）
    pub fn native(self) -> &'static str {
        match self {
            Lang::Ja => "日本語",
            Lang::En => "English",
            Lang::Zh => "中文",
        }
    }
    /// 設定ファイルに書く符号
    pub fn code(self) -> &'static str {
        match self {
            Lang::Ja => "ja",
            Lang::En => "en",
            Lang::Zh => "zh",
        }
    }
    pub fn from_code(s: &str) -> Option<Lang> {
        LANGS.into_iter().find(|l| l.code() == s.trim())
    }
    pub fn next(self) -> Lang {
        LANGS[(self as usize + 1) % LANGS.len()]
    }
}

pub fn lang() -> Lang {
    LANGS[LANG.load(Ordering::Relaxed) as usize % LANGS.len()]
}

pub fn set_lang(l: Lang) {
    LANG.store(l as u8, Ordering::Relaxed);
}

/// 三言語の文から、いまの言語のものを選ぶ
pub fn pick(ja: &'static str, en: &'static str, zh: &'static str) -> &'static str {
    match lang() {
        Lang::Ja => ja,
        Lang::En => en,
        Lang::Zh => zh,
    }
}

/// 決まった文：`t!("日本語", "English", "中文")`
#[macro_export]
macro_rules! t {
    ($ja:expr, $en:expr, $zh:expr $(,)?) => {
        $crate::lang::pick($ja, $en, $zh)
    };
}

/// 数や名前を差し込む文：`tf!("{}年", "{} years", "{}年", n)`
#[macro_export]
macro_rules! tf {
    ($ja:literal, $en:literal, $zh:literal $(, $a:expr)* $(,)?) => {
        match $crate::lang::lang() {
            $crate::lang::Lang::Ja => format!($ja $(, $a)*),
            $crate::lang::Lang::En => format!($en $(, $a)*),
            $crate::lang::Lang::Zh => format!($zh $(, $a)*),
        }
    };
}

fn dict() -> &'static HashMap<&'static str, (&'static str, &'static str)> {
    static D: OnceLock<HashMap<&'static str, (&'static str, &'static str)>> = OnceLock::new();
    D.get_or_init(|| crate::i18n::DICT.iter().map(|&(ja, en, zh)| (ja, (en, zh))).collect())
}

/// データの文（国名・出来事・特徴・英霊など）を、いまの言語に訳す。辞書に無ければそのまま
pub fn tx(ja: &'static str) -> &'static str {
    match lang() {
        Lang::Ja => ja,
        l => match dict().get(ja) {
            Some(&(en, zh)) => {
                if l == Lang::En {
                    en
                } else {
                    zh
                }
            }
            None => ja,
        },
    }
}

/// 辞書にその文があるか（テスト用）
#[cfg(test)]
pub fn has(ja: &str) -> bool {
    dict().contains_key(ja)
}

/// 窓の題名
pub fn window_title() -> &'static str {
    t!("春秋戦国 — 中華統一への道", "Warring States — The Road to Unification", "春秋战国 — 中华统一之路")
}

/// 言葉をつなぐ区切り（日本語・中国語は「・」「、」、英語は「, 」）
pub fn sep() -> &'static str {
    t!("・", ", ", "、")
}

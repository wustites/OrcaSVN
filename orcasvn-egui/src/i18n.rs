use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Lang {
    #[default]
    ZhCn,
    EnUs,
}

static LANG: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(lang: Lang) {
    LANG.store(lang as u8, Ordering::Relaxed);
}

/// Pick the string for the active UI language.
pub fn t(zh: &'static str, en: &'static str) -> &'static str {
    if LANG.load(Ordering::Relaxed) == Lang::EnUs as u8 {
        en
    } else {
        zh
    }
}

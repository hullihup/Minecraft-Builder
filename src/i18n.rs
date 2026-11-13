//! Tiny bilingual helper: every UI string is written as `t("Deutsch", "English")`.

use std::sync::atomic::{AtomicBool, Ordering};

static GERMAN: AtomicBool = AtomicBool::new(true);

pub fn set_german(de: bool) {
    GERMAN.store(de, Ordering::Relaxed);
}

pub fn is_german() -> bool {
    GERMAN.load(Ordering::Relaxed)
}

/// Picks the string for the active UI language.
pub fn t(de: &'static str, en: &'static str) -> &'static str {
    if is_german() { de } else { en }
}

/// Same as [`t`] but for an owned pair, used by block definitions.
pub type L = (&'static str, &'static str);

pub fn tl(l: L) -> &'static str {
    t(l.0, l.1)
}

//! Table UI symbols from Mana 1.18, with Font Awesome fallbacks.
//! Checked against the font stylesheet and the policy table on 2026-09-17.
//! No logos, planeswalker marks, or faction watermarks.

/// Mana's hand, library, graveyard and exile, in that order.
pub const ZONES: [char; 4] = ['\u{e9ca}', '\u{e9cb}', '\u{e9cc}', '\u{e9cd}'];
/// Untap, upkeep, draw, main, combat, attack, block, damage, combat end,
/// second main, end step and cleanup. Generic actions use Font Awesome.
pub const PHASES: [char; 12] = [
    '\u{e61b}', '\u{e942}', '\u{e9ca}', '\u{e624}', '\u{e9ce}', '\u{e950}', '\u{e9c3}', '\u{e9dd}',
    '\u{f11e}', '\u{e624}', '\u{f253}', '\u{f51a}',
];
/// Lifelink, skull counter, and energy.
pub const LIFE: char = '\u{e95c}';
/// Skull counter used for poison.
pub const POISON: char = '\u{e940}';
/// Energy counter.
pub const ENERGY: char = '\u{e907}';

/// Whether this codepoint comes from Mana rather than Font Awesome.
#[must_use]
pub const fn is_mana(glyph: char) -> bool {
    (glyph as u32) < 0xf000
}

//! Original Life Coil: an optional mana, tap and life price for one card.
use baylee_cards_dsl::prelude::*;

/// Common: three mana, tap and two life draw one card.
pub static TOWER_COIL_COMMON: TokenDef = TokenDef {
    name: "Life Coil — Common",
    types: TypeSet::ARTIFACT,
    abilities: &[activated!(
        cost!("{3}", TapSelf, PayLife(2)),
        &[Effect::draw(1)]
    )],
    ..TokenDef::DEFAULT
};

/// Uncommon: two mana, tap and two life draw one card.
pub static TOWER_COIL_UNCOMMON: TokenDef = TokenDef {
    name: "Life Coil — Uncommon",
    types: TypeSet::ARTIFACT,
    abilities: &[activated!(
        cost!("{2}", TapSelf, PayLife(2)),
        &[Effect::draw(1)]
    )],
    ..TokenDef::DEFAULT
};

/// Rare: one mana, tap and two life draw one card.
pub static TOWER_COIL_RARE: TokenDef = TokenDef {
    name: "Life Coil — Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[activated!(
        cost!("{1}", TapSelf, PayLife(2)),
        &[Effect::draw(1)]
    )],
    ..TokenDef::DEFAULT
};

/// Mythic Rare: one mana and two life draw one card, without tapping.
pub static TOWER_COIL_MYTHIC: TokenDef = TokenDef {
    name: "Life Coil — Mythic Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[activated!(cost!("{1}", PayLife(2)), &[Effect::draw(1)])],
    ..TokenDef::DEFAULT
};

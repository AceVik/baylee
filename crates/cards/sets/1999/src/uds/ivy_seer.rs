//! Ivy Seer — {3}{G} — Creature — Elf Wizard
//! Oracle: {2}{G}, {T}: Reveal any number of green cards in your hand. Target creature gets +X/+X until end of turn, where X is the number of cards revealed this way.
//! Set: UDS #110 — Urza's Destiny | Scryfall ID: 018ad11c-1351-4eff-94ac-3926037d7247 | Oracle ID: f61591d5-0f2a-48cd-a091-863d2e93e019
// NOT SUPPORTED: variable-count hand reveal (reveal any number of green cards from your hand and scale X by the number revealed) is unsupported; plays as a 1/1 Elf Wizard body
// PARTIAL: {3}{G} 1/1 Elf Wizard creature body implemented; missing "{2}{G}, {T}: Reveal any number of green cards in your hand. Target creature gets +X/+X until end of turn, where X is the number of cards revealed this way" because the DSL has no cost or effect to reveal any number of matching cards from your hand and count them.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IVY_SEER,
    oracle_id = "f61591d5-0f2a-48cd-a091-863d2e93e019",
    scryfall_id = "018ad11c-1351-4eff-94ac-3926037d7247",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "variable-count hand reveal (reveal any number of green cards from your hand and scale X by the number revealed) is unsupported; plays as a 1/1 Elf Wizard body"
    ),
    faces = &[face!(
        name = "Ivy Seer",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

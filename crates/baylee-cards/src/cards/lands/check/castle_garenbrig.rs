//! Castle Garenbrig — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Forest.
//! Oracle: {T}: Add {G}.
//! Oracle: {2}{G}{G}, {T}: Add six {G}. Spend this mana only to cast creature spells or activate abilities of creatures.
//! Set: ELD #240 — Throne of Eldraine | Scryfall ID: e3c2c66c-f7f0-41d5-a805-a129aeaf1b75 | Oracle ID: de75e5dd-8a52-406c-b55c-96d686885500
// PARTIAL — enters tapped unless you control a Forest and taps for {G}.
// The restricted mana ability cannot express spending mana to activate abilities of creatures.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static FOREST_YOU_CONTROL: Filter = f!(your Filter::HasSubtype(land::FOREST));

card!(
    index = index::CASTLE_GARENBRIG,
    oracle_id = "de75e5dd-8a52-406c-b55c-96d686885500",
    scryfall_id = "e3c2c66c-f7f0-41d5-a805-a129aeaf1b75",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "ManaRestriction's filter names spells only, so \"or activate abilities of creatures\" is not expressible",
    ),
    faces = &[face!(
        name = "Castle Garenbrig",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&FOREST_YOU_CONTROL)],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // NOT SUPPORTED: {2}{G}{G}, {T}: Add six {G}. Spend this mana only to cast creature spells or activate abilities of creatures.
    ],
);

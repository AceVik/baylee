//! Zirilan of the Claw — {3}{R}{R} — Legendary Creature — Lizard Shaman
//! Oracle: {1}{R}{R}, {T}: Search your library for a Dragon permanent card, put that card onto the battlefield, then shuffle. That Dragon gains haste until end of turn. Exile it at the beginning of the next end step.
//! Set: MIR #204 — Mirage | Scryfall ID: a105b7f4-c93b-4b54-acf8-7212907d9cd6 | Oracle ID: 3aeb28ac-8ed2-4a90-a631-d84361ac6acc
// PARTIAL — {1}{R}{R}, {T} searches your library for a Dragon permanent card and puts it onto the battlefield; no haste grant or next-end-step exile rider on the searched card exists yet.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DRAGON_PERMANENT: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::DRAGON),
    Filter::Or(&[
        Filter::ARTIFACT,
        Filter::CREATURE,
        Filter::ENCHANTMENT,
        Filter::LAND,
        Filter::PLANESWALKER,
    ]),
]);

card!(
    index = index::ZIRILAN_OF_THE_CLAW,
    oracle_id = "3aeb28ac-8ed2-4a90-a631-d84361ac6acc",
    scryfall_id = "a105b7f4-c93b-4b54-acf8-7212907d9cd6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "{1}{R}{R}, {T} searches your library for a Dragon permanent card and puts it onto the battlefield; no haste grant or next-end-step exile rider on the searched card exists yet",
    ),
    faces = &[face!(
        name = "Zirilan of the Claw",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::LIZARD, subtypes::creature::SHAMAN],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!("{1}{R}{R}", TapSelf),
        &[Effect::SearchLibrary {
            filter: &DRAGON_PERMANENT,
            finds: &[Find::BATTLEFIELD],
            optional: false,
        }],
    )],
);

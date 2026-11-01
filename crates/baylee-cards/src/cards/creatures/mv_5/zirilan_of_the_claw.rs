//! Zirilan of the Claw — {3}{R}{R} — Legendary Creature — Lizard Shaman
//! Oracle: {1}{R}{R}, {T}: Search your library for a Dragon permanent card, put that card onto the battlefield, then shuffle. That Dragon gains haste until end of turn. Exile it at the beginning of the next end step.
//! Set: MIR #204 — Mirage | Scryfall ID: a105b7f4-c93b-4b54-acf8-7212907d9cd6 | Oracle ID: 3aeb28ac-8ed2-4a90-a631-d84361ac6acc
// PARTIAL — the 3/4 body only: the {1}{R}{R}, {T} ability is not played, since no effect can name the Dragon it puts onto the battlefield for "That Dragon gains haste until end of turn" and "Exile it at the beginning of the next end step", and a search without them leaves the Dragon on the battlefield for good.
// NOT SUPPORTED: "{1}{R}{R}, {T}: Search your library for a Dragon permanent card, put that card onto the battlefield, then shuffle. That Dragon gains haste until end of turn. Exile it at the beginning of the next end step." The search alone would keep the Dragon forever.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZIRILAN_OF_THE_CLAW,
    oracle_id = "3aeb28ac-8ed2-4a90-a631-d84361ac6acc",
    scryfall_id = "a105b7f4-c93b-4b54-acf8-7212907d9cd6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "the 3/4 body only: the {1}{R}{R}, {T} Dragon search is not played, since no effect can name the Dragon it puts onto the battlefield for its haste until end of turn and its exile at the beginning of the next end step, and without them the Dragon would stay for good",
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
);

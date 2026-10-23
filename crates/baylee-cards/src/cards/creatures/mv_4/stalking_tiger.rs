//! Stalking Tiger — {3}{G} — Creature — Cat
//! Oracle: This creature can't be blocked by more than one creature.
//! Set: W17 #28 — Welcome Deck 2017 | Scryfall ID: 2048b66a-586f-49ad-bd2b-7fce1eb45ce6 | Oracle ID: e6af1429-276f-41a8-88d0-062661dc0cd4
// PARTIAL — 3/3 Cat without "can't be blocked by more than one creature": no blocker-count restriction on a single attacker exists yet.
// NOT SUPPORTED: "This creature can't be blocked by more than one creature." There is no per-attacker cap on blockers.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STALKING_TIGER,
    oracle_id = "e6af1429-276f-41a8-88d0-062661dc0cd4",
    scryfall_id = "2048b66a-586f-49ad-bd2b-7fce1eb45ce6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "3/3 Cat without \"can't be blocked by more than one creature\": no blocker-count restriction on a single attacker exists yet",
    ),
    faces = &[face!(
        name = "Stalking Tiger",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CAT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

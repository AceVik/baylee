//! Bogardan Phoenix — {2}{R}{R}{R} — Creature — Phoenix
//! Oracle: Flying
//! Oracle: When this creature dies, exile it if it had a death counter on it. Otherwise, return it to the battlefield under your control and put a death counter on it.
//! Set: VIS #76 — Visions | Scryfall ID: 253db28a-3873-4364-80d7-a8164000ea9e | Oracle ID: 72586a7a-3c73-4012-8664-1db0f268eb60
// PARTIAL — has flying and returns from the graveyard to the battlefield under your control with a death counter when it dies; no last-known-information death-counter check to exile it instead if it had a death counter when it died exists yet.

use baylee_cards_dsl::counters;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BOGARDAN_PHOENIX,
    oracle_id = "72586a7a-3c73-4012-8664-1db0f268eb60",
    scryfall_id = "253db28a-3873-4364-80d7-a8164000ea9e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "has flying and returns from the graveyard to the battlefield under your control with a death counter when it dies; no last-known-information death-counter check to exile it instead if it had a death counter when it died exists yet",
    ),
    faces = &[face!(
        name = "Bogardan Phoenix",
        mana_cost = mana!("{2}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHOENIX],
        power = Some(3),
        toughness = Some(3),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::ReturnSourceFromGraveyard {
            face: None,
            tapped: false,
            owner_control: false,
            counters: Some((counters::DEATH, 1)),
        }],
    )],
);

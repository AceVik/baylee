//! Bogardan Phoenix — {2}{R}{R}{R} — Creature — Phoenix
//! Oracle: Flying
//! Oracle: When this creature dies, exile it if it had a death counter on it. Otherwise, return it to the battlefield under your control and put a death counter on it.
//! Set: VIS #76 — Visions | Scryfall ID: 253db28a-3873-4364-80d7-a8164000ea9e | Oracle ID: 72586a7a-3c73-4012-8664-1db0f268eb60
// IMPLEMENTED — flying; when it dies without a death counter it returns to the battlefield under your control with one, and when it dies with a death counter it is exiled.

use baylee_cards_dsl::counters;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BOGARDAN_PHOENIX,
    oracle_id = "72586a7a-3c73-4012-8664-1db0f268eb60",
    scryfall_id = "253db28a-3873-4364-80d7-a8164000ea9e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bogardan Phoenix",
        mana_cost = mana!("{2}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHOENIX],
        power = Some(3),
        toughness = Some(3),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[
        // "Exile it if it had a death counter on it": a dies trigger asks its
        // filter of the permanent as it left, counters included (CR 603.10a).
        triggered!(
            Trigger::Dies(&Filter::And(&[
                Filter::This,
                Filter::Not(&Filter::HasCounter(counters::DEATH)),
            ])),
            &[Effect::ReturnSourceFromGraveyard {
                face: None,
                tapped: false,
                owner_control: false,
                counters: Some((counters::DEATH, 1)),
            }],
        ),
        triggered!(
            Trigger::Dies(&Filter::And(&[
                Filter::This,
                Filter::HasCounter(counters::DEATH),
            ])),
            &[Effect::ExileSource],
        ),
    ],
);

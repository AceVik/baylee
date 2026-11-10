//! Avenging Angel — {3}{W}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: When this creature dies, you may put it on top of its owner's library.
//! Set: TPR #6 — Tempest Remastered | Scryfall ID: 1edc1be8-f1be-448a-b869-6fd81f7e21ec | Oracle ID: 3415a199-05ac-4627-be97-f842eb1415a3
// IMPLEMENTED — Flying and optional dies trigger putting this creature on top of its owner's library.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVENGING_ANGEL,
    oracle_id = "3415a199-05ac-4627-be97-f842eb1415a3",
    scryfall_id = "1edc1be8-f1be-448a-b869-6fd81f7e21ec",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Avenging Angel",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::MayDo {
            effects: &[Effect::PutSourceOnTopOfLibrary],
        }]
    )],
);

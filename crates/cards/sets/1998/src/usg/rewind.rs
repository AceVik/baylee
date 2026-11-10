//! Rewind — {2}{U}{U} — Instant
//! Oracle: Counter target spell. Untap up to four lands.
//! Set: USG #93 — Urza's Saga | Scryfall ID: 9e51c4fb-fb29-4b1c-b78e-1fadf94fc9a5 | Oracle ID: bb27bfdf-fe8d-45bd-ad62-8118dce06eda
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REWIND,
    oracle_id = "bb27bfdf-fe8d-45bd-ad62-8118dce06eda",
    scryfall_id = "9e51c4fb-fb29-4b1c-b78e-1fadf94fc9a5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rewind",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::CounterTargetSpell,
            Effect::UntapChosen {
                filter: &Filter::LAND,
                count: 4,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    ),],
);

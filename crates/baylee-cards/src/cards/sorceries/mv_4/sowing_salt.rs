//! Sowing Salt — {2}{R}{R} — Sorcery
//! Oracle: Exile target nonbasic land. Search its controller's graveyard, hand, and library for all cards with the same name as that land and exile them. Then that player shuffles.
//! Set: BOK #118 — Betrayers of Kamigawa | Scryfall ID: bd243ce5-152b-4d9c-991e-be56b6731d99 | Oracle ID: 131d2b70-959d-4814-b0c8-38031f141718
// NOT SUPPORTED: searching the land's controller's graveyard, hand, and library for all cards with the same name and exiling them is not in Effect.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOWING_SALT,
    oracle_id = "131d2b70-959d-4814-b0c8-38031f141718",
    scryfall_id = "bd243ce5-152b-4d9c-991e-be56b6731d99",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "exiles target nonbasic land; searching its controller's graveyard, hand, and library for all cards with the same name and exiling them is not in Effect"
    ),
    faces = &[face!(
        name = "Sowing Salt",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::exile(TargetSpec::Object(&Filter::NONBASIC_LAND))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::NONBASIC_LAND))),
    )],
);

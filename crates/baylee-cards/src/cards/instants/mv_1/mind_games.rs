//! Mind Games — {U} — Instant
//! Oracle: Buyback {2}{U} (You may pay an additional {2}{U} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Tap target artifact, creature, or land.
//! Set: STH #38 — Stronghold | Scryfall ID: 4da50979-1f5d-48d1-9406-dfc785273c04 | Oracle ID: ea43372f-b0a7-4d1b-be3e-a4a738b06d59
// PARTIAL: taps target artifact, creature, or land; buyback {2}{U} is not supported

use baylee_cards_dsl::prelude::*;

static ARTIFACT_CREATURE_OR_LAND: Filter =
    Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::LAND]);

card!(
    index = index::MIND_GAMES,
    oracle_id = "ea43372f-b0a7-4d1b-be3e-a4a738b06d59",
    scryfall_id = "4da50979-1f5d-48d1-9406-dfc785273c04",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "taps target artifact, creature, or land; buyback {2}{U} is not supported"
    ),
    faces = &[face!(
        name = "Mind Games",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::TapTarget],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &ARTIFACT_CREATURE_OR_LAND
        ))),
    )],
);

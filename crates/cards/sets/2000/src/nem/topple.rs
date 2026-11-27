//! Topple — {2}{W} — Sorcery
//! Oracle: Exile target creature with the greatest power among creatures on the battlefield. (If two or more creatures are tied for greatest power, target any one of them.)
//! Set: NEM #24 — Nemesis | Scryfall ID: a7c25c67-4214-4318-a718-7d351e713f80 | Oracle ID: 75eb30cc-688d-400a-abe3-5319b79fcbae
// IMPLEMENTED — exiles target creature tied for or having the greatest power on the battlefield.

use baylee_cards_dsl::prelude::*;

static GREATEST_POWER_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::PowerEqualsAmount(&Amount::GreatestPowerAmong(&Filter::CREATURE)),
]);

card!(
    index = index::TOPPLE,
    oracle_id = "75eb30cc-688d-400a-abe3-5319b79fcbae",
    scryfall_id = "a7c25c67-4214-4318-a718-7d351e713f80",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Topple",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::exile(TargetSpec::Object(&GREATEST_POWER_CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&GREATEST_POWER_CREATURE))),
    ),],
);

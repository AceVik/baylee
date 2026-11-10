//! Ghoul's Feast — {1}{B} — Instant
//! Oracle: Target creature gets +X/+0 until end of turn, where X is the number of creature cards in your graveyard.
//! Set: DDJ #67 — Duel Decks: Izzet vs. Golgari | Scryfall ID: a2909e6e-d196-4654-9193-2c9e0cfd90ee | Oracle ID: 042e0533-faf4-475e-be7b-438d23c6e605
// IMPLEMENTED — target creature gets +X/+0 until end of turn, X counted as
// the creature cards in your graveyard (Amount::CountOf).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GHOUL_S_FEAST,
    oracle_id = "042e0533-faf4-475e-be7b-438d23c6e605",
    scryfall_id = "a2909e6e-d196-4654-9193-2c9e0cfd90ee",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Ghoul's Feast",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::CountOf {
                filter: &Filter::CREATURE,
                zone: ZoneSel::GraveyardYou,
            },
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);

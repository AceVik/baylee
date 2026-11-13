//! Hell Swarm — {B} — Instant
//! Oracle: All creatures get -1/-0 until end of turn.
//! Set: LEG #103 — Legends | Scryfall ID: 64164d1b-75f4-456e-a717-90ce554dc16c | Oracle ID: f8df23ed-e239-435a-a4a9-cf10da6df28f
// IMPLEMENTED — one untargeted `PumpFilter` over every creature gives -1/-0
// until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HELL_SWARM,
    oracle_id = "f8df23ed-e239-435a-a4a9-cf10da6df28f",
    scryfall_id = "64164d1b-75f4-456e-a717-90ce554dc16c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Hell Swarm",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::CREATURE,
        controlled_by: None,
        power: Amount::NegXFixed(1),
        toughness: Amount::Fixed(0),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }])],
);

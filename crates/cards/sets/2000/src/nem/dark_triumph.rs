//! Dark Triumph — {4}{B} — Instant
//! Oracle: If you control a Swamp, you may sacrifice a creature rather than pay this spell's mana cost.
//! Oracle: Creatures you control get +2/+0 until end of turn.
//! Set: NEM #55 — Nemesis | Scryfall ID: 794657cd-b292-41d7-a4a6-3f3dd20dc07a | Oracle ID: 885630a8-c43b-4804-acd6-14a7c7321d73
// NOT SUPPORTED: creatures you control get +2/+0 until end of turn for {4}{B}; the alternative cost of sacrificing a creature while controlling a Swamp is not yet modeled
// PARTIAL: creatures you control get +2/+0 until end of turn for {4}{B}; the alternative cost of sacrificing a creature while controlling a Swamp is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DARK_TRIUMPH,
    oracle_id = "885630a8-c43b-4804-acd6-14a7c7321d73",
    scryfall_id = "794657cd-b292-41d7-a4a6-3f3dd20dc07a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "creatures you control get +2/+0 until end of turn for {4}{B}; the alternative cost of sacrificing a creature while controlling a Swamp is not yet modeled",
    ),
    faces = &[face!(
        name = "Dark Triumph",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::YOUR_CREATURE,
        controlled_by: None,
        power: Amount::Fixed(2),
        toughness: Amount::Fixed(0),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }]),],
);

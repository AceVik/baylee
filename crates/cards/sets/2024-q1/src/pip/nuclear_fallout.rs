//! Nuclear Fallout — {X}{B}{B} — Sorcery
//! Oracle: Each creature gets twice -X/-X until end of turn. Each player gets X rad counters.
//! Set: PIP #47 — Fallout | Scryfall ID: e717e4b0-a8c1-4411-b18a-1d17e7225503 | Oracle ID: f09388e2-f190-4b48-ba85-3ffc33020392
use baylee_cards_dsl::prelude::*;

card!(
    index = index::NUCLEAR_FALLOUT,
    oracle_id = "f09388e2-f190-4b48-ba85-3ffc33020392",
    scryfall_id = "e717e4b0-a8c1-4411-b18a-1d17e7225503",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Nuclear Fallout",
        mana_cost = mana!("{X}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::PumpFilter {
            filter: &Filter::CREATURE,
            controlled_by: None,
            power: Amount::Negated(&Amount::DoubleX),
            toughness: Amount::Negated(&Amount::DoubleX),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        },
        Effect::Rad {
            who: PlayerRel::EachPlayer,
            amount: Amount::X,
        },
    ])],
);

//! Blind Fury — {2}{R}{R} — Instant
//! Oracle: All creatures lose trample until end of turn. If a creature would deal combat damage to a creature this turn, it deals double that damage to that creature instead.
//! Set: MIR #158 — Mirage | Scryfall ID: 0ff91a17-0a5f-456f-a431-8505ba29e679 | Oracle ID: 5dbfd316-a0a9-4caa-99b1-069931d2aaa4
// PARTIAL — all creatures lose trample until end of turn; the damage doubling is not played.
// NOT SUPPORTED: "If a creature would deal combat damage to a creature this turn, it deals double that damage to that creature instead." No effect or static ability doubles damage dealt by creatures to creatures.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLIND_FURY,
    oracle_id = "5dbfd316-a0a9-4caa-99b1-069931d2aaa4",
    scryfall_id = "0ff91a17-0a5f-456f-a431-8505ba29e679",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "removes trample from all creatures until end of turn; doubling combat damage dealt by creatures to creatures this turn is not supported",
    ),
    faces = &[face!(
        name = "Blind Fury",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::continuous(
        &Filter::CREATURE,
        Modifier::RemoveKeyword(KeywordSet::TRAMPLE),
        Duration::UntilEndOfTurn,
    )])],
);

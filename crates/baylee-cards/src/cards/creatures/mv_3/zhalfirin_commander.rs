//! Zhalfirin Commander — {2}{W} — Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: {1}{W}{W}: Target Knight creature gets +1/+1 until end of turn.
//! Set: DDG #13 — Duel Decks: Knights vs. Dragons | Scryfall ID: e296cd1a-0508-46c1-aeda-82ecde991caa | Oracle ID: 1053a778-da6a-4b8e-81fb-537a355d4bfe
// IMPLEMENTED — flanking and {1}{W}{W}: target Knight creature gets +1/+1 until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static KNIGHT_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::KNIGHT),
]);

card!(
    index = index::ZHALFIRIN_COMMANDER,
    oracle_id = "1053a778-da6a-4b8e-81fb-537a355d4bfe",
    scryfall_id = "e296cd1a-0508-46c1-aeda-82ecde991caa",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLANKING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Zhalfirin Commander",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLANKING,
    ),],
    abilities = &[activated!(
        cost!("{1}{W}{W}"),
        &[Effect::PumpTarget {
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&KNIGHT_CREATURE)),
    ),],
);

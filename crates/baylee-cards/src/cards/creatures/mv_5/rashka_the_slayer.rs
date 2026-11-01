//! Rashka the Slayer — {3}{W}{W} — Legendary Creature — Human Archer
//! Oracle: Reach (This creature can block creatures with flying.)
//! Oracle: Whenever Rashka blocks one or more black creatures, Rashka gets +1/+2 until end of turn.
//! Set: HML #12 — Homelands | Scryfall ID: ddf30363-9db2-44c5-8c13-dbf1aaa8c86b | Oracle ID: 8e1e477a-192a-4e5c-a18b-7e25d9bd47f3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::BlockedBySource,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);

card!(
    index = index::RASHKA_THE_SLAYER,
    oracle_id = "8e1e477a-192a-4e5c-a18b-7e25d9bd47f3",
    scryfall_id = "ddf30363-9db2-44c5-8c13-dbf1aaa8c86b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    keywords = KeywordSet::REACH,
    faces = &[face!(
        name = "Rashka the Slayer",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ARCHER],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedByOneOrMore(&BLACK_CREATURE),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(1, 2),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);

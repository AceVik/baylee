//! Tetravus — {6} — Artifact Creature — Construct
//! Oracle: Flying
//! Oracle: This creature enters with three +1/+1 counters on it.
//! Oracle: At the beginning of your upkeep, you may remove any number of +1/+1 counters from this creature. If you do, create that many 1/1 colorless Tetravite artifact creature tokens. They each have flying and "This token can't be enchanted."
//! Oracle: At the beginning of your upkeep, you may exile any number of tokens created with this creature. If you do, put that many +1/+1 counters on this creature.
//! Set: ME4 #233 — Masters Edition IV | Scryfall ID: c1b83a15-1d0b-4fc6-b7cb-fcd0063bfa7d | Oracle ID: 85255c26-4e74-4cf0-91a6-78ddba5abdc6
// PARTIAL — flying and the three +1/+1 counters on entry are written; both upkeep exchanges are off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TETRAVUS,
    oracle_id = "85255c26-4e74-4cf0-91a6-78ddba5abdc6",
    scryfall_id = "c1b83a15-1d0b-4fc6-b7cb-fcd0063bfa7d",
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "no effect removes a chosen number of counters or exiles a chosen \
         number of tokens created by this source, and no Tetravite token is \
         registered"
    ),
    faces = &[face!(
        name = "Tetravus",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(1),
        toughness = Some(1),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(3)
        }],
    ),],
    // NOT SUPPORTED: "At the beginning of your upkeep, you may remove any
    // number of +1/+1 counters from this creature. If you do, create that
    // many 1/1 colorless Tetravite artifact creature tokens. They each have
    // flying and \"This token can't be enchanted.\"" — no DSL effect removes
    // a number chosen as it resolves (`Effect::RemoveCounterSelf` takes a
    // fixed n and `Amount::X` is an announced activation cost), no effect
    // reads back how many were removed, and no `TokenDef` for Tetravite
    // exists in the registry.
    // NOT SUPPORTED: "At the beginning of your upkeep, you may exile any
    // number of tokens created with this creature. If you do, put that many
    // +1/+1 counters on this creature." — there is no vocabulary for tokens
    // created by this source, and no effect exiles a chosen number of them.
    abilities = &[],
);

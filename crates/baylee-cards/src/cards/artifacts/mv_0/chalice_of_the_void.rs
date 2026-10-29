//! Chalice of the Void — {X}{X} — Artifact
//! Oracle: This artifact enters with X charge counters on it.
//! Oracle: Whenever a player casts a spell with mana value equal to the number of charge counters on this artifact, counter that spell.
//! Set: A25 #222 — Masters 25 | Scryfall ID: 1f0d2e8e-c8f2-4b31-a6ba-6283fc8740d4 | Oracle ID: 74027ba4-a6ba-4696-baf1-60ef0b27ee0a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static SPELL_FILTER: Filter = Filter::CmcEqualsCountersOnSource(CounterKind::Charge);

card!(
    index = index::CHALICE_OF_THE_VOID,
    oracle_id = "74027ba4-a6ba-4696-baf1-60ef0b27ee0a",
    scryfall_id = "1f0d2e8e-c8f2-4b31-a6ba-6283fc8740d4",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chalice of the Void",
        mana_cost = mana!("{X}{X}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::Charge,
            amount: Amount::X,
        }],
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&SPELL_FILTER),
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::EventObject)),
    )],
);

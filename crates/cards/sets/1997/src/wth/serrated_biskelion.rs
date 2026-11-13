//! Serrated Biskelion — {3} — Artifact Creature — Construct
//! Oracle: {T}: Put a -1/-1 counter on this creature and a -1/-1 counter on target creature.
//! Set: DDF #46 — Duel Decks: Elspeth vs. Tezzeret | Scryfall ID: cc2e3173-d2d5-4c6b-8156-0d78f8b1b5d1 | Oracle ID: c8552706-83f2-41d7-8068-75891abb8bde
// IMPLEMENTED — Taps to put a -1/-1 counter on itself and a -1/-1 counter on target creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SERRATED_BISKELION,
    oracle_id = "c8552706-83f2-41d7-8068-75891abb8bde",
    scryfall_id = "cc2e3173-d2d5-4c6b-8156-0d78f8b1b5d1",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Serrated Biskelion",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!(TapSelf),
        &[
            Effect::AddCounterSource {
                kind: CounterKind::M1M1,
                amount: Amount::Fixed(1),
            },
            Effect::AddCounter {
                kind: CounterKind::M1M1,
                amount: Amount::Fixed(1),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),],
);

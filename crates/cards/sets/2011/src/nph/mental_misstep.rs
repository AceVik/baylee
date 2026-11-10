//! Mental Misstep — {U/P} — Instant
//! Oracle: ({U/P} can be paid with either {U} or 2 life.)
//! Oracle: Counter target spell with mana value 1.
//! Set: NPH #38 — New Phyrexia | Scryfall ID: 61e9c6df-1c84-4eab-9076-a4feb6347c10 | Oracle ID: 1a0770e6-b093-4439-baff-6889a50ba12e
// IMPLEMENTED — hard counter whose target may only be a one-mana spell.
// "Mana value 1" is the two bounds meeting, `Filter` having no equality
// variant; the Phyrexian pip rides on the printed cost, as Metamorph's does.

static MANA_VALUE_ONE: Filter = Filter::And(&[Filter::CmcAtMost(1), Filter::CmcAtLeast(1)]);

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MENTAL_MISSTEP,
    oracle_id = "1a0770e6-b093-4439-baff-6889a50ba12e",
    scryfall_id = "61e9c6df-1c84-4eab-9076-a4feb6347c10",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mental Misstep",
        mana_cost = mana!("{U/P}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&MANA_VALUE_ONE)))
    )],
);

// Engine-level coverage belongs in `engine::s4_tests`, beside
// `counterspell_counters_a_creature_spell`: this is the pool's first stack
// target filtered on mana value, so a two-mana spell must not be offered and
// a one-mana spell must be.

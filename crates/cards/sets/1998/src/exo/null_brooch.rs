//! Null Brooch — {4} — Artifact
//! Oracle: {2}, {T}, Discard your hand: Counter target noncreature spell.
//! Set: EXO #136 — Exodus | Scryfall ID: d5d5a0c6-916c-428a-ae66-8adc8844e56e | Oracle ID: 6f885041-3e57-4a69-84f2-fd207ff9f31b
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NULL_BROOCH,
    oracle_id = "6f885041-3e57-4a69-84f2-fd207ff9f31b",
    scryfall_id = "d5d5a0c6-916c-428a-ae66-8adc8844e56e",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Null Brooch",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}", TapSelf, DiscardHand),
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::NONCREATURE))),
    )],
);

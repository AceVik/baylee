//! Rogue's Passage — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Target creature can't be blocked this turn.
//! Set: HOC #212 — The Hobbit Eternal | Scryfall ID: 4493b62b-f354-47ff-9dcf-cc6e29de77c6 | Oracle ID: f29dc596-2121-4421-8463-15f6c2e8b9b3
// IMPLEMENTED - a colorless mana ability, plus {4},{T} to make a creature
// unblockable for the turn.
//
// The grant is `Effect::PumpTarget` with no pump at all: it is the variant
// that says "the target" outright, where `CreateContinuousEffect` would have
// to overload `Filter::This` to mean it. The land's own animation cards use
// that other form because they mean *themselves*.

use baylee_cards_dsl::prelude::*;

card! {
    index: 937,
    oracle_id: "f29dc596-2121-4421-8463-15f6c2e8b9b3",
    scryfall_id: "4493b62b-f354-47ff-9dcf-cc6e29de77c6",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Rogue's Passage",
        types: TypeSet::LAND,
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{4}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::UNBLOCKABLE,
                duration: Duration::UntilEndOfTurn,
            }],
            target: Some(TargetSpec::Object(&Filter::CREATURE))
        ),
    ],
}

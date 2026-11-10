//! Hidden Hideout — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add one mana of any color in your commander's color identity.
//! Oracle: {2}, {T}: Target creature you control with a counter on it gains lifelink until end of turn.
//! Set: TMC #43 — Teenage Mutant Ninja Turtles Eternal | Scryfall ID: f356522c-018d-43c7-988a-9ef9829ae75c | Oracle ID: b639e1fe-d099-4cab-a0d0-a1b33c7f31dd

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HIDDEN_HIDEOUT,
    oracle_id = "b639e1fe-d099-4cab-a0d0-a1b33c7f31dd",
    scryfall_id = "f356522c-018d-43c7-988a-9ef9829ae75c",
    faces = &[face!(
        name = "Hidden Hideout",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_commander_identity()]),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::ModifyTargets {
                modifier: Modifier::AddKeyword(KeywordSet::LIFELINK),
                duration: Duration::UntilEndOfTurn
            }],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::YOUR_CREATURE,
                Filter::HasAnyCounter
            ])))
        ),
    ],
);

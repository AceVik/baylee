//! Mirrorpool — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {2}{C}, {T}, Sacrifice this land: Copy target instant or sorcery spell you control. You may choose new targets for the copy.
//! Oracle: {4}{C}, {T}, Sacrifice this land: Create a token that's a copy of target creature you control.
//! Set: CMM #1010 — Commander Masters | Scryfall ID: 0441cd2c-3646-4c69-ae97-ad3bcea7466f | Oracle ID: 57b86d5c-3269-44bc-a838-3c5439d820d9
// IMPLEMENTED — enters tapped; {T} for {C}; and the two sacrifice-and-copy
// abilities (a spell you control, and a creature you control).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIRRORPOOL,
    oracle_id = "57b86d5c-3269-44bc-a838-3c5439d820d9",
    scryfall_id = "0441cd2c-3646-4c69-ae97-ad3bcea7466f",
    faces = &[face!(
        name = "Mirrorpool",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}{C}", TapSelf, SacrificeSelf),
            &[Effect::CopyTargetSpell { mods: &[] }],
            target = Some(TargetSpec::Spell(&f!(your INSTANT_OR_SORCERY))),
        ),
        activated!(
            cost!("{4}{C}", TapSelf, SacrificeSelf),
            &[Effect::CreateTokenCopyOf {
                target: Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
                kicked_bonus: 0,
            }],
            target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
        ),
    ],
);

//! Gossamer Chains — {W}{W} — Enchantment
//! Oracle: Return this enchantment to its owner's hand: Prevent all combat damage that would be dealt by target unblocked creature this turn.
//! Set: VIS #6 — Visions | Scryfall ID: e9917a29-c6b4-4e0a-a301-21868bd27e17 | Oracle ID: af3936ab-0156-438d-9fef-a924d2fcffc3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GOSSAMER_CHAINS,
    oracle_id = "af3936ab-0156-438d-9fef-a924d2fcffc3",
    scryfall_id = "e9917a29-c6b4-4e0a-a301-21868bd27e17",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gossamer Chains",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(ReturnSelfToHand),
        &[Effect::continuous(
            &Filter::This,
            Modifier::PreventDamageFromIt,
            Duration::UntilEndOfTurn,
        )],
        target = Some(TargetSpec::Object(&Filter::And(&[
            Filter::CREATURE,
            Filter::Unblocked,
        ]))),
    )],
);

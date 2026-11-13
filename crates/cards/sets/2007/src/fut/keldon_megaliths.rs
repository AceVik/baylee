//! Keldon Megaliths — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Oracle: Hellbent — {1}{R}, {T}: This land deals 1 damage to any target. Activate only if you have no cards in hand.
//! Set: JVC #58 — Duel Decks Anthology: Jace vs. Chandra | Scryfall ID: da7c4600-1dc3-4a9d-a112-1b70abcb8951 | Oracle ID: ec0ea7f7-52ce-40d1-b34c-e36dd4b26120
// IMPLEMENTED — the land arrives tapped, taps for {R}, and the hellbent
// ability is gated on the empty hand it prints.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KELDON_MEGALITHS,
    oracle_id = "ec0ea7f7-52ce-40d1-b34c-e36dd4b26120",
    scryfall_id = "da7c4600-1dc3-4a9d-a112-1b70abcb8951",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Keldon Megaliths",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        // "Hellbent" is an ability word and has no rules meaning, so the
        // gate is written as the sentence it stands for and nothing here
        // reads the word — the same bargain `Condition::GraveyardCountAtLeast`
        // makes with threshold on Barbarian Ring, which is otherwise this
        // same land.
        activated!(
            cost!("{1}{R}", TapSelf),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            }],
            target = Some(TargetSpec::AnyTarget),
            condition = Some(Condition::HandSizeAtMost(0)),
        ),
    ],
);

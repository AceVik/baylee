//! Recurring Nightmare — {2}{B} — Enchantment
//! Oracle: Sacrifice a creature, Return this enchantment to its owner's hand: Return target creature card from your graveyard to the battlefield. Activate only as a sorcery.
//! Set: TPR #113 — Tempest Remastered | Scryfall ID: b50e1800-a45c-43bd-8886-8a06145d9346 | Oracle ID: a6708b11-1bcd-4208-a967-fe91f2e3313c
// IMPLEMENTED — sorcery-speed reanimation, paid for by eating a creature and
// bouncing the enchantment. Both halves are cost and not effect, which is
// what the printed colon says: "Sacrifice a creature, Return this enchantment
// to its owner's hand:" — so the Nightmare is already in its owner's hand
// while the ability is on the stack, and an opponent has nothing on the
// battlefield to answer. Which creature is eaten is asked per activation by
// `engine::cost_wizard`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RECURRING_NIGHTMARE,
    oracle_id = "a6708b11-1bcd-4208-a967-fe91f2e3313c",
    scryfall_id = "b50e1800-a45c-43bd-8886-8a06145d9346",
    faces = &[face!(
        name = "Recurring Nightmare",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    )],
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!(Sacrifice(&Filter::YOUR_CREATURE), ReturnSelfToHand),
        &[Effect::reanimate(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You
        ))],
        target = Some(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You
        )),
        timing = ActivationTiming::SorcerySpeed
    )],
);

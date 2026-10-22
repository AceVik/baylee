//! Haunting Misery — {1}{B}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, exile X creature cards from your graveyard.
//! Oracle: Haunting Misery deals X damage to target player or planeswalker.
//! Set: WTH #71 — Weatherlight | Scryfall ID: 939b83ba-8ba8-4b98-8a13-a037ba7805e9 | Oracle ID: 8db6651b-c368-46fd-b9c2-69a4d6562246
// PARTIAL — Deals 1 damage to target player or planeswalker; exiling X creature cards from your graveyard as an additional spell cost to deal X damage is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HAUNTING_MISERY,
    oracle_id = "8db6651b-c368-46fd-b9c2-69a4d6562246",
    scryfall_id = "939b83ba-8ba8-4b98-8a13-a037ba7805e9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Deals 1 damage to target player or planeswalker; exiling X creature cards from your graveyard as an additional spell cost to deal X damage is not yet modeled."
    ),
    faces = &[face!(
        name = "Haunting Misery",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)
        }],
        targets = Some(TargetReq::one(TargetSpec::PlayerOrObject(
            &Filter::PLANESWALKER
        )))
    ),],
);

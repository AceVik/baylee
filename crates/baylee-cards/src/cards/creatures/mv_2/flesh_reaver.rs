//! Flesh Reaver — {1}{B} — Creature — Phyrexian Horror
//! Oracle: Whenever this creature deals damage to a creature or opponent, this creature deals that much damage to you.
//! Set: USG #136 — Urza's Saga | Scryfall ID: e3dc6a91-ca13-45da-ba65-8fbb16c159c0 | Oracle ID: a143fdfe-f1aa-4eab-8c81-8c72a4dc1695
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLESH_REAVER,
    oracle_id = "a143fdfe-f1aa-4eab-8c81-8c72a4dc1695",
    scryfall_id = "e3dc6a91-ca13-45da-ba65-8fbb16c159c0",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "triggering when Flesh Reaver deals damage to a creature is not in Trigger; triggers when Flesh Reaver deals damage to an opponent to deal that much damage to you",
    ),
    faces = &[face!(
        name = "Flesh Reaver",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::HORROR],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToOpponent(&Filter::This),
        &[Effect::DealDamage {
            target: TargetSpec::Player(PlayerRel::You),
            amount: Amount::EventAmount,
        }],
    )],
);

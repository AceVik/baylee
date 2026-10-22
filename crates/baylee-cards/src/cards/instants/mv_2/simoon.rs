//! Simoon — {R}{G} — Instant
//! Oracle: Simoon deals 1 damage to each creature target opponent controls.
//! Set: INV #272 — Invasion | Scryfall ID: 84b1930d-2e4b-472f-98a9-008fd632f3be | Oracle ID: 128e201a-f520-4917-a1a4-2f3836f1f92d
// PARTIAL: targets an opponent and deals 1 damage to each creature opponents control; scoping DealDamageEach to only the targeted opponent's creatures in multiplayer is not yet modeled in Filter.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SIMOON,
    oracle_id = "128e201a-f520-4917-a1a4-2f3836f1f92d",
    scryfall_id = "84b1930d-2e4b-472f-98a9-008fd632f3be",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Partial(
        "targets an opponent and deals 1 damage to each creature opponents control; scoping DealDamageEach to only the targeted opponent's creatures in multiplayer is not yet modeled in Filter",
    ),
    faces = &[face!(
        name = "Simoon",
        mana_cost = mana!("{R}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::damage_each(1, &Filter::OPPONENT_CREATURE)],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);

//! City of Brass — (no cost) — Land
//! Oracle: Whenever this land becomes tapped, it deals 1 damage to you.
//! Oracle: {T}: Add one mana of any color.
//! Set: TMC #62 — Teenage Mutant Ninja Turtles Eternal | Scryfall ID: c21565d0-fc40-4d89-9b27-87c03385e0af | Oracle ID: f25351e3-539b-4bbc-b92d-6480acf4d722
// IMPLEMENTED — any-color mana + becomes-tapped damage trigger
// (Trigger::BecomesTapped).

use baylee_cards_dsl::prelude::*;

card! {
    index: 20,
    oracle_id: "f25351e3-539b-4bbc-b92d-6480acf4d722",
    scryfall_id: "c21565d0-fc40-4d89-9b27-87c03385e0af",
    faces: &[face! {
        name: "City of Brass",
        types: TypeSet::LAND,
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::BecomesTapped(&Filter::This), &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            }]),
        mana_ability!(&[Effect::mana_choice(ALL_MANA_COLORS)]),
    ],
}

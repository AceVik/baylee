//! Sunken Hollow — (no cost) — Land
//! Oracle: ({T}: Add {U} or {B}.)
//! Oracle: This land enters tapped unless you control two or more basic lands.
//! Set: BFZ #249 — Battle for Zendikar | Scryfall ID: 3a8eef9b-9b03-42cd-a27a-07021bf0b33f | Oracle ID: cd2c90ac-2b04-461c-92f3-939871b6b6a3
// IMPLEMENTED — two-color mana choice, the way Taiga writes it: a land with
// two basic types gets no intrinsic ability from `casting::intrinsic_mana`,
// which refuses to pick a color on the player's behalf.
// DIVERGES FROM THE PRINTING, and knowingly: the printed condition counts
// basic lands and `EnterModifier` has no counting variant, so this enters
// tapped unless you control an Island or a Swamp — the same answer on most
// boards, wrong on one holding two Forests. The card is also printed
// `Land — Island Swamp` and carries neither subtype here, which is what a
// fetchland and every "you control an Island" would read. All ten BFZ battle
// lands are written this way, so both are one change across the cycle — with
// `Coverage::Partial` for the condition — and not a patch to this file.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static CHECK: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::LAND,
    Filter::Or(&[
        Filter::HasSubtype(land::ISLAND),
        Filter::HasSubtype(land::SWAMP),
    ]),
]);

card! {
    index: 159,
    oracle_id: "cd2c90ac-2b04-461c-92f3-939871b6b6a3",
    scryfall_id: "3a8eef9b-9b03-42cd-a27a-07021bf0b33f",
    faces: &[face! {
        name: "Sunken Hollow",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::TappedUnless(&CHECK)],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(&[ManaColor::Blue, ManaColor::Black])])],
}

//! Double Major — {G}{U} — Instant
//! Oracle: Copy target creature spell you control, except it isn't legendary if the spell is legendary. (A copy of a creature spell becomes a token.)
//! Set: STX #179 — Strixhaven: School of Mages | Scryfall ID: c3d35413-8742-4443-8859-93c91112978d | Oracle ID: ece44a82-dcf0-4439-bdd9-a09c99a6f159
// IMPLEMENTED — copies the creature spell and strips LEGENDARY from the copy
// (CR 707.10), so the legend rule does not eat it when both resolve.

static YOUR_CREATURE_SPELL: Filter = Filter::And(&[Filter::ControlledByYou, Filter::CREATURE]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 34,
    oracle_id: "ece44a82-dcf0-4439-bdd9-a09c99a6f159",
    scryfall_id: "c3d35413-8742-4443-8859-93c91112978d",
    faces: &[face! {
        name: "Double Major",
        mana_cost: baylee_core::mana!("{G}{U}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::CopyTargetSpell {
            mods: &[CopyMod::RemoveSupertype(
                SupertypeSet::LEGENDARY,
            )],
        }], targets: Some(TargetReq::one(TargetSpec::Spell(&YOUR_CREATURE_SPELL))))],
}

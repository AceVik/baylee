//! Mouth of Ronom — (no cost) — Snow Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}{S}, {T}, Sacrifice this land: It deals 4 damage to target creature. ({S} can be paid with one mana from a snow source.)
//! Set: CSP #148 — Coldsnap | Scryfall ID: 2d8f7396-23d3-48e2-8686-2b59ccbfc7e5 | Oracle ID: 7c05d239-39fc-4d34-a853-e3d591f4a235
// IMPLEMENTED — {T} for {C}; and the {4}{S}, {T}, sacrifice-this-land ability
// dealing 4 damage to a target creature (snow mana is a `mana!` symbol).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOUTH_OF_RONOM,
    oracle_id = "7c05d239-39fc-4d34-a853-e3d591f4a235",
    scryfall_id = "2d8f7396-23d3-48e2-8686-2b59ccbfc7e5",
    faces = &[face!(
        name = "Mouth of Ronom",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::SNOW,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}{S}", TapSelf, SacrificeSelf),
            &[Effect::DealDamage {
                amount: Amount::Fixed(4),
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);

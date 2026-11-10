//! Mech Hangar — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a Pilot or Vehicle spell.
//! Oracle: {3}, {T}: Target Vehicle becomes an artifact creature until end of turn.
//! Set: NEO #270 — Kamigawa: Neon Dynasty | Scryfall ID: c093984d-38cd-4b49-b179-1e289ab442d1 | Oracle ID: abc04775-171d-41f3-83ea-4b4eb72723d5
// IMPLEMENTED — {C}, any-color mana restricted to Pilot/Vehicle spells, and {3}, {T} animates target Vehicle into an artifact creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PILOT_OR_VEHICLE: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::creature::PILOT),
    Filter::HasSubtype(subtypes::artifact::VEHICLE),
]);

card!(
    index = index::MECH_HANGAR,
    oracle_id = "abc04775-171d-41f3-83ea-4b4eb72723d5",
    scryfall_id = "c093984d-38cd-4b49-b179-1e289ab442d1",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Mech Hangar", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_of_any_color().restricted(&PILOT_OR_VEHICLE, SpendRider::None)
        ]),
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::continuous(
                &Filter::This,
                Modifier::AddType(TypeSet::ARTIFACT.union(TypeSet::CREATURE)),
                Duration::UntilEndOfTurn,
            )],
            target = Some(TargetSpec::Object(&Filter::HasSubtype(
                subtypes::artifact::VEHICLE
            ))),
        ),
    ],
);

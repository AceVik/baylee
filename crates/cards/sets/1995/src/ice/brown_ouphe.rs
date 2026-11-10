//! Brown Ouphe — {G} — Creature — Ouphe
//! Oracle: {1}{G}, {T}: Counter target activated ability from an artifact source. (Mana abilities can't be targeted.)
//! Set: MRD #115 — Mirrodin | Scryfall ID: 1d8779ec-0ebd-45a1-96c9-4b1146871ab7 | Oracle ID: 5b42e91a-e3bd-4f69-9abf-ed6273715fff
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ARTIFACT_ACTIVATED_ABILITY: Filter = Filter::ActivatedAbilityFrom(&Filter::ARTIFACT);

card!(
    index = index::BROWN_OUPHE,
    oracle_id = "5b42e91a-e3bd-4f69-9abf-ed6273715fff",
    scryfall_id = "1d8779ec-0ebd-45a1-96c9-4b1146871ab7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Brown Ouphe",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OUPHE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{G}", TapSelf),
        &[Effect::CounterTargetAbility],
        target = Some(TargetSpec::AbilityOnStack(&ARTIFACT_ACTIVATED_ABILITY)),
    )],
);

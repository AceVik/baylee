//! Aladdin — {2}{R}{R} — Creature — Human Rogue
//! Oracle: {1}{R}{R}, {T}: Gain control of target artifact for as long as you control this creature.
//! Set: ME4 #106 — Masters Edition IV | Scryfall ID: 462094ff-5ec8-4a9b-8ab1-012c5f7d85ca | Oracle ID: 9a410f83-ed92-4b55-834a-c7cec8f5d1e2
// IMPLEMENTED — the {1}{R}{R}, {T} ability gives its controller a layer-2
// `GainControl` over a target artifact for as long as they control Aladdin.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ALADDIN,
    oracle_id = "9a410f83-ed92-4b55-834a-c7cec8f5d1e2",
    scryfall_id = "462094ff-5ec8-4a9b-8ab1-012c5f7d85ca",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Aladdin",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{1}{R}{R}", TapSelf),
        &[Effect::continuous(
            &Filter::This,
            Modifier::GainControl,
            Duration::WhileYouControlSource,
        )],
        target = Some(TargetSpec::Object(&Filter::ARTIFACT)),
    )],
);

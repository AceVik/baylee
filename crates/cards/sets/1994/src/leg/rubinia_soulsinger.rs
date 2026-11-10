//! Rubinia Soulsinger — {2}{G}{W}{U} — Legendary Creature — Faerie
//! Oracle: You may choose not to untap Rubinia Soulsinger during your untap step.
//! Oracle: {T}: Gain control of target creature for as long as you control Rubinia Soulsinger and Rubinia Soulsinger remains tapped.
//! Set: CMA #191 — Commander Anthology | Scryfall ID: ce216785-aa81-44b4-9562-9b7eca743a0c | Oracle ID: bd3eeaba-964b-49ea-bb11-5875a78b8a4c

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RUBINIA_SOULSINGER,
    oracle_id = "bd3eeaba-964b-49ea-bb11-5875a78b8a4c",
    scryfall_id = "ce216785-aa81-44b4-9562-9b7eca743a0c",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Rubinia Soulsinger",
        mana_cost = mana!("{2}{G}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::FAERIE],
        power = Some(2),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            Cost::TAP,
            &[Effect::ModifyTargets {
                modifier: Modifier::GainControl,
                duration: Duration::WhileSourceTapped {
                    control: true,
                    power_limited: false
                }
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE))
        ),
    ],
);

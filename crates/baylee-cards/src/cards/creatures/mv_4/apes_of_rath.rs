//! Apes of Rath — {2}{G}{G} — Creature — Ape
//! Oracle: Whenever this creature attacks, it doesn't untap during its controller's next untap step.
//! Set: TMP #214 — Tempest | Scryfall ID: 25eff287-6b53-4e6d-9da2-d80d05bb8c51 | Oracle ID: b52f5850-f1aa-412a-bae9-2ce76c287188
// IMPLEMENTED — Whenever this creature attacks, it doesn't untap during its controller's next untap step.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::APES_OF_RATH,
    oracle_id = "b52f5850-f1aa-412a-bae9-2ce76c287188",
    scryfall_id = "25eff287-6b53-4e6d-9da2-d80d05bb8c51",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Apes of Rath",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::APE],
        power = Some(5),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::continuous(
            &Filter::This,
            Modifier::DoesNotUntap,
            Duration::UntilYourNextUntapStep,
        )]
    )],
);

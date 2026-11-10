//! Brass Man — {1} — Artifact Creature — Construct
//! Oracle: This creature doesn't untap during your untap step.
//! Oracle: At the beginning of your upkeep, you may pay {1}. If you do, untap this creature.
//! Set: ME4 #185 — Masters Edition IV | Scryfall ID: 82b0ff45-b98c-4e45-9a05-bbbdaaa62d3e | Oracle ID: 64f56228-7874-4465-ba58-1049083ea02f
// IMPLEMENTED — the untap suppression is a static layer-6 `DoesNotUntap`, and
// the upkeep's optional {1} is a payment asked as the trigger resolves.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRASS_MAN,
    oracle_id = "64f56228-7874-4465-ba58-1049083ea02f",
    scryfall_id = "82b0ff45-b98c-4e45-9a05-bbbdaaa62d3e",
    faces = &[face!(
        name = "Brass Man",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(1),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayThen {
                player: PlayerRel::You,
                mana: Amount::Fixed(1),
                effects: &[Effect::UntapSelf],
            }],
        ),
    ],
);

//! Rabbit Battery — {R} — Artifact Creature — Equipment Rabbit
//! Oracle: Haste
//! Oracle: Equipped creature gets +1/+1 and has haste.
//! Oracle: Reconfigure {R} ({R}: Attach to target creature you control; or unattach from a creature. Reconfigure only as a sorcery. While attached, this isn't a creature.)
//! Set: NEO #157 — Kamigawa: Neon Dynasty | Scryfall ID: 5d33a5b7-797b-4079-8d62-edd124c0fb5a | Oracle ID: c739e180-2f14-41ed-8e7e-50b7df985f35

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RABBIT_BATTERY,
    oracle_id = "c739e180-2f14-41ed-8e7e-50b7df985f35",
    scryfall_id = "5d33a5b7-797b-4079-8d62-edd124c0fb5a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::HASTE,
    faces = &[face!(
        name = "Rabbit Battery",
        mana_cost = mana!("{R}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::artifact::EQUIPMENT, subtypes::creature::RABBIT],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, 1)),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddKeyword(KeywordSet::HASTE)
        ),
        activated!(
            cost!("{R}"),
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::ANOTHER_CREATURE_YOU_CONTROL)
            }],
            target = Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE_YOU_CONTROL)),
            timing = ActivationTiming::SorcerySpeed
        ),
        static_ability!(
            Filter::This,
            Modifier::RemoveType(TypeSet::CREATURE),
            condition = Some(Condition::SourceMatches(&Filter::IsAttached))
        ),
        activated!(
            cost!("{R}"),
            &[Effect::DetachSource],
            timing = ActivationTiming::SorcerySpeed,
            condition = Some(Condition::SourceMatches(&Filter::IsAttached))
        ),
    ],
);

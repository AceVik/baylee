//! Artifact Ward — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be blocked by artifact creatures.
//! Oracle: Prevent all damage that would be dealt to enchanted creature by artifact sources.
//! Oracle: Enchanted creature can't be the target of abilities from artifact sources.
//! Set: ATQ #3 — Antiquities | Scryfall ID: b3a5101a-ec66-4658-950c-9ad49c29b836 | Oracle ID: 9bd3a4bb-cc12-4e5f-a33f-77ab0c7788db
// IMPLEMENTED — enchant creature; the enchanted creature can't be blocked by
// artifact creatures, damage from artifact sources to it is prevented, and
// abilities from artifact sources can't target it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Artifact creature" — the blocker the second printed sentence names.
static ARTIFACT_CREATURE: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CREATURE]);

/// The enchanted creature.
static ENCHANTED: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::ARTIFACT_WARD,
    oracle_id = "9bd3a4bb-cc12-4e5f-a33f-77ab0c7788db",
    scryfall_id = "b3a5101a-ec66-4658-950c-9ad49c29b836",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Artifact Ward",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(ENCHANTED, Modifier::CantBeBlockedBy(&ARTIFACT_CREATURE)),
        static_ability!(ENCHANTED, Modifier::PreventDamageFrom(&Filter::ARTIFACT)),
        static_ability!(
            ENCHANTED,
            Modifier::CantBeTargetedByAbilitiesFrom(&Filter::ARTIFACT)
        ),
    ],
);

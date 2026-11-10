//! Mirrorhall Mimic // Ghastly Mimicry — {3}{U} — Creature — Spirit // Enchantment — Aura
//! Oracle: You may have this creature enter as a copy of any creature on the battlefield, except it's a Spirit in addition to its other types.
//! Oracle: Disturb {3}{U}{U} (You may cast this card from your graveyard transformed for its disturb cost.)
//! Oracle: Enchant creature
//! Oracle: At the beginning of your upkeep, create a token that's a copy of enchanted creature, except it's a Spirit in addition to its other types.
//! Oracle: If Ghastly Mimicry would be put into a graveyard from anywhere, exile it instead.
//! Set: VOW #68 — Innistrad: Crimson Vow | Scryfall ID: 823ad188-bd56-476d-9853-bed90bfad582 | Oracle ID: 5768fe50-a134-492c-a725-5ed02610c39f
// IMPLEMENTED — clone front + disturb; Ghastly Mimicry enchants a creature,
// copies it each upkeep as a Spirit, and is exiled instead of going to a
// graveyard from the stack or the battlefield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{creature, enchantment};

static GHASTLY_MIMICRY: &[AbilityDef] = &[
    spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE)
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),
    triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::CreateTokenCopyOfEquipped {
            kicked_bonus: 0,
            mods: &[CopyMod::AddSubtype(creature::SPIRIT)],
        }]
    ),
    AbilityDef::Replacement(ReplacementRule::ExileSelfInsteadOfGraveyard),
];

card!(
    index = index::MIRRORHALL_MIMIC,
    oracle_id = "5768fe50-a134-492c-a725-5ed02610c39f",
    scryfall_id = "823ad188-bd56-476d-9853-bed90bfad582",
    faces = &[
        face!(
            name = "Mirrorhall Mimic",
            mana_cost = mana!("{3}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[creature::SPIRIT],
            power = Some(0),
            toughness = Some(0),
        ),
        face!(
            name = "Ghastly Mimicry",
            mana_cost = mana!("{3}{U}{U}"),
            types = TypeSet::ENCHANTMENT,
            subtypes = &[enchantment::AURA],
            castable_from_hand = false, // disturb: cast from the graveyard
            disturb = true,
            abilities = GHASTLY_MIMICRY,
        ),
    ],
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    abilities = &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::CREATURE),
        mods: &[CopyMod::AddSubtype(creature::SPIRIT)],
    }],
);

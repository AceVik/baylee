//! Grave Servitude — {1}{B} — Enchantment — Aura
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +3/-1 and is black.
//! Set: MIR #126 — Mirage | Scryfall ID: fbe072da-0524-492b-af3d-7c2600e915ab | Oracle ID: 6f54e7ea-bb87-4ce0-96b6-4fc82605ed2a
// NOT SUPPORTED: sacrificing the Aura at the beginning of the next cleanup step if cast any time a sorcery couldn't have been cast.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::GRAVE_SERVITUDE,
    oracle_id = "6f54e7ea-bb87-4ce0-96b6-4fc82605ed2a",
    scryfall_id = "fbe072da-0524-492b-af3d-7c2600e915ab",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "has flash, enchants target creature, and gives enchanted creature +3/-1 and makes it black; sacrificing it at the beginning of the next cleanup step if cast any time a sorcery couldn't have been cast is not supported",
    ),
    faces = &[face!(
        name = "Grave Servitude",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        keywords = KeywordSet::FLASH,
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(ENCHANTED_CREATURE, Modifier::ModifyPT(3, -1)),
        static_ability!(
            ENCHANTED_CREATURE,
            Modifier::SetColor(ColorSet::from_slice(&[Color::Black])),
        ),
    ],
);

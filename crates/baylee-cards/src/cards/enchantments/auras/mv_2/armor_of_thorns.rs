//! Armor of Thorns — {1}{G} — Enchantment — Aura
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Enchant nonblack creature
//! Oracle: Enchanted creature gets +2/+2.
//! Set: VMA #194 — Vintage Masters | Scryfall ID: c32d3e47-8c7c-4bfd-8b3a-b0751d68dc42 | Oracle ID: 31ead5a5-aa01-4601-8d3e-267b65fce3bb
// NOT SUPPORTED: sacrificing the Aura at the beginning of the next cleanup step if cast any time a sorcery couldn't have been cast.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NONBLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);
static ENCHANTED_NONBLACK_CREATURE: Filter =
    Filter::And(&[NONBLACK_CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::ARMOR_OF_THORNS,
    oracle_id = "31ead5a5-aa01-4601-8d3e-267b65fce3bb",
    scryfall_id = "c32d3e47-8c7c-4bfd-8b3a-b0751d68dc42",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "has flash, enchants target nonblack creature, and gives enchanted creature +2/+2; sacrificing it at the beginning of the next cleanup step if cast any time a sorcery couldn't have been cast is not supported",
    ),
    faces = &[face!(
        name = "Armor of Thorns",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        keywords = KeywordSet::FLASH,
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&NONBLACK_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&NONBLACK_CREATURE))),
        ),
        static_ability!(ENCHANTED_NONBLACK_CREATURE, Modifier::ModifyPT(2, 2)),
    ],
);

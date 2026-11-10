//! Armor of Thorns — {1}{G} — Enchantment — Aura
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Enchant nonblack creature
//! Oracle: Enchanted creature gets +2/+2.
//! Set: VMA #194 — Vintage Masters | Scryfall ID: c32d3e47-8c7c-4bfd-8b3a-b0751d68dc42 | Oracle ID: 31ead5a5-aa01-4601-8d3e-267b65fce3bb
// PARTIAL — a sorcery-speed Aura: enchant nonblack creature, enchanted creature gets +2/+2.
// NOT SUPPORTED: "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step." Flash without the sacrifice would be a free instant-speed +2/+2, and no Condition says "cast when a sorcery couldn't have been cast", so the flash is left out.

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
        "Enchant nonblack creature and +2/+2 work; casting it as though it had flash, with the sacrifice at the next cleanup step when cast at a time a sorcery couldn't have been cast, is not played, so the Aura can only be cast at sorcery speed.",
    ),
    faces = &[face!(
        name = "Armor of Thorns",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
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

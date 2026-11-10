//! Soar — {1}{U} — Enchantment — Aura
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +0/+1 and has flying.
//! Set: MIR #93 — Mirage | Scryfall ID: bd3666dd-a90e-43c2-bd78-ea9c1af08a0e | Oracle ID: acee6f4f-8b66-495d-bb0e-71c339e7dc49
// NOT SUPPORTED: "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step." Flash is left off on purpose: the DSL cannot say the sacrifice rider (the cast timing is not remembered), and flash without it would be stronger than printed. The Aura casts at sorcery speed only, where the printed rider never applies.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::SOAR,
    oracle_id = "acee6f4f-8b66-495d-bb0e-71c339e7dc49",
    scryfall_id = "bd3666dd-a90e-43c2-bd78-ea9c1af08a0e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "enchants a creature and gives it +0/+1 and flying, cast at sorcery speed only; casting it as though it had flash, with the sacrifice at the next cleanup step when it is cast at a time a sorcery couldn't have been cast, is not supported (flash is left off, since without the sacrifice it would be stronger than printed)",
    ),
    faces = &[face!(
        name = "Soar",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(ENCHANTED_CREATURE, Modifier::ModifyPT(0, 1)),
        static_ability!(ENCHANTED_CREATURE, Modifier::AddKeyword(KeywordSet::FLYING)),
    ],
);

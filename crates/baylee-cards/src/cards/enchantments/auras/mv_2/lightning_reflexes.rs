//! Lightning Reflexes — {1}{R} — Enchantment — Aura
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+0 and has first strike.
//! Set: DMR #128 — Dominaria Remastered | Scryfall ID: 9653f159-3f56-4446-bb6d-b4b4ee0cd61d | Oracle ID: 2b0c82f1-078a-4e8b-8e43-407f5c5d8549
// NOT SUPPORTED: "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step." Flash is left off on purpose: the DSL cannot say the sacrifice rider (the cast timing is not remembered), and flash without it would be stronger than printed. The Aura casts at sorcery speed only, where the printed rider never applies.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::LIGHTNING_REFLEXES,
    oracle_id = "2b0c82f1-078a-4e8b-8e43-407f5c5d8549",
    scryfall_id = "9653f159-3f56-4446-bb6d-b4b4ee0cd61d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "enchants target creature and gives it +1/+0 and first strike, cast at sorcery speed only; casting it as though it had flash, with the sacrifice at the next cleanup step when it is cast at a time a sorcery couldn't have been cast, is not supported (flash is left off, since without the sacrifice it would be stronger than printed)",
    ),
    faces = &[face!(
        name = "Lightning Reflexes",
        mana_cost = mana!("{1}{R}"),
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
        static_ability!(ENCHANTED_CREATURE, Modifier::ModifyPT(1, 0)),
        static_ability!(
            ENCHANTED_CREATURE,
            Modifier::AddKeyword(KeywordSet::FIRST_STRIKE)
        ),
    ],
);

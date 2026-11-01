//! Mystic Veil — {1}{U} — Enchantment — Aura
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has shroud. (It can't be the target of spells or abilities.)
//! Set: VIS #38 — Visions | Scryfall ID: 7ddb640d-5c54-4d0a-b8c2-e22fe04f96c2 | Oracle ID: 43c88fcd-aac9-4741-8fd6-31ac81c5bb6f
// PARTIAL — an Aura that gives the enchanted creature shroud, cast at sorcery speed only; the flash and its sacrifice rider are not supported.
// NOT SUPPORTED: "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step." Flash is left off on purpose: the DSL cannot say the sacrifice rider (the cast timing is not remembered), and flash without it would be stronger than printed. The Aura casts at sorcery speed only, where the printed rider never applies.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MYSTIC_VEIL,
    oracle_id = "43c88fcd-aac9-4741-8fd6-31ac81c5bb6f",
    scryfall_id = "7ddb640d-5c54-4d0a-b8c2-e22fe04f96c2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "enchants a creature and gives it shroud, cast at sorcery speed only; casting it as though it had flash, with the sacrifice at the next cleanup step when it is cast at a time a sorcery couldn't have been cast, is not supported (flash is left off, since without the sacrifice it would be stronger than printed)"
    ),
    faces = &[face!(
        name = "Mystic Veil",
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
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::AddKeyword(KeywordSet::SHROUD),
        ),
    ],
);

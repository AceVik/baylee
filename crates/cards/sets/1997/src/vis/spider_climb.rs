//! Spider Climb — {G} — Enchantment — Aura
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +0/+3 and has reach. (It can block creatures with flying.)
//! Set: VIS #120 — Visions | Scryfall ID: a1818812-4cb8-4fe1-98c0-b40086b4991c | Oracle ID: 2636aafa-83f9-4b72-b729-04e1c3aa0eec
// PARTIAL — an Aura that gives the enchanted creature +0/+3 and reach, cast at sorcery speed only; the flash and its sacrifice rider are not supported.
// NOT SUPPORTED: "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step." Flash is left off on purpose: the DSL cannot say the sacrifice rider (the cast timing is not remembered), and flash without it would be stronger than printed. The Aura casts at sorcery speed only, where the printed rider never applies.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::SPIDER_CLIMB,
    oracle_id = "2636aafa-83f9-4b72-b729-04e1c3aa0eec",
    scryfall_id = "a1818812-4cb8-4fe1-98c0-b40086b4991c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "enchants a creature and gives it +0/+3 and reach, cast at sorcery speed only; casting it as though it had flash, with the sacrifice at the next cleanup step when it is cast at a time a sorcery couldn't have been cast, is not supported (flash is left off, since without the sacrifice it would be stronger than printed)",
    ),
    faces = &[face!(
        name = "Spider Climb",
        mana_cost = mana!("{G}"),
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
        static_ability!(ENCHANTED_CREATURE, Modifier::ModifyPT(0, 3)),
        static_ability!(ENCHANTED_CREATURE, Modifier::AddKeyword(KeywordSet::REACH)),
    ],
);

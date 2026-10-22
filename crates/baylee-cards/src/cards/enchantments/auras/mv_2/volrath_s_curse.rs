//! Volrath's Curse — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't attack or block, and its activated abilities can't be activated. That creature's controller may sacrifice a permanent of their choice for that player to ignore this effect until end of turn.
//! Oracle: {1}{U}: Return this Aura to its owner's hand.
//! Set: TPR #77 — Tempest Remastered | Scryfall ID: c80be3e6-37da-43bd-b959-9dbfead3baa1 | Oracle ID: f4ff3efb-c9ed-4451-a7f2-5ff52ffb6cab
// PARTIAL — Enchanted creature can't attack or block and its activated abilities can't be activated, and {1}{U} returns this Aura to its owner's hand; the special action allowing the enchanted creature's controller to sacrifice a permanent to ignore this effect until end of turn is not in the DSL.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::VOLRATH_S_CURSE,
    oracle_id = "f4ff3efb-c9ed-4451-a7f2-5ff52ffb6cab",
    scryfall_id = "c80be3e6-37da-43bd-b959-9dbfead3baa1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the special action allowing the enchanted creature's controller to sacrifice a permanent to ignore Volrath's Curse's effect until end of turn is not in the DSL; the Aura prevents the enchanted creature from attacking, blocking, or activating abilities and has {1}{U} to return itself to its owner's hand"
    ),
    faces = &[face!(
        name = "Volrath's Curse",
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
        static_ability!(ENCHANTED_CREATURE, Modifier::CantAttack),
        static_ability!(ENCHANTED_CREATURE, Modifier::CantBlock(&Filter::CREATURE)),
        static_ability!(ENCHANTED_CREATURE, Modifier::CantActivate),
        activated!(
            cost!("{1}{U}"),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        ),
    ],
);

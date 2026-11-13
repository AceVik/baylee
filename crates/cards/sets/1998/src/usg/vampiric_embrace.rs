//! Vampiric Embrace — {2}{B}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/+2 and has flying.
//! Oracle: Whenever a creature dealt damage by enchanted creature this turn dies, put a +1/+1 counter on that creature.
//! Set: USG #164 — Urza's Saga | Scryfall ID: 889be765-5716-4549-b544-0a49d3962e16 | Oracle ID: c7a50ece-4fe4-4e54-9319-b92a4144445d
// PARTIAL: triggering when a creature dealt damage by enchanted creature this turn dies to put a +1/+1 counter on enchanted creature is not in Trigger; enchants a creature and grants +2/+2 and flying.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::VAMPIRIC_EMBRACE,
    oracle_id = "c7a50ece-4fe4-4e54-9319-b92a4144445d",
    scryfall_id = "889be765-5716-4549-b544-0a49d3962e16",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "triggering when a creature dealt damage by enchanted creature this turn dies to put a +1/+1 counter on enchanted creature is not in Trigger; enchants a creature and grants +2/+2 and flying",
    ),
    faces = &[face!(
        name = "Vampiric Embrace",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: triggering when a creature dealt damage by enchanted creature this turn dies to put a +1/+1 counter on enchanted creature is not in Trigger; enchants a creature and grants +2/+2 and flying.
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(ENCHANTED_CREATURE, Modifier::ModifyPT(2, 2)),
        static_ability!(ENCHANTED_CREATURE, Modifier::AddKeyword(KeywordSet::FLYING)),
    ],
);

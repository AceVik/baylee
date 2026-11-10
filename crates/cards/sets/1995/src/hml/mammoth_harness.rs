//! Mammoth Harness — {3}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature loses flying.
//! Oracle: Whenever enchanted creature blocks or becomes blocked by a creature, the other creature gains first strike until end of turn.
//! Set: HML #91 — Homelands | Scryfall ID: 5be67121-068c-4770-bc42-c081577a442c | Oracle ID: f2e106c6-7666-42cd-9a02-e51327fb80e8
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MAMMOTH_HARNESS,
    oracle_id = "f2e106c6-7666-42cd-9a02-e51327fb80e8",
    scryfall_id = "5be67121-068c-4770-bc42-c081577a442c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mammoth Harness",
        mana_cost = mana!("{3}{G}"),
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
            Filter::AttachedToBySource,
            Modifier::RemoveKeyword(KeywordSet::FLYING)
        ),
        triggered!(
            Trigger::AttachedBlocksOrBecomesBlockedBy(&Filter::CREATURE),
            &[Effect::ModifyEvent {
                modifier: Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
                duration: Duration::UntilEndOfTurn,
            }],
        ),
    ],
);

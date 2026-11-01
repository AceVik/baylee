//! Veteran's Voice — {R} — Enchantment — Aura
//! Oracle: Enchant creature you control
//! Oracle: Tap enchanted creature: Target creature other than the creature tapped this way gets +2/+1 until end of turn. Activate only if enchanted creature is untapped.
//! Set: ALL #84a — Alliances | Scryfall ID: 93babf85-eb13-4e06-b45b-8927791bcde5 | Oracle ID: 68d25ba4-d710-4090-bceb-5cfbd6bb63a1
// IMPLEMENTED — Enchant creature you control; tap enchanted creature: target other creature gets +2/+1 until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);
static OTHER_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::AttachedToBySource)]);

card!(
    index = index::VETERAN_S_VOICE,
    oracle_id = "68d25ba4-d710-4090-bceb-5cfbd6bb63a1",
    scryfall_id = "93babf85-eb13-4e06-b45b-8927791bcde5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Veteran's Voice",
        mana_cost = mana!("{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        ),
        activated!(
            cost!(TapOther(&ENCHANTED_CREATURE)),
            &[Effect::PumpTarget {
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&OTHER_CREATURE)),
        ),
    ],
);

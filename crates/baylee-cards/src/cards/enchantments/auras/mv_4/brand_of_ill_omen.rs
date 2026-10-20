//! Brand of Ill Omen — {3}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Cumulative upkeep {R} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Enchanted creature's controller can't cast creature spells.
//! Set: ICE #177 — Ice Age | Scryfall ID: ceeb7bbc-2d41-4709-95be-1ceb952ed1fb | Oracle ID: 947bd180-ee95-4db4-b5d9-432428018eb8

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "Enchanted creature's controller can't cast creature spells." —
// the DSL has no static cast restriction scoped to the enchanted permanent's controller.
card!(
    index = index::BRAND_OF_ILL_OMEN,
    oracle_id = "947bd180-ee95-4db4-b5d9-432428018eb8",
    scryfall_id = "ceeb7bbc-2d41-4709-95be-1ceb952ed1fb",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage =
        Coverage::Partial("missing static cast restriction on enchanted creature's controller"),
    faces = &[face!(
        name = "Brand of Ill Omen",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1),
                },
                Effect::PlayerMayPayManaEachThenElse {
                    player: PlayerRel::You,
                    each: mana!("{R}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield
            )))
        ),
    ],
);

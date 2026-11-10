//! Betrothed of Fire — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Sacrifice an untapped creature: Enchanted creature gets +2/+0 until end of turn.
//! Oracle: Sacrifice enchanted creature: Creatures you control get +2/+0 until end of turn.
//! Set: WTH #89 — Weatherlight | Scryfall ID: 5e517aa4-d8ba-4a49-bf9f-172bf029fa52 | Oracle ID: 77849657-c872-4d54-9149-a08bbe90f787
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static COST1: Filter = Filter::And(&[Filter::CREATURE, Filter::Untapped, Filter::ControlledByYou]);

static COST2: Filter = Filter::And(&[Filter::AttachedToBySource, Filter::ControlledByYou]);

card!(
    index = index::BETROTHED_OF_FIRE,
    oracle_id = "77849657-c872-4d54-9149-a08bbe90f787",
    scryfall_id = "5e517aa4-d8ba-4a49-bf9f-172bf029fa52",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Betrothed of Fire",
        mana_cost = mana!("{1}{R}"),
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
        activated!(
            cost!(Sacrifice(&COST1)),
            &[Effect::PumpFilter {
                filter: &Filter::AttachedToBySource,
                controlled_by: None,
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn
            }]
        ),
        activated!(
            cost!(Sacrifice(&COST2)),
            &[Effect::PumpFilter {
                filter: &Filter::YOUR_CREATURE,
                controlled_by: None,
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn
            }]
        ),
    ],
);

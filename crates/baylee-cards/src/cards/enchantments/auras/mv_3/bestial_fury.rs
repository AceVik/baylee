//! Bestial Fury — {2}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When this Aura enters, draw a card at the beginning of the next turn's upkeep.
//! Oracle: Whenever enchanted creature becomes blocked, it gets +4/+0 and gains trample until end of turn.
//! Set: ME1 #88 — Masters Edition | Scryfall ID: e4820e94-0dd8-42c1-b606-331442b76111 | Oracle ID: dfc08963-132f-4ebf-9384-15d17d1098f7
// NOT SUPPORTED: the Aura's own "whenever enchanted creature becomes blocked" trigger (once, however many block): it is granted to enchanted creature instead, so that creature's controller controls the trigger and it is lost when the creature loses its abilities.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BESTIAL_FURY,
    oracle_id = "dfc08963-132f-4ebf-9384-15d17d1098f7",
    scryfall_id = "e4820e94-0dd8-42c1-b606-331442b76111",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "attaches to target creature, draws a card at the beginning of the next turn's upkeep when it enters, and grants enchanted creature a trigger that gives it +4/+0 and trample until end of turn whenever it becomes blocked; the becomes-blocked trigger is granted to the enchanted creature instead of being the Aura's own, so the creature's controller controls it and it is lost when the creature loses its abilities",
    ),
    faces = &[face!(
        name = "Bestial Fury",
        mana_cost = mana!("{2}{R}"),
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
        triggered!(
            Trigger::ETB,
            &[Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            }],
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantTriggered {
                trigger: Trigger::BecomesBlocked,
                effects: &[Effect::PumpFilter {
                    filter: &Filter::This,
                    controlled_by: None,
                    power: Amount::Fixed(4),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::TRAMPLE,
                    duration: Duration::UntilEndOfTurn,
                }],
                target: None,
            },
        ),
    ],
);

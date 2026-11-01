//! Mindbender Spores — {2}{G} — Creature — Fungus Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Flying
//! Oracle: Whenever this creature blocks a creature, put four fungus counters on that creature. The creature gains "This creature doesn't untap during your untap step if it has a fungus counter on it" and "At the beginning of your upkeep, remove a fungus counter from this creature."
//! Set: MIR #229 — Mirage | Scryfall ID: f41066eb-be5e-4a6d-9156-64f5e56c7ab3 | Oracle ID: 0e3b7ec1-3760-4ac4-81ef-1ab389392f1b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NO_UNTAP_WITH_FUNGUS: Filter =
    Filter::And(&[Filter::This, Filter::HasCounter(counters::FUNGUS)]);

card!(
    index = index::MINDBENDER_SPORES,
    oracle_id = "0e3b7ec1-3760-4ac4-81ef-1ab389392f1b",
    scryfall_id = "f41066eb-be5e-4a6d-9156-64f5e56c7ab3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::DEFENDER.union(KeywordSet::FLYING),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mindbender Spores",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FUNGUS, subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksCreature(&Filter::CREATURE),
        &[
            Effect::AddCounter {
                kind: counters::FUNGUS,
                amount: Amount::Fixed(4),
            },
            Effect::continuous(
                &Filter::Reference,
                Modifier::GrantStatic {
                    filter: &NO_UNTAP_WITH_FUNGUS,
                    modifier: &Modifier::DoesNotUntap,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(
                &Filter::Reference,
                Modifier::GrantTriggered {
                    trigger: Trigger::StepBegin {
                        step: StepKind::Upkeep,
                        whose: PlayerRel::You,
                    },
                    effects: &[Effect::RemoveCounterSelf {
                        kind: counters::FUNGUS,
                        n: 1,
                    }],
                    target: None,
                },
                Duration::Indefinitely,
            ),
        ],
        targets = Some(TargetReq::one(TargetSpec::EventObject)),
    )],
);

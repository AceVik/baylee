//! Phantasmal Sphere — {1}{U} — Creature — Illusion
//! Oracle: Flying
//! Oracle: At the beginning of your upkeep, put a +1/+1 counter on this creature, then sacrifice this creature unless you pay {1} for each +1/+1 counter on it.
//! Oracle: When this creature leaves the battlefield, target opponent creates an X/X blue Orb creature token with flying, where X is the number of +1/+1 counters on this creature.
//! Set: ALL #32 — Alliances | Scryfall ID: a84617c7-c70a-497c-b834-3d98346180cf | Oracle ID: 4230c7a3-4591-411e-afab-d2bfb58446de
// NOT SUPPORTED: creating an X/X blue Orb creature token with flying under target opponent's control when Phantasmal Sphere leaves the battlefield requires a variable-P/T Orb token definition and targeted-opponent dynamic token creation.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHANTASMAL_SPHERE,
    oracle_id = "4230c7a3-4591-411e-afab-d2bfb58446de",
    scryfall_id = "a84617c7-c70a-497c-b834-3d98346180cf",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "Creating an X/X blue Orb creature token with flying under target opponent's control when Phantasmal Sphere leaves the battlefield, where X is the number of +1/+1 counters on it, requires a variable-P/T Orb token definition and targeted-opponent dynamic token creation; in play, Phantasmal Sphere has flying and its upkeep ability that puts a +1/+1 counter on it and sacrifices it unless you pay {1} for each +1/+1 counter on it.",
    ),
    faces = &[face!(
        name = "Phantasmal Sphere",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(0),
        toughness = Some(1),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[
            Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
            Effect::PlayerMayPayManaEachThenElse {
                player: PlayerRel::You,
                each: mana!("{1}"),
                times: Amount::CountersOnSource(CounterKind::P1P1),
                paid: &[],
                unpaid: &[Effect::SacrificeSelf],
            },
        ],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield
        ))),
    )],
);

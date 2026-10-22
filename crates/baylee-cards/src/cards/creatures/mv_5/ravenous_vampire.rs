//! Ravenous Vampire — {3}{B}{B} — Creature — Vampire
//! Oracle: Flying
//! Oracle: At the beginning of your upkeep, you may sacrifice a nonartifact creature. If you do, put a +1/+1 counter on this creature. If you don't, tap this creature.
//! Set: MIR #136 — Mirage | Scryfall ID: 5ee67033-7f0f-49b4-8472-3bc4fbb8ffe1 | Oracle ID: 2c029419-3601-4158-b10f-45a19de7faa3
// PARTIAL: The engine has no combined paid/unpaid branch for optional non-mana cost payment (PlayerMayPayCostThen has no unpaid branch); at the beginning of your upkeep you may sacrifice a nonartifact creature to put a +1/+1 counter on this creature, without tapping it if you decline

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const NONARTIFACT_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::ARTIFACT)]);

static ADD_P1P1: Effect = Effect::AddCounter {
    kind: CounterKind::P1P1,
    amount: Amount::Fixed(1),
};
static SACRIFICE_NONARTIFACT_CREATURE: CostPart = CostPart::Sacrifice(&NONARTIFACT_CREATURE);

card!(
    index = index::RAVENOUS_VAMPIRE,
    oracle_id = "2c029419-3601-4158-b10f-45a19de7faa3",
    scryfall_id = "5ee67033-7f0f-49b4-8472-3bc4fbb8ffe1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "The engine has no combined paid/unpaid branch for optional non-mana cost payment (PlayerMayPayCostThen has no unpaid branch); at the beginning of your upkeep you may sacrifice a nonartifact creature to put a +1/+1 counter on this creature, without tapping it if you decline"
    ),
    faces = &[face!(
        name = "Ravenous Vampire",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE],
        power = Some(3),
        toughness = Some(3),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::PlayerMayPayCostThen {
            player: PlayerRel::You,
            cost: &SACRIFICE_NONARTIFACT_CREATURE,
            effect: &ADD_P1P1,
        }],
    )],
);

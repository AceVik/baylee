//! Osai Vultures — {1}{W} — Creature — Bird
//! Oracle: Flying
//! Oracle: At the beginning of each end step, if a creature died this turn, put a carrion counter on this creature.
//! Oracle: Remove two carrion counters from this creature: This creature gets +1/+1 until end of turn.
//! Set: ME4 #21 — Masters Edition IV | Scryfall ID: 2c665c02-4010-4bc6-85f0-bc03bef38ce9 | Oracle ID: 65b7a7ca-0e39-4612-9bcd-743bf903544a
// PARTIAL — flying only; the carrion-counter trigger and ability are off the
// card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OSAI_VULTURES,
    oracle_id = "65b7a7ca-0e39-4612-9bcd-743bf903544a",
    scryfall_id = "2c665c02-4010-4bc6-85f0-bc03bef38ce9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "no counters::CARRION id is assigned in baylee_cards_dsl::counters, so \
         neither the counter the trigger places nor the two the ability \
         removes can be named, and no Condition says a creature died this turn"
    ),
    faces = &[face!(
        name = "Osai Vultures",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// NOT SUPPORTED: "At the beginning of each end step, if a creature died this
// turn, put a carrion counter on this creature." — the trigger
// (`Trigger::StepBegin { step: StepKind::End, whose: PlayerRel::EachPlayer }`)
// and the effect (`Effect::AddCounter`) are both sayable, but no
// `counters::CARRION` id is assigned and a card may not spell
// `CounterKind::Custom` as a number; the printed intervening `if` is missing
// too, because no `Condition` reads "a creature died this turn"
// (`Amount::CreaturesDiedThisTurn` is an amount, not a condition).
// NOT SUPPORTED: "Remove two carrion counters from this creature: This
// creature gets +1/+1 until end of turn." — the pump is sayable
// (`Effect::PumpFilter` on `Filter::This` for `Duration::UntilEndOfTurn`),
// but `CostPart::RemoveCounterSelf` has no `counters::CARRION` id to remove.

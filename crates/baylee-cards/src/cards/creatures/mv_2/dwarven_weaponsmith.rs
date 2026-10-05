//! Dwarven Weaponsmith — {1}{R} — Creature — Dwarf Artificer
//! Oracle: {T}, Sacrifice an artifact: Put a +1/+1 counter on target creature. Activate only during your upkeep.
//! Set: SUM #145 — Summer Magic / Edgar | Scryfall ID: aa9d6484-84ce-4dfe-b512-a9168b970ec1 | Oracle ID: a3541870-3dc9-4571-be98-c0a2b6c468fb
// IMPLEMENTED — {T}, sacrifice an artifact: +1/+1 counter on target creature, your upkeep only.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DWARVEN_WEAPONSMITH,
    oracle_id = "a3541870-3dc9-4571-be98-c0a2b6c468fb",
    scryfall_id = "aa9d6484-84ce-4dfe-b512-a9168b970ec1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dwarven Weaponsmith",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF, subtypes::creature::ARTIFICER],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    // "Activate only during your upkeep" is the same printed window as Rock
    // Hydra's third ability: `YourTurn` and `DuringStep(Upkeep)`.
    abilities = &[activated!(
        cost!(TapSelf, Sacrifice(&Filter::YOUR_ARTIFACT)),
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::Upkeep),
        ])),
    )],
);

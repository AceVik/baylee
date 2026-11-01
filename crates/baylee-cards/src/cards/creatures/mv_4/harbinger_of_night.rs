//! Harbinger of Night — {2}{B}{B} — Creature — Spirit
//! Oracle: At the beginning of your upkeep, put a -1/-1 counter on each creature.
//! Set: MIR #128 — Mirage | Scryfall ID: 33124133-ed2c-4b86-a135-ac76f4fe4da5 | Oracle ID: 9827d01d-c822-4f1b-9042-a2092c72c914
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HARBINGER_OF_NIGHT,
    oracle_id = "9827d01d-c822-4f1b-9042-a2092c72c914",
    scryfall_id = "33124133-ed2c-4b86-a135-ac76f4fe4da5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Harbinger of Night",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::AddCounterFilter {
            filter: &Filter::CREATURE,
            kind: CounterKind::M1M1,
            amount: Amount::Fixed(1),
        }],
    )],
);

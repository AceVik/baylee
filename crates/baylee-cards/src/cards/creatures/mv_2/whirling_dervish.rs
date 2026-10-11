//! Whirling Dervish — {G}{G} — Creature — Human Monk
//! Oracle: Protection from black
//! Oracle: At the beginning of each end step, if this creature dealt damage to an opponent this turn, put a +1/+1 counter on it.
//! Set: TSB #90 — Time Spiral Timeshifted | Scryfall ID: 9f1772bc-3330-4a10-9524-5bdcbd50c871 | Oracle ID: 43f89524-4b22-4707-9aba-6eb4c01d75dc

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHIRLING_DERVISH,
    oracle_id = "43f89524-4b22-4707-9aba-6eb4c01d75dc",
    scryfall_id = "9f1772bc-3330-4a10-9524-5bdcbd50c871",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Whirling Dervish",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::MONK],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Black])))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::EachPlayer
            },
            &[Effect::AddCounterSource {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1)
            }],
            condition = Some(Condition::SourceDamagedOpponentThisTurn)
        ),
    ],
);

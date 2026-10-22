//! Mangara's Blessing — {2}{W} — Instant
//! Oracle: You gain 5 life.
//! Oracle: When a spell or ability an opponent controls causes you to discard this card, you gain 2 life, and you return this card from your graveyard to your hand at the beginning of the next end step.
//! Set: MIR #25 — Mirage | Scryfall ID: b0125f79-ff68-4fb9-b309-8d277259f323 | Oracle ID: 5d9431de-7cf0-43d4-9b86-e0ef4a7756f9
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANGARA_S_BLESSING,
    oracle_id = "5d9431de-7cf0-43d4-9b86-e0ef4a7756f9",
    scryfall_id = "b0125f79-ff68-4fb9-b309-8d277259f323",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mangara's Blessing",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        spell!(&[Effect::GainLife {
            amount: Amount::Fixed(5),
        }]),
        triggered!(
            Trigger::DiscardedThisByOpponent,
            &[
                Effect::GainLife {
                    amount: Amount::Fixed(2),
                },
                Effect::AtNextEndStep {
                    effects: &[Effect::ReturnToHand {
                        target: TargetSpec::ThisObject,
                    }],
                },
            ],
        ),
    ],
);

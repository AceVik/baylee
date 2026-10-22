//! Psychic Vortex — {2}{U}{U} — Enchantment
//! Oracle: Cumulative upkeep—Draw a card. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of your end step, sacrifice a land and discard your hand.
//! Set: WTH #50 — Weatherlight | Scryfall ID: 3bc2a419-7122-4eeb-bb64-738a647cfd82 | Oracle ID: 451e35dd-e0b5-4157-8511-bacb315e19bd
// PARTIAL — Adds an age counter and lets you draw a card for each age counter on it at your upkeep, and sacrifices a land and discards your hand at your end step; sacrificing Psychic Vortex if you decline to draw for its cumulative upkeep is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PSYCHIC_VORTEX,
    oracle_id = "451e35dd-e0b5-4157-8511-bacb315e19bd",
    scryfall_id = "3bc2a419-7122-4eeb-bb64-738a647cfd82",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Adds an age counter and lets you draw a card for each age counter on it at your upkeep, and sacrifices a land and discards your hand at your end step; sacrificing Psychic Vortex if you decline to draw for its cumulative upkeep is not yet modeled.",
    ),
    faces = &[face!(
        name = "Psychic Vortex",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounterSource {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1),
                },
                Effect::MayDo {
                    effects: &[Effect::DrawCards {
                        amount: Amount::CountersOnSource(counters::AGE),
                    }],
                },
            ],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You,
            },
            &[
                Effect::SacrificeFilter {
                    who: PlayerRel::You,
                    filter: &Filter::LAND,
                },
                Effect::DiscardHand {
                    who: PlayerRel::You,
                },
            ],
        ),
    ],
);

//! Luminarch Ascension — {1}{W} — Enchantment
//! Oracle: At the beginning of each opponent's end step, if you didn't lose life this turn, you may put a quest counter on this enchantment. (Damage causes loss of life.)
//! Oracle: {1}{W}: Create a 4/4 white Angel creature token with flying. Activate only if this enchantment has four or more quest counters on it.
//! Set: A25 #23 — Masters 25 | Scryfall ID: b3770d86-4496-4c06-aab1-2917cfec100e | Oracle ID: 90076bf5-aa9a-4a6e-9035-9aa97fd5561e
// IMPLEMENTED — quest counters via end-step trigger + counter-gated angel activation (CountersOnSelf).
// condition evaluated from the journal). The angel-token ability needs
// activation gating by counters (M2+); currently always activatable.

use baylee_cards_dsl::prelude::*;

use crate::tokens::ANGEL_4_4_WHITE_FLYING as ANGEL;

card!(
    index = index::LUMINARCH_ASCENSION,
    oracle_id = "90076bf5-aa9a-4a6e-9035-9aa97fd5561e",
    scryfall_id = "b3770d86-4496-4c06-aab1-2917cfec100e",
    faces = &[face!(
        name = "Luminarch Ascension",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    )],
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::Opponent,
            },
            &[Effect::IfNotLostLifeThisTurn {
                // The "may" sits *inside* the intervening-if: a turn where
                // you did lose life asks nothing at all, because the
                // condition is checked on resolution and the ability simply
                // does nothing (CR 603.4).
                then: &[Effect::MayDo {
                    effects: &[Effect::AddCounter {
                        kind: counters::QUEST,
                        amount: Amount::Fixed(1),
                    }],
                }],
            }]
        ),
        activated!(
            cost!("{1}{W}"),
            &[Effect::CreateToken { token: &ANGEL }],
            condition = Some(Condition::CountersOnSelf(counters::QUEST, 4))
        ),
    ],
);

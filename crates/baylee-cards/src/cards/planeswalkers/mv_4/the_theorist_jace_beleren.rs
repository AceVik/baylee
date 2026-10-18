//! The Theorist, Jace Beleren — {2}{U}{U} — Legendary Planeswalker — Jace
//! Oracle: At the beginning of each opponent's draw step, you draw a card.
//! Oracle: +1: Create a 1/1 blue Illusion creature token.
//! Oracle: −2: For each opponent, return up to one target artifact or creature that player controls to its owner's hand.
//! Oracle: −6: Draw three cards. Then put X +1/+1 counters on each creature you control, where X is the number of cards in your hand.
//! Set: FRA #43 — Reality Fracture | Scryfall ID: 20bb8c55-4b0b-425f-8201-b54fa2fdde86 | Oracle ID: 89a6e876-0b00-4671-9652-766fd7ef9bf2
// IMPLEMENTED — opponent draw-step trigger, +1 Illusion token, −2 per-opponent bounce, −6 draw 3 + hand-size counters.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OPPONENT_ARTIFACT_OR_CREATURE: TargetSpec =
    TargetSpec::ObjectOfEachOpponent(&Filter::ARTIFACT_OR_CREATURE);

card!(
    index = index::THE_THEORIST_JACE_BELEREN,
    oracle_id = "89a6e876-0b00-4671-9652-766fd7ef9bf2",
    scryfall_id = "20bb8c55-4b0b-425f-8201-b54fa2fdde86",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "The Theorist, Jace Beleren",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::JACE],
        loyalty = Some(3),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Draw,
                whose: PlayerRel::Opponent,
            },
            &[Effect::draw(1)],
        ),
        loyalty!(
            1,
            &[Effect::CreateToken {
                token: &generated_tokens::ILLUSION_1_1_BLUE,
            }],
        ),
        loyalty!(
            -2,
            &[Effect::bounce(OPPONENT_ARTIFACT_OR_CREATURE)],
            targets = Some(TargetReq::up_to(OPPONENT_ARTIFACT_OR_CREATURE, u8::MAX)),
        ),
        loyalty!(
            -6,
            &[
                Effect::draw(3),
                Effect::AddCounterFilter {
                    filter: &Filter::YOUR_CREATURE,
                    kind: CounterKind::P1P1,
                    amount: Amount::HandSize(PlayerRel::You),
                },
            ],
        ),
    ],
);

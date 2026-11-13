//! Resplendent Angel — {1}{W}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: At the beginning of each end step, if you gained 5 or more life this turn, create a 4/4 white Angel creature token with flying and vigilance.
//! Oracle: {3}{W}{W}{W}: Until end of turn, this creature gets +2/+2 and gains lifelink.
//! Set: LCI #32 — The Lost Caverns of Ixalan | Scryfall ID: 59adbd35-9959-4a0f-babf-46ea3c15f018 | Oracle ID: 2ac93092-1a57-4cf7-8f03-1dca86d5c476
// IMPLEMENTED — life-gain token trigger and temporary pump/lifelink activation.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RESPLENDENT_ANGEL,
    oracle_id = "2ac93092-1a57-4cf7-8f03-1dca86d5c476",
    scryfall_id = "59adbd35-9959-4a0f-babf-46ea3c15f018",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Resplendent Angel",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::CreateToken {
                token: &generated_tokens::ANGEL_4_4_WHITE_FLYING_VIGILANCE,
            }],
            condition = Some(Condition::AmountAtLeast(
                Amount::LifeGainedThisTurn(PlayerRel::You),
                5,
            )),
        ),
        activated!(
            cost!("{3}{W}{W}{W}"),
            &[Effect::PumpFilter {
                filter: &Filter::This,
                controlled_by: None,
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(2),
                keywords: KeywordSet::LIFELINK,
                duration: Duration::UntilEndOfTurn,
            }]
        ),
    ],
);

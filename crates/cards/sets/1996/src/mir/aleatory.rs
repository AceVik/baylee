//! Aleatory — {1}{R} — Instant
//! Oracle: Cast this spell only during combat after blockers are declared.
//! Oracle: Flip a coin. If you win the flip, target creature gets +1/+1 until end of turn.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: MIR #155 — Mirage | Scryfall ID: 7380af0a-8a8f-44fd-9456-14a68a2830d3 | Oracle ID: da4cab2d-0068-440f-ba7c-f2d4e5fa976e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALEATORY,
    oracle_id = "da4cab2d-0068-440f-ba7c-f2d4e5fa976e",
    scryfall_id = "7380af0a-8a8f-44fd-9456-14a68a2830d3",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aleatory",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::FlipCoin {
                won: &[Effect::PumpTarget {
                    power: Amount::Fixed(1),
                    toughness: Amount::Fixed(1),
                    keywords: KeywordSet::EMPTY,
                    duration: Duration::UntilEndOfTurn,
                }],
                lost: &[],
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        condition = Some(Condition::All(&[
            Condition::DuringCombat,
            Condition::Not(&Condition::BeforeStep(StepKind::DeclareBlockers)),
        ])),
    )],
);

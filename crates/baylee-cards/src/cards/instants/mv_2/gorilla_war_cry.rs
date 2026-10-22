//! Gorilla War Cry — {1}{R} — Instant
//! Oracle: Cast this spell only during combat before blockers are declared.
//! Oracle: All creatures gain menace until end of turn. (They can't be blocked except by two or more creatures.)
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: ME4 #124 — Masters Edition IV | Scryfall ID: 7f738b12-8664-4f27-a006-c4574a71885a | Oracle ID: 60a7f61c-41ad-4dce-b556-a99bf38c3d73
use baylee_cards_dsl::prelude::*;

card!(
    index = index::GORILLA_WAR_CRY,
    oracle_id = "60a7f61c-41ad-4dce-b556-a99bf38c3d73",
    scryfall_id = "7f738b12-8664-4f27-a006-c4574a71885a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gorilla War Cry",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::PumpFilter {
                filter: &Filter::CREATURE,
                controlled_by: None,
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::MENACE,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        condition = Some(Condition::All(&[
            Condition::DuringCombat,
            Condition::BeforeStep(StepKind::DeclareBlockers),
        ])),
    )],
);

//! Ebony Charm — {B} — Instant
//! Oracle: Choose one —
//! Oracle: • Target opponent loses 1 life and you gain 1 life.
//! Oracle: • Exile up to three target cards from a single graveyard.
//! Oracle: • Target creature gains fear until end of turn. (It can't be blocked except by artifact creatures and/or black creatures.)
//! Set: MIR #120 — Mirage | Scryfall ID: 937c8016-43c8-418b-9057-b5d6715f71f7 | Oracle ID: 2f3237fe-9e07-4aad-8bc8-7ef4b510dd36
// IMPLEMENTED — modal charm with opponent drain, single-graveyard exile up to three, or granting fear until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EBONY_CHARM,
    oracle_id = "2f3237fe-9e07-4aad-8bc8-7ef4b510dd36",
    scryfall_id = "937c8016-43c8-418b-9057-b5d6715f71f7",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ebony Charm",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[
                    Effect::LoseLife {
                        amount: Amount::Fixed(1),
                        target: PlayerRel::Chosen,
                    },
                    Effect::GainLife {
                        amount: Amount::Fixed(1),
                    },
                ],
                targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
            ),
            mode!(
                &[Effect::exile(TargetSpec::CardInGraveyard(
                    &Filter::Any,
                    PlayerRel::Chosen,
                ))],
                targets = Some(TargetReq::up_to(
                    TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::Chosen),
                    3,
                ))
            ),
            mode!(
                &[Effect::PumpTarget {
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::FEAR,
                    duration: Duration::UntilEndOfTurn,
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
            ),
        ],
    }],
);

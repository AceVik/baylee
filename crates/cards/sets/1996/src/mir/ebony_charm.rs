//! Ebony Charm — {B} — Instant
//! Oracle: Choose one —
//! Oracle: • Target opponent loses 1 life and you gain 1 life.
//! Oracle: • Exile up to three target cards from a single graveyard.
//! Oracle: • Target creature gains fear until end of turn. (It can't be blocked except by artifact creatures and/or black creatures.)
//! Set: MIR #120 — Mirage | Scryfall ID: 937c8016-43c8-418b-9057-b5d6715f71f7 | Oracle ID: 2f3237fe-9e07-4aad-8bc8-7ef4b510dd36
// PARTIAL — mode 2 exiles at most one card instead of up to three: "from a single graveyard" is enforced for an activated ability only (start_activation), so three targets of a spell could be taken from two graveyards.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EBONY_CHARM,
    oracle_id = "2f3237fe-9e07-4aad-8bc8-7ef4b510dd36",
    scryfall_id = "937c8016-43c8-418b-9057-b5d6715f71f7",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "the exile mode takes at most one target card instead of up to three: 'from a single graveyard' is narrowed for an activated ability only, never for a spell, so up to three targets could come from two graveyards; the drain and fear modes are as printed",
    ),
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
            // "Up to three target cards from a single graveyard": one card is
            // always from a single graveyard, three might not be (see above).
            mode!(
                &[Effect::exile(TargetSpec::CardInGraveyard(
                    &Filter::Any,
                    PlayerRel::EachPlayer,
                ))],
                targets = Some(TargetReq::up_to(
                    TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer),
                    1,
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

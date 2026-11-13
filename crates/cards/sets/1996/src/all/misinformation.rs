//! Misinformation — {B} — Instant
//! Oracle: Put up to three target cards from an opponent's graveyard on top of their library in any order.
//! Set: ME2 #105 — Masters Edition II | Scryfall ID: 90a79cd5-d951-4ad7-a6a1-0cade4d0c189 | Oracle ID: 03dc8820-ec10-42c1-8200-d323cfcbf2b0
// PARTIAL — puts the first of the up-to-three targeted cards from an opponent's graveyard on top of its owner's library; the other two stay.
// NOT SUPPORTED: putting up to three target cards from an opponent's graveyard on top of their library in any order requires multi-target graveyard-to-top ordering in Effect::GraveyardToTop.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MISINFORMATION,
    oracle_id = "03dc8820-ec10-42c1-8200-d323cfcbf2b0",
    scryfall_id = "90a79cd5-d951-4ad7-a6a1-0cade4d0c189",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Putting up to three target cards from an opponent's graveyard on top of their library in any order requires multi-target graveyard-to-top ordering in Effect::GraveyardToTop; in play, Misinformation puts the first targeted card from an opponent's graveyard on top of its owner's library.",
    ),
    faces = &[face!(
        name = "Misinformation",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::GraveyardToTop {
            target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::Opponent),
        }],
        targets = Some(TargetReq::up_to(
            TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::Opponent),
            3,
        )),
    )],
);

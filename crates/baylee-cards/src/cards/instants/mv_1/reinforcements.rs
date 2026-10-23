//! Reinforcements — {W} — Instant
//! Oracle: Put up to three target creature cards from your graveyard on top of your library.
//! Set: ME2 #28 — Masters Edition II | Scryfall ID: 36f250f5-e667-458d-aed7-3ead3f3db64b | Oracle ID: ff27ff96-afd2-45df-b799-3046179e27c6
// PARTIAL — puts the first of the up-to-three targeted creature cards from your graveyard on top of your library; the other two stay.
// NOT SUPPORTED: putting up to three target creature cards from your graveyard on top of your library in any order requires multi-target graveyard-to-top ordering in Effect::GraveyardToTop.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REINFORCEMENTS,
    oracle_id = "ff27ff96-afd2-45df-b799-3046179e27c6",
    scryfall_id = "36f250f5-e667-458d-aed7-3ead3f3db64b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Putting up to three target creature cards from your graveyard on top of your library in any order requires multi-target graveyard-to-top ordering in Effect::GraveyardToTop; in play, Reinforcements puts the first targeted creature card from your graveyard on top of your library.",
    ),
    faces = &[face!(
        name = "Reinforcements",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::GraveyardToTop {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
        }],
        targets = Some(TargetReq::up_to(
            TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            3,
        )),
    )],
);

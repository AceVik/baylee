//! Gaea's Blessing — {1}{G} — Sorcery
//! Oracle: Target player shuffles up to three target cards from their graveyard into their library.
//! Oracle: Draw a card.
//! Oracle: When this card is put into your graveyard from your library, shuffle your graveyard into your library.
//! Set: DOM #161 — Dominaria | Scryfall ID: 23cf81ed-b86c-42b8-b796-2032b0a3654a | Oracle ID: bd5478df-3854-404b-8634-9debcef69b99
// NOT SUPPORTED: triggering when Gaea's Blessing is put into your graveyard from your library is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GAEA_S_BLESSING,
    oracle_id = "bd5478df-3854-404b-8634-9debcef69b99",
    scryfall_id = "23cf81ed-b86c-42b8-b796-2032b0a3654a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "puts up to three target cards from target player's graveyard into their library, shuffles that player's library, and draws a card; the triggered ability when Gaea's Blessing is put into your graveyard from your library is not yet modeled",
    ),
    faces = &[face!(
        name = "Gaea's Blessing",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::TargetedGraveyardToTop,
            Effect::ShuffleLibrary {
                who: PlayerRel::TargetPlayerOrController,
            },
            Effect::draw(1),
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
        second_targets = Some(TargetReq::up_to(
            TargetSpec::CardInFirstTargetsGraveyard(&Filter::Any),
            3,
        )),
    )],
);

//! Intuition — {2}{U} — Instant
//! Oracle: Search your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle.
//! Set: TPR #54 — Tempest Remastered | Scryfall ID: b13e73a5-067d-4dbd-9c98-34a0db6140de | Oracle ID: 3c9faba7-f2d3-4978-be94-020dc8003dc0
// PARTIAL — SearchOpponentSplits requires the searched cards to have different names and asks an opponent to choose two for the graveyard rather than using the targeted opponent to choose one for hand; searches the library for up to three cards with different names, has an opponent put two into the graveyard and the rest into hand, then shuffles.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INTUITION,
    oracle_id = "3c9faba7-f2d3-4978-be94-020dc8003dc0",
    scryfall_id = "b13e73a5-067d-4dbd-9c98-34a0db6140de",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "SearchOpponentSplits requires the searched cards to have different names and asks an opponent to choose two for the graveyard rather than using the targeted opponent to choose one for hand; searches the library for up to three cards with different names, has an opponent put two into the graveyard and the rest into hand, then shuffles."
    ),
    faces = &[face!(
        name = "Intuition",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::SearchOpponentSplits {
            filter: &Filter::Any,
            up_to: 3,
            chosen: 2,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);

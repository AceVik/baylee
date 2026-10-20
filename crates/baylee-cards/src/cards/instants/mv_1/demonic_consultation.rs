//! Demonic Consultation — {B} — Instant
//! Oracle: Choose a card name. Exile the top six cards of your library, then reveal cards from the top of your library until you reveal a card with the chosen name. Put that card into your hand and exile all other cards revealed this way.
//! Set: ME2 #85 — Masters Edition II | Scryfall ID: 1d779f19-3068-4976-b96b-8f93d156900b | Oracle ID: 9a1412db-45ad-46ea-8f12-a85d203113d8
// PARTIAL: exiling top six cards and revealing until the named card is found not supported; checks top card.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Exile the top six cards of your library, then reveal cards from the top of your library until you reveal a card with the chosen name. Put that card into your hand and exile all other cards revealed this way.
card!(
    index = index::DEMONIC_CONSULTATION,
    oracle_id = "9a1412db-45ad-46ea-8f12-a85d203113d8",
    scryfall_id = "1d779f19-3068-4976-b96b-8f93d156900b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "exiling top six cards and revealing until the named card is found not supported; checks top card"
    ),
    faces = &[face!(
        name = "Demonic Consultation",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[
        Effect::ChooseCardName {
            player: PlayerRel::You,
            types: TypeSet::EMPTY,
            remember: false,
        },
        Effect::RevealTopByName {
            player: PlayerRel::You,
            matched: SearchDest::Hand,
            otherwise: SearchDest::Graveyard,
        },
    ])],
);

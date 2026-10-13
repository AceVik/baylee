//! Recall — {X}{X}{U} — Sorcery
//! Oracle: Discard X cards, then return a card from your graveyard to your hand for each card discarded this way. Exile Recall.
//! Set: ME3 #46 — Masters Edition III | Scryfall ID: dd82d1f8-12bb-4b71-9a25-76a9dcb68345 | Oracle ID: 4158f397-4ee4-4f53-90f2-3b64bfe4f9b8
// IMPLEMENTED — discards X cards of your choice, returns a card from your
// graveyard to your hand for each card discarded this way, then exiles Recall.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RECALL,
    oracle_id = "4158f397-4ee4-4f53-90f2-3b64bfe4f9b8",
    scryfall_id = "dd82d1f8-12bb-4b71-9a25-76a9dcb68345",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Recall",
        mana_cost = mana!("{X}{X}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        // "Discard X cards" — the announced X, of the player's choice.
        Effect::DiscardCount {
            who: PlayerRel::You,
            count: Amount::X,
        },
        // "then return a card from your graveyard to your hand for each card
        // discarded this way."
        Effect::Repeat {
            times: Amount::ThisWay(ThisWay::Discarded),
            effects: &[Effect::ChooseFromYourGraveyard {
                filter: &Filter::Any,
                destination: SearchDest::Hand,
                tapped: false,
                optional: false,
            }],
        },
        Effect::ExileSource,
    ])],
);

//! Aerial Caravan — {4}{U}{U} — Creature — Human Soldier
//! Oracle: Flying
//! Oracle: {1}{U}{U}: Exile the top card of your library. Until end of turn, you may play that card. (Reveal the card as you exile it.)
//! Set: MMQ #58 — Mercadian Masques | Scryfall ID: adac91af-5165-4779-99f7-e75c83fa5d5d | Oracle ID: d13aff7c-f0ee-44e2-ae1c-62cd86ad46a6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AERIAL_CARAVAN,
    oracle_id = "d13aff7c-f0ee-44e2-ae1c-62cd86ad46a6",
    scryfall_id = "adac91af-5165-4779-99f7-e75c83fa5d5d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aerial Caravan",
        mana_cost = mana!("{4}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(4),
        toughness = Some(3),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[activated!(
        cost!("{1}{U}{U}"),
        &[Effect::ExileTopToPlay {
            who: PlayerRel::You,
            play: ExilePlay {
                count: 1,
                window: PlayWindow::ThisTurn,
                free: false,
                mana_any_type: false,
                spell_types: None,
            },
        }],
    )],
);

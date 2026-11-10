//! Petra Sphinx — {2}{W}{W}{W} — Creature — Sphinx
//! Oracle: {T}: Target player chooses a card name, then reveals the top card of their library. If that card has the chosen name, that player puts it into their hand. If it doesn't, the player puts it into their graveyard.
//! Set: ME1 #23 — Masters Edition | Scryfall ID: e799b225-7bd7-40dd-b555-37ea500e12fe | Oracle ID: f1ae89ff-b23b-44b2-a30c-6f2ae3216189
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PETRA_SPHINX,
    oracle_id = "f1ae89ff-b23b-44b2-a30c-6f2ae3216189",
    scryfall_id = "e799b225-7bd7-40dd-b555-37ea500e12fe",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Petra Sphinx",
        mana_cost = mana!("{2}{W}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPHINX],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!(TapSelf),
        &[
            Effect::ChooseCardName {
                player: PlayerRel::Chosen,
                types: TypeSet::EMPTY,
                excluded_types: TypeSet::EMPTY,
                remember: false
            },
            Effect::RevealTopByName {
                player: PlayerRel::Chosen,
                matched: SearchDest::Hand,
                otherwise: SearchDest::Graveyard
            },
        ],
        target = Some(TargetSpec::AnyPlayer)
    )],
);

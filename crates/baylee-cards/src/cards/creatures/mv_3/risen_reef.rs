//! Risen Reef — {1}{G}{U} — Creature — Elemental
//! Oracle: Whenever this creature or another Elemental you control enters, look at the top card of your library. If it's a land card, you may put it onto the battlefield tapped. If you don't put the card onto the battlefield, put it into your hand.
//! Set: ECC #132 — Lorwyn Eclipsed Commander | Scryfall ID: 715ad2ff-7eae-42f1-bb3c-a3afc2c9b82a | Oracle ID: 2ae71e86-4400-4a30-9077-4d57a43e7395
// IMPLEMENTED — this or another Elemental entering looks at the top card; a
// land may go onto the battlefield tapped, anything else goes to hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RISEN_REEF,
    oracle_id = "2ae71e86-4400-4a30-9077-4d57a43e7395",
    scryfall_id = "715ad2ff-7eae-42f1-bb3c-a3afc2c9b82a",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Risen Reef",
        mana_cost = mana!("{1}{G}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        // "Whenever this creature or another Elemental you control enters":
        // `This` keeps the Reef in the sentence if it ever loses the type.
        Trigger::EntersBattlefield(&Filter::Or(&[
            Filter::This,
            Filter::And(&[
                Filter::HasSubtype(subtypes::creature::ELEMENTAL),
                Filter::ControlledByYou,
            ]),
        ])),
        // "Look at the top card of your library. If it's a land card, you
        // may put it onto the battlefield tapped. If you don't put the card
        // onto the battlefield, put it into your hand."
        &[Effect::LookAtTopMayPut {
            filter: &Filter::LAND,
            matched: Find::BATTLEFIELD_TAPPED,
            otherwise: SearchDest::Hand,
        }],
    )],
);

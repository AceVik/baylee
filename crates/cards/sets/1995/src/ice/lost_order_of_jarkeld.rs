//! Lost Order of Jarkeld — {2}{W}{W} — Creature — Human Knight
//! Oracle: As this creature enters, choose an opponent.
//! Oracle: Lost Order of Jarkeld's power and toughness are each equal to 1 plus the number of creatures the chosen player controls.
//! Set: ME2 #24 — Masters Edition II | Scryfall ID: a2294041-2837-4313-95ae-3a1222770a8e | Oracle ID: 75f047dc-4a51-494d-8765-fa7502d1fd06

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LOST_ORDER_OF_JARKELD,
    oracle_id = "75f047dc-4a51-494d-8765-fa7502d1fd06",
    scryfall_id = "a2294041-2837-4313-95ae-3a1222770a8e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lost Order of Jarkeld",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(1),
        toughness = Some(1),
        enter_modifiers = &[EnterModifier::ChooseOpponent],
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPT {
            count: PtCount::Plus {
                base: &PtCount::ChosenPlayerControls(&Filter::CREATURE),
                offset: 1,
            },
            toughness_plus: 0,
        }
    )],
);

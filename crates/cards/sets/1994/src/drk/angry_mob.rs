//! Angry Mob — {2}{W}{W} — Creature — Human
//! Oracle: Trample
//! Oracle: During your turn, Angry Mob's power and toughness are each equal to 2 plus the number of Swamps your opponents control. During turns other than yours, Angry Mob's power and toughness are each 2.
//! Set: ME1 #1 — Masters Edition | Scryfall ID: 2dcaea36-52fb-4c71-a4f1-5c060b918ff1 | Oracle ID: 967c2c79-7e49-448d-8d30-b2eb08b21d0a

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const OPPONENT_SWAMP: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::HasSubtype(subtypes::land::SWAMP),
]);

card!(
    index = index::ANGRY_MOB,
    oracle_id = "967c2c79-7e49-448d-8d30-b2eb08b21d0a",
    scryfall_id = "2dcaea36-52fb-4c71-a4f1-5c060b918ff1",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Angry Mob",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::TRAMPLE,
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPT {
            count: PtCount::Plus {
                base: &PtCount::OnBattlefield(&OPPONENT_SWAMP),
                offset: 2,
            },
            toughness_plus: 0,
        },
        condition = Some(Condition::YourTurn),
    )],
);

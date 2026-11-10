//! Harbor Guardian — {2}{W}{U} — Creature — Gargoyle
//! Oracle: Reach (This creature can block creatures with flying.)
//! Oracle: Whenever this creature attacks, defending player may draw a card.
//! Set: MIR #266 — Mirage | Scryfall ID: 78a2359f-6586-42a6-a855-c0049b448cb9 | Oracle ID: d72b69aa-fb61-441e-9b62-4534d7a32a69
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HARBOR_GUARDIAN,
    oracle_id = "d72b69aa-fb61-441e-9b62-4534d7a32a69",
    scryfall_id = "78a2359f-6586-42a6-a855-c0049b448cb9",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Harbor Guardian",
        mana_cost = mana!("{2}{W}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GARGOYLE],
        power = Some(3),
        toughness = Some(4),
        keywords = KeywordSet::REACH,
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::MayDoFor {
            player: PlayerRel::DefendingPlayer,
            effects: &[Effect::DrawCardsFor {
                amount: Amount::Fixed(1),
                who: PlayerRel::DefendingPlayer,
            }],
        }],
    )],
);

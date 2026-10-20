//! Chub Toad — {2}{G} — Creature — Frog
//! Oracle: Whenever this creature blocks or becomes blocked, it gets +2/+2 until end of turn.
//! Set: ME1 #116 — Masters Edition | Scryfall ID: 4fb0c8a5-ea67-4c18-94b6-757e290ee34e | Oracle ID: a0daace2-92fc-4c7d-b497-94b4ba775559
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHUB_TOAD,
    oracle_id = "a0daace2-92fc-4c7d-b497-94b4ba775559",
    scryfall_id = "4fb0c8a5-ea67-4c18-94b6-757e290ee34e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chub Toad",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FROG],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(2, 2),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);

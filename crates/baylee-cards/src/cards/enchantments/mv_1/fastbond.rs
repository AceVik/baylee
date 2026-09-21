//! Fastbond — {G} — Enchantment
//! Oracle: You may play any number of lands on each of your turns.
//! Oracle: Whenever you play a land, if it wasn't the first land you played this turn, this enchantment deals 1 damage to you.
//! Set: VMA #209 — Vintage Masters | Scryfall ID: daf43523-558c-4701-9fa3-5d1ceb82a006 | Oracle ID: e27193b7-1a47-4555-865d-b1fd4c6d597f
// PARTIAL — the unlimited land drops are granted as ExtraLandDrops(u8::MAX), the
// DSL's "any number" for a u8 field; the landfall damage clause has no trigger
// event and no intervening-if it could be written with.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FASTBOND,
    oracle_id = "e27193b7-1a47-4555-865d-b1fd4c6d597f",
    scryfall_id = "daf43523-558c-4701-9fa3-5d1ceb82a006",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Fastbond",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Partial(
        "no Trigger event for \"you play a land\" (Trigger::EntersBattlefield also fires on a land put onto the battlefield) and no Condition counts the lands played this turn"
    ),
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::ExtraLandDrops(u8::MAX)
    )],
);

// NOT SUPPORTED: "Whenever you play a land, if it wasn't the first land you
// played this turn, this enchantment deals 1 damage to you." — the DSL has no
// trigger for a land being *played* (an EntersBattlefield trigger would also
// fire on a land an effect put onto the battlefield, which is a different
// card), and no Condition can count the lands its controller has played this
// turn.

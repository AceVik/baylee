//! Fastbond — {G} — Enchantment
//! Oracle: You may play any number of lands on each of your turns.
//! Oracle: Whenever you play a land, if it wasn't the first land you played this turn, this enchantment deals 1 damage to you.
//! Set: VMA #209 — Vintage Masters | Scryfall ID: daf43523-558c-4701-9fa3-5d1ceb82a006 | Oracle ID: e27193b7-1a47-4555-865d-b1fd4c6d597f
// IMPLEMENTED — "any number of lands" is ExtraLandDrops(u8::MAX): the sum
// saturates (`casting::land_drops_allowed`), so it is 256 land drops a turn,
// which no game reaches. Every land played after the first — out of the hand
// or from wherever a permission allows — deals 1 damage to its controller;
// a land an effect puts onto the battlefield is not played and does not.

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
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::Any, Modifier::ExtraLandDrops(u8::MAX)),
        // "Whenever you play a land, if it wasn't the first land you played
        // this turn, this enchantment deals 1 damage to you." The `if` is an
        // intervening one (CR 603.4): the first land triggers nothing.
        triggered!(
            Trigger::PlaysLand(PlayerRel::You),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            }],
            condition = Some(Condition::LandsPlayedThisTurnAtLeast(2))
        ),
    ],
);

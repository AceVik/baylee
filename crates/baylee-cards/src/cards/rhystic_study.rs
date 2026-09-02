//! Rhystic Study — {2}{U} — Enchantment
//! Oracle: Whenever an opponent casts a spell, you may have that player pay {1}. If they don't, you draw a card.
//! Set: J25 #587 — Foundations Jumpstart | Scryfall ID: 9f37c5b6-a59c-45cd-9a99-e9357fe9ea1b | Oracle ID: 53236dd7-845a-444c-96d5-f41ed7325d8f
// IMPLEMENTED — opponent-choice {1} tax on opponents' spells.

static DRAW_ONE: Effect = Effect::DrawCards {
    amount: Amount::Fixed(1),
};

use baylee_cards_dsl::prelude::*;

card! {
    index: 133,
    oracle_id: "53236dd7-845a-444c-96d5-f41ed7325d8f",
    scryfall_id: "9f37c5b6-a59c-45cd-9a99-e9357fe9ea1b",
    faces: &[face! {
        name: "Rhystic Study",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::ENCHANTMENT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::SpellCast(&Filter::ControlledByOpponent), &[Effect::PlayerMayPayOr {
            player: PlayerRel::Opponent,
            mana: 1,
            effect: &DRAW_ONE,
        }])],
}

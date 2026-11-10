//! Rhystic Study — {2}{U} — Enchantment
//! Oracle: Whenever an opponent casts a spell, you may draw a card unless that player pays {1}.
//! Set: J22 #114 — Jumpstart 2022 | Scryfall ID: 9f37c5b6-a59c-45cd-9a99-e9357fe9ea1b | Oracle ID: 53236dd7-845a-444c-96d5-f41ed7325d8f
// IMPLEMENTED — the {1} tax is asked of the opponent who cast the spell, and
// only them; unpaid, this enchantment's controller may draw.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RHYSTIC_STUDY,
    oracle_id = "53236dd7-845a-444c-96d5-f41ed7325d8f",
    scryfall_id = "9f37c5b6-a59c-45cd-9a99-e9357fe9ea1b",
    faces = &[face!(
        name = "Rhystic Study",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    )],
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::ControlledByOpponent),
        // "That player": the one who cast the spell, not each opponent.
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::EventPlayer,
            mana: Amount::Fixed(1),
            effect: &Effect::MayDo {
                effects: &[Effect::draw(1)]
            },
        }]
    )],
);

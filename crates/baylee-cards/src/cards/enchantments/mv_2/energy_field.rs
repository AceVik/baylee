//! Energy Field — {1}{U} — Enchantment
//! Oracle: Prevent all damage that would be dealt to you by sources you don't control.
//! Oracle: When a card is put into your graveyard from anywhere, sacrifice this enchantment.
//! Set: USG #73 — Urza's Saga | Scryfall ID: 81ff5770-b207-41e1-97b7-b9347c72b407 | Oracle ID: fbf7a7f4-9915-4a53-84b3-e55e9f487323
// PARTIAL

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENERGY_FIELD,
    oracle_id = "fbf7a7f4-9915-4a53-84b3-e55e9f487323",
    scryfall_id = "81ff5770-b207-41e1-97b7-b9347c72b407",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "triggering when a card is put into your graveyard from non-battlefield/non-hand zones (such as library or stack) is not in Trigger; prevents all damage to you from sources you don't control and sacrifices this enchantment when a permanent you control dies or when you discard a card",
    ),
    faces = &[face!(
        name = "Energy Field",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::PreventDamageToYouFrom(&Filter::ControlledByOpponent)
        ),
        triggered!(
            Trigger::Dies(&Filter::ControlledByYou),
            &[Effect::SacrificeSelf],
        ),
        triggered!(
            Trigger::Discarded {
                who: PlayerRel::You,
                filter: &Filter::Any,
            },
            &[Effect::SacrificeSelf],
        ),
    ],
);

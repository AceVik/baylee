//! Energy Field — {1}{U} — Enchantment
//! Oracle: Prevent all damage that would be dealt to you by sources you don't control.
//! Oracle: When a card is put into your graveyard from anywhere, sacrifice this enchantment.
//! Set: USG #73 — Urza's Saga | Scryfall ID: 81ff5770-b207-41e1-97b7-b9347c72b407 | Oracle ID: fbf7a7f4-9915-4a53-84b3-e55e9f487323
// IMPLEMENTED — prevents damage to you from sources you don't control and sacrifices itself when a non-token card enters your graveyard.

use baylee_cards_dsl::prelude::*;

static FOREIGN_SOURCE: Filter = Filter::Not(&Filter::ControlledByYou);
static YOUR_NONTOKEN_CARD: Filter =
    Filter::And(&[Filter::OwnedByYou, Filter::Not(&Filter::IsToken)]);

card!(
    index = index::ENERGY_FIELD,
    oracle_id = "fbf7a7f4-9915-4a53-84b3-e55e9f487323",
    scryfall_id = "81ff5770-b207-41e1-97b7-b9347c72b407",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Energy Field",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::PreventDamageToYouFrom(&FOREIGN_SOURCE),
        ),
        triggered!(
            Trigger::ChangesZoneAll {
                origin: None,
                destination: Some(ZoneRef::Graveyard),
                filter: &YOUR_NONTOKEN_CARD,
                exclude_self: false,
            },
            &[Effect::SacrificeSelf],
        ),
    ],
);

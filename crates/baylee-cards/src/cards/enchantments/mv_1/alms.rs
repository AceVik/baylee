//! Alms — {W} — Enchantment
//! Oracle: {1}, Exile the top card of your graveyard: Prevent the next 1 damage that would be dealt to target creature this turn.
//! Set: WTH #3 — Weatherlight | Scryfall ID: 97382dd8-2754-4ca3-8ba8-d655acaf22ac | Oracle ID: efd46a6e-a901-4224-8a29-d53a4dc7750c
// NOT SUPPORTED: requiring the exiled graveyard card to be the top card of your graveyard is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALMS,
    oracle_id = "efd46a6e-a901-4224-8a29-d53a4dc7750c",
    scryfall_id = "97382dd8-2754-4ca3-8ba8-d655acaf22ac",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "activates for {1} and exiling a card from your graveyard to prevent the next 1 damage to target creature this turn; requiring the exiled card to be the top card of your graveyard is not yet modeled",
    ),
    faces = &[face!(
        name = "Alms",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{1}", ExileFromGraveyard(&Filter::Any)),
        &[Effect::PreventNextDamage {
            target: TargetSpec::Object(&Filter::CREATURE),
            amount: Amount::Fixed(1),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);

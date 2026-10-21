//! Oath of Lim-Dûl — {3}{B} — Enchantment
//! Oracle: Whenever you lose life, for each 1 life you lost, sacrifice a permanent other than this enchantment unless you discard a card. (Damage dealt to you causes you to lose life.)
//! Oracle: {B}{B}: Draw a card.
//! Set: ICE #156 — Ice Age | Scryfall ID: f16df768-06de-43a0-b548-44fb0887490b | Oracle ID: 64e163c2-2378-4e4c-aefe-5e0d2fd19988
use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: No Trigger::LosesLife exists to trigger when you lose life and repeat a sacrifice-or-discard choice per point of life lost; only the {B}{B}: Draw a card ability is implemented, with no drawback when you lose life.
card!(
    index = index::OATH_OF_LIM_DUL,
    oracle_id = "64e163c2-2378-4e4c-aefe-5e0d2fd19988",
    scryfall_id = "f16df768-06de-43a0-b548-44fb0887490b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "no life-loss trigger exists for the sacrifice-or-discard drawback; only the {B}{B} card-draw ability works",
    ),
    faces = &[face!(
        name = "Oath of Lim-Dûl",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(cost!("{B}{B}"), &[Effect::draw(1)])],
);

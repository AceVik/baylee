//! Defense of the Heart — {3}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, if an opponent controls three or more creatures, sacrifice this enchantment, search your library for up to two creature cards, put those cards onto the battlefield, then shuffle.
//! Set: ULG #100 — Urza's Legacy | Scryfall ID: 9e93381f-627f-41b6-b1b6-45d712a44d8e | Oracle ID: e7e1b166-9267-426d-897d-24903327b48d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEFENSE_OF_THE_HEART,
    oracle_id = "e7e1b166-9267-426d-897d-24903327b48d",
    scryfall_id = "9e93381f-627f-41b6-b1b6-45d712a44d8e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Defense of the Heart",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[
            Effect::SacrificeSelf,
            Effect::SearchLibrary {
                filter: &Filter::CREATURE,
                finds: &[Find::BATTLEFIELD, Find::BATTLEFIELD],
                optional: true,
            },
        ],
        condition = Some(Condition::OpponentControlCount(
            &Filter::OPPONENT_CREATURE,
            3,
        )),
    )],
);

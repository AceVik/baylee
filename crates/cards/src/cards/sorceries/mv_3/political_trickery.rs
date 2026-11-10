//! Political Trickery — {2}{U} — Sorcery
//! Oracle: Exchange control of target land you control and target land an opponent controls. (This effect lasts indefinitely.)
//! Set: MIR #81 — Mirage | Scryfall ID: 3e56e720-5e8b-4685-969c-e073de78b9a1 | Oracle ID: e0eaa43f-1c46-4299-9ecd-5f4a32d292cd
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const YOUR_LAND: Filter = Filter::And(&[Filter::LAND, Filter::ControlledByYou]);
const OPPONENT_LAND: Filter = Filter::And(&[Filter::LAND, Filter::ControlledByOpponent]);

card!(
    index = index::POLITICAL_TRICKERY,
    oracle_id = "e0eaa43f-1c46-4299-9ecd-5f4a32d292cd",
    scryfall_id = "3e56e720-5e8b-4685-969c-e073de78b9a1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Political Trickery",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::ExchangeControl],
        targets = Some(TargetReq::one(TargetSpec::Object(&YOUR_LAND))),
        second_targets = Some(TargetReq::one(TargetSpec::Object(&OPPONENT_LAND))),
    )],
);

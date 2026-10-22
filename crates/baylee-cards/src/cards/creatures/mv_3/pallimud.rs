//! Pallimud — {2}{R} — Creature — Beast
//! Oracle: As this creature enters, choose an opponent.
//! Oracle: Pallimud's power is equal to the number of tapped lands the chosen player controls.
//! Set: TMP #195 — Tempest | Scryfall ID: 61adc314-cfb2-4fdd-925c-cc1dc4692992 | Oracle ID: cad0f865-62e0-47ba-84ae-8e97cb52bbd1
// PARTIAL: chooses an opponent as it enters and defines its power as the number of tapped lands opponents control; scoping the characteristic-defining count to the single chosen opponent in multiplayer is not in PtCount.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static TAPPED_OPPONENT_LAND: Filter =
    Filter::And(&[Filter::LAND, Filter::Tapped, Filter::ControlledByOpponent]);

card!(
    index = index::PALLIMUD,
    oracle_id = "cad0f865-62e0-47ba-84ae-8e97cb52bbd1",
    scryfall_id = "61adc314-cfb2-4fdd-925c-cc1dc4692992",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "chooses an opponent as it enters and defines its power as the number of tapped lands opponents control; scoping the characteristic-defining count to the single chosen opponent in multiplayer is not in PtCount",
    ),
    faces = &[face!(
        name = "Pallimud",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(0),
        toughness = Some(3),
        enter_modifiers = &[EnterModifier::ChooseOpponent],
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPTSeparate {
            power: PtCount::OnBattlefield(&TAPPED_OPPONENT_LAND),
            toughness: PtCount::Fixed(3),
        },
    )],
);

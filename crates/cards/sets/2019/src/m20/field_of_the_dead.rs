//! Field of the Dead — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: Whenever this land or another land you control enters, if you control seven or more lands with different names, create a 2/2 black Zombie creature token.
//! Set: M20 #247 — Core Set 2020 | Scryfall ID: 470ca3f4-29aa-4c4c-8ff2-8cdd70c69943 | Oracle ID: aa959340-c869-4caa-92c7-572bd8d23eef
// IMPLEMENTED — enters tapped, taps for {C}, and a land entering under your
// control makes a Zombie while seven or more of your lands have different
// names (`Condition::ControlDistinctNames`, asked as an intervening-if).

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIELD_OF_THE_DEAD,
    oracle_id = "aa959340-c869-4caa-92c7-572bd8d23eef",
    scryfall_id = "470ca3f4-29aa-4c4c-8ff2-8cdd70c69943",
    faces = &[face!(
        name = "Field of the Dead",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // "This land or another land you control" is every land you control,
        // the source included: `Trigger::ETB` is this same trigger over
        // `Filter::This`.
        triggered!(
            Trigger::EntersBattlefield(&Filter::YOUR_LAND),
            &[Effect::CreateToken {
                token: &generated_tokens::ZOMBIE_2_2_BLACK,
            }],
            condition = Some(Condition::ControlDistinctNames(&Filter::LAND, 7)),
        ),
    ],
);

//! Thrull Wizard — {2}{B} — Creature — Thrull Wizard
//! Oracle: {1}{B}: Counter target black spell unless that spell's controller pays {B} or {3}.
//! Set: FEM #46 — Fallen Empires | Scryfall ID: c4e732fb-cbef-4fd8-b704-e4d513a6cf2d | Oracle ID: 3cae0e4b-1827-4ed2-832f-98f8e79941a5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLACK_SPELL: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Black]));

card!(
    index = index::THRULL_WIZARD,
    oracle_id = "3cae0e4b-1827-4ed2-832f-98f8e79941a5",
    scryfall_id = "c4e732fb-cbef-4fd8-b704-e4d513a6cf2d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thrull Wizard",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::THRULL, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{B}"),
        &[Effect::PlayerMayPayManaOr {
            player: PlayerRel::ControllerOfTarget,
            cost: mana!("{B}"),
            effect: &Effect::PlayerMayPayOr {
                player: PlayerRel::ControllerOfTarget,
                mana: Amount::Fixed(3),
                effect: &Effect::CounterTargetSpell,
            },
        }],
        target = Some(TargetSpec::Spell(&BLACK_SPELL)),
    ),],
);

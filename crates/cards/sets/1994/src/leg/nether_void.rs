//! Nether Void — {3}{B} — World Enchantment
//! Oracle: Whenever a player casts a spell, counter it unless that player pays {3}.
//! Set: ME3 #73 — Masters Edition III | Scryfall ID: a3452173-01e1-4a4a-981e-e6b6dea61c61 | Oracle ID: 53bab610-1d27-4897-a4ad-cfecca82b811
// IMPLEMENTED — whenever a player casts a spell, that player may pay {3}; if they don't, the spell is countered.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NETHER_VOID,
    oracle_id = "53bab610-1d27-4897-a4ad-cfecca82b811",
    scryfall_id = "a3452173-01e1-4a4a-981e-e6b6dea61c61",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Nether Void",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::Any),
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfEvent,
            mana: Amount::Fixed(3),
            effect: &Effect::CounterTargetSpell
        }],
        targets = Some(TargetReq::one(TargetSpec::EventObject))
    ),],
);

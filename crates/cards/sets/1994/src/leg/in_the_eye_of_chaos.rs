//! In the Eye of Chaos — {2}{U} — World Enchantment
//! Oracle: Whenever a player casts an instant spell, counter it unless that player pays {X}, where X is its mana value.
//! Set: ME4 #51 — Masters Edition IV | Scryfall ID: 6964fc80-7f39-4ffd-80fd-17d9a75e8e84 | Oracle ID: 6a0da9f3-cb06-42b1-ae73-b142c0eedaff
// IMPLEMENTED — the cast trigger taxes the caster the spell's mana value
// (`Amount::TargetCmc`) and counters it when they don't pay.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IN_THE_EYE_OF_CHAOS,
    oracle_id = "6a0da9f3-cb06-42b1-ae73-b142c0eedaff",
    scryfall_id = "6964fc80-7f39-4ffd-80fd-17d9a75e8e84",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "In the Eye of Chaos",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::HasType(TypeSet::INSTANT)),
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfEvent,
            mana: Amount::TargetCmc,
            effect: &Effect::CounterTargetSpell,
        }],
        targets = Some(TargetReq::one(TargetSpec::EventObject))
    )],
);

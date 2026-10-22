//! Dwarven Vigilantes — {2}{R} — Creature — Dwarf
//! Oracle: Whenever this creature attacks and isn't blocked, you may have it deal damage equal to its power to target creature. If you do, this creature assigns no combat damage this turn.
//! Set: VIS #77 — Visions | Scryfall ID: 077d33bb-41bf-440d-939b-67ab5aacb092 | Oracle ID: b49baf80-237c-41ee-8a22-2b6a68a313e7
// IMPLEMENTED — whenever it attacks and isn't blocked, you may have it deal damage equal to its power to target creature and assign no combat damage this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DWARVEN_VIGILANTES,
    oracle_id = "b49baf80-237c-41ee-8a22-2b6a68a313e7",
    scryfall_id = "077d33bb-41bf-440d-939b-67ab5aacb092",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dwarven Vigilantes",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[
                Effect::DealDamage {
                    amount: Amount::SourcePower,
                    target: TargetSpec::Object(&Filter::CREATURE),
                },
                Effect::ModifySource {
                    modifier: Modifier::AssignsNoCombatDamage,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);

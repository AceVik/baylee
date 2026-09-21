//! Rishkar, Peema Renegade — {2}{G} — Legendary Creature — Elf Druid
//! Oracle: When Rishkar enters, put a +1/+1 counter on each of up to two target creatures.
//! Oracle: Each creature you control with a counter on it has "{T}: Add {G}."
//! Set: CMM #317 — Commander Masters | Scryfall ID: 88f4aecc-b728-4960-8131-1e5aa6c3c030 | Oracle ID: 761021ce-4559-464e-aa03-85c2fe78e267
// PARTIAL — the enters trigger is built: a +1/+1 counter on each of up to two
// target creatures. The second sentence is not.
// NOT SUPPORTED: Each creature you control with a counter on it has "{T}: Add {G}."
// — no `Filter` variant can say "has a counter on it", and granting it to
// `Filter::YOUR_CREATURE` would hand the mana ability to every creature you
// control, counter or not.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RISHKAR_PEEMA_RENEGADE,
    oracle_id = "761021ce-4559-464e-aa03-85c2fe78e267",
    scryfall_id = "88f4aecc-b728-4960-8131-1e5aa6c3c030",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Rishkar, Peema Renegade",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::DRUID],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Partial("no Filter variant says \"has a counter on it\""),
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        targets = Some(TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 2)),
    )],
);

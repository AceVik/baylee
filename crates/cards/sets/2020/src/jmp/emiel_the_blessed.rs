//! Emiel the Blessed — {2}{W}{W} — Legendary Creature — Unicorn
//! Oracle: {3}: Exile another target creature you control, then return it to the battlefield under its owner's control.
//! Oracle: Whenever another creature you control enters, you may pay {G/W}. If you do, put a +1/+1 counter on it. If it's a Unicorn, put two +1/+1 counters on it instead. ({G/W} can be paid with either {G} or {W}.)
//! Set: 2X2 #10 — Double Masters 2022 | Scryfall ID: 0f594562-7e9f-47e6-a033-fb70e3cf1e10 | Oracle ID: b11c250c-f191-4c52-ba02-a9176f163447

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMIEL_THE_BLESSED,
    oracle_id = "b11c250c-f191-4c52-ba02-a9176f163447",
    scryfall_id = "0f594562-7e9f-47e6-a033-fb70e3cf1e10",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Emiel the Blessed",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::UNICORN],
        power = Some(4),
        toughness = Some(4),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        activated!(
            cost!("{3}"),
            &[Effect::blink_to_owner(TargetSpec::Object(
                &Filter::ANOTHER_CREATURE_YOU_CONTROL
            ))],
            target = Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE_YOU_CONTROL))
        ),
        triggered!(
            Trigger::EntersBattlefield(&Filter::ANOTHER_CREATURE_YOU_CONTROL),
            &[Effect::PlayerMayPayManaThen {
                player: PlayerRel::You,
                cost: mana!("{G/W}"),
                effects: &[Effect::IfTargetMatchesElse {
                    filter: &Filter::HasSubtype(subtypes::creature::UNICORN),
                    then: &[Effect::AddCounter {
                        kind: CounterKind::P1P1,
                        amount: Amount::Fixed(2)
                    }],
                    otherwise: &[Effect::AddCounter {
                        kind: CounterKind::P1P1,
                        amount: Amount::Fixed(1)
                    }],
                }],
            }],
            targets = Some(TargetReq::one(TargetSpec::EventObject))
        ),
    ],
);

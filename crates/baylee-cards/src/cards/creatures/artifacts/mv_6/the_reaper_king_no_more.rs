//! The Reaper, King No More — {2/B}{2/R}{2/G} — Legendary Artifact Creature — Scarecrow
//! Oracle: When The Reaper enters, put a -1/-1 counter on each of up to two target creatures.
//! Oracle: Whenever a creature an opponent controls with a -1/-1 counter on it dies, you may put that card onto the battlefield under your control. Do this only once each turn.
//! Set: ECC #4 — Lorwyn Eclipsed Commander | Scryfall ID: d2119f3f-27d2-4d43-a5ef-35c1b73d182a | Oracle ID: 39b67a4d-6a87-41f0-a86f-b66671ccc20d
// IMPLEMENTED — a -1/-1 counter on each of up to two target creatures, and the
// opponent's creatures that die wearing one are yours, once each turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_REAPER_KING_NO_MORE,
    oracle_id = "39b67a4d-6a87-41f0-a86f-b66671ccc20d",
    scryfall_id = "d2119f3f-27d2-4d43-a5ef-35c1b73d182a",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "The Reaper, King No More",
        mana_cost = mana!("{2/B}{2/R}{2/G}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::SCARECROW],
        power = Some(3),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // "When The Reaper enters, put a -1/-1 counter on each of up to two
        // target creatures."
        triggered!(
            Trigger::ETB,
            &[Effect::AddCounter {
                kind: CounterKind::M1M1,
                amount: Amount::Fixed(1),
            }],
            targets = Some(TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 2)),
        ),
        // "Whenever a creature an opponent controls with a -1/-1 counter on
        // it dies, you may put that card onto the battlefield under your
        // control. Do this only once each turn."
        triggered!(
            Trigger::Dies(&Filter::And(&[
                Filter::CREATURE,
                Filter::ControlledByOpponent,
                Filter::HasCounter(CounterKind::M1M1),
            ])),
            &[Effect::MayDoOnceEachTurn {
                effects: &[Effect::GraveyardToBattlefield {
                    target: TargetSpec::EventObject,
                    owner_control: false,
                    counters: None,
                }],
            }],
        ),
    ],
);

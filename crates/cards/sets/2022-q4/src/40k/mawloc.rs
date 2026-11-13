//! Mawloc — {X}{R}{G} — Creature — Tyranid
//! Oracle: Ravenous (This creature enters with X +1/+1 counters on it. If X is 5 or more, draw a card when it enters.)
//! Oracle: Terror from the Deep — When this creature enters, it fights up to one target creature an opponent controls. If that creature would die this turn, exile it instead.
//! Set: 40K #133 — Warhammer 40,000 Commander | Scryfall ID: bbec97ff-bb84-4496-9e09-d4fc1570c516 | Oracle ID: e5d928dc-b465-4cf7-ab11-d5bd3328f8e7
// IMPLEMENTED — ravenous (X +1/+1 counters, a card at X 5 or more), and it
// fights up to one opponent's creature, which is exiled if it would die this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MAWLOC,
    oracle_id = "e5d928dc-b465-4cf7-ab11-d5bd3328f8e7",
    scryfall_id = "bbec97ff-bb84-4496-9e09-d4fc1570c516",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(
        name = "Mawloc",
        mana_cost = mana!("{X}{R}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TYRANID],
        power = Some(2),
        toughness = Some(2),
        // Ravenous, its replacement half (CR 702.156a): "This permanent
        // enters with X +1/+1 counters on it."
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::X,
        }],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // Ravenous, its triggered half: "When this permanent enters, if X
        // is 5 or more, draw a card."
        triggered!(
            Trigger::ETB,
            &[Effect::draw(1)],
            condition = Some(Condition::XAtLeast(5)),
        ),
        // "Terror from the Deep — When this creature enters, it fights up to
        // one target creature an opponent controls. If that creature would
        // die this turn, exile it instead."
        triggered!(
            Trigger::ETB,
            &[
                Effect::Fight {
                    fighter: TargetSlot::This,
                    foe: TargetSlot::First,
                },
                Effect::ExileIfDiesThisTurn {
                    target: TargetSpec::Object(&Filter::OPPONENT_CREATURE),
                },
            ],
            targets = Some(TargetReq::up_to_one(TargetSpec::Object(
                &Filter::OPPONENT_CREATURE
            ))),
        ),
    ],
);

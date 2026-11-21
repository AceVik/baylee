//! Rule tests for `KeywordSet::PROVOKE` (CR 702.39a–b),
//! `Effect::MustBlockSourceIfAbleThisCombat`, and `Effect::MustBlockIfAble`
//! (CR 509.1c).
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::AnswerFault;
use crate::object::Status;
use baylee_cards_dsl::{Cost, CostPart, Duration, Filter, Modifier, TargetSpec};
use baylee_core::ids::Defender;

const PROVOKER: u32 = 9400;
const DEFENDER_BEAR: u32 = 9401;
const DEFENDER_WALL: u32 = 9402;
const GRANT_PROVOKE: u32 = 9403;
const SECOND_PROVOKER: u32 = 9404;
const FLYING_PROVOKER: u32 = 9405;
const TAXED_BLOCKER: u32 = 9406;
const INSTANT_KILL: u32 = 9407;
const INSTANT_BLINK: u32 = 9408;
const RAMPANT_CALLER: u32 = 9409;

static TAP_SELF_NOOP: &[AbilityDef] = &[paid(
    Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::TapSelf],
    },
    &[Effect::GainLife {
        amount: baylee_cards_dsl::Amount::Fixed(0),
    }],
    None,
)];

static GRANT_SECOND_PROVOKE: &[AbilityDef] = &[free(
    &[Effect::ModifyTargets {
        modifier: Modifier::AddKeyword(KeywordSet::PROVOKE),
        duration: Duration::UntilEndOfTurn,
    }],
    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

static TAXED_BLOCKER_ABILITIES: &[AbilityDef] = &[
    paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::TapSelf],
        },
        &[Effect::GainLife {
            amount: baylee_cards_dsl::Amount::Fixed(0),
        }],
        None,
    ),
    baylee_cards_dsl::static_ability!(Filter::This, Modifier::CantBlockUnlessPayMana(1)),
];

static KILL_TARGET: &[AbilityDef] = &[baylee_cards_dsl::spell!(
    &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

static BLINK_TARGET: &[AbilityDef] = &[baylee_cards_dsl::spell!(
    &[Effect::Blink {
        target: TargetSpec::Object(&Filter::CREATURE),
        owner_control: false,
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

static RAMPANT_ABILITY: &[AbilityDef] = &[free(
    &[Effect::MustBlockIfAble {
        blocker: TargetSpec::Object(&Filter::CREATURE),
        attacker: TargetSpec::ThisObject,
        this_combat_only: false,
    }],
    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

fn cards() -> Vec<&'static CardDef> {
    vec![
        card(
            PROVOKER,
            creature_face("Provoker", "{1}", 3, 3),
            KeywordSet::HASTE.union(KeywordSet::PROVOKE),
            &[],
        ),
        card(
            SECOND_PROVOKER,
            creature_face("Second Provoker", "{1}", 2, 2),
            KeywordSet::HASTE.union(KeywordSet::PROVOKE),
            &[],
        ),
        card(
            FLYING_PROVOKER,
            creature_face("Flying Provoker", "{1}", 2, 2),
            KeywordSet::HASTE
                .union(KeywordSet::FLYING)
                .union(KeywordSet::PROVOKE),
            &[],
        ),
        card(
            DEFENDER_BEAR,
            creature_face("Defender Bear", "{1}", 2, 2),
            KeywordSet::EMPTY,
            TAP_SELF_NOOP,
        ),
        card(
            DEFENDER_WALL,
            creature_face("Defender Wall", "{1}", 0, 5),
            KeywordSet::EMPTY,
            TAP_SELF_NOOP,
        ),
        card(
            GRANT_PROVOKE,
            face("Provoke Banner", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            GRANT_SECOND_PROVOKE,
        ),
        card(
            TAXED_BLOCKER,
            creature_face("Taxed Blocker", "{1}", 2, 2),
            KeywordSet::EMPTY,
            TAXED_BLOCKER_ABILITIES,
        ),
        card(
            INSTANT_KILL,
            face("Instant Kill", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            KILL_TARGET,
        ),
        card(
            INSTANT_BLINK,
            face("Instant Blink", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            BLINK_TARGET,
        ),
        card(
            RAMPANT_CALLER,
            creature_face("Rampant Caller", "{1}", 3, 3),
            KeywordSet::HASTE,
            RAMPANT_ABILITY,
        ),
    ]
}

fn tap_permanents_in_main(engine: &mut Bench, who: PlayerId, target: ObjectId) {
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(engine, who, target, 0);
    settle(engine, me());
    assert!(
        engine
            .state()
            .object(target)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}

#[test]
fn provoke_untaps_tapped_defender_enforces_block_and_expires_at_combat_end() {
    let mut engine = bench(
        9401,
        cards(),
        [
            Seat::with(&[PROVOKER, RAMPANT_CALLER]),
            Seat::with(&[DEFENDER_BEAR, DEFENDER_WALL]),
        ],
    );
    to_main(&mut engine, me());
    let provoker = the(&engine, ZoneLocation::Battlefield, PROVOKER);
    let rampant = the(&engine, ZoneLocation::Battlefield, RAMPANT_CALLER);
    let bear = the(&engine, ZoneLocation::Battlefield, DEFENDER_BEAR);
    let wall = the(&engine, ZoneLocation::Battlefield, DEFENDER_WALL);

    // Rampant Caller sets a turn-long pair requirement on Wall (`this_combat_only: false`).
    activate(&mut engine, me(), rampant, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![wall],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // Opponent taps Defender Bear before combat so Provoke has to untap it.
    tap_permanents_in_main(&mut engine, them(), bear);

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(provoker, Defender::Player(them()))],
            },
        )
        .unwrap();

    // Provoke triggers on attack and offers only defending player's creatures.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected Provoke target choice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![bear, wall]);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    assert!(
        !engine
            .state()
            .object(bear)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "choosing yes on Provoke untaps the target creature (CR 702.39a)"
    );

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let pending = engine.pending().clone();
    let Pending::ChooseBlockers {
        ref obeying,
        ref demands,
        ..
    } = pending
    else {
        unreachable!()
    };
    assert_eq!(obeying, &vec![(bear, provoker)]);
    assert_eq!(demands.len(), 1);
    assert_eq!(demands[0].count, 1);
    assert_eq!(
        pending.answer_fault(&PlayerAction::DeclareBlockers { blockers: vec![] }),
        Some(AnswerFault::MustBlock),
        "refusing to block with the provoked creature violates CR 509.1c"
    );

    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, provoker)],
            },
        )
        .unwrap();

    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), crate::turn::Step::Main)
            && e.state().turn.phase == Phase::SecondMain
    });
    assert!(
        objects(&engine, ZoneLocation::Graveyard(them()), DEFENDER_BEAR).len() == 1,
        "provoked 2/2 died blocking the 3/3 Provoker"
    );
    // At end of combat, Provoke's `this_combat_only: true` entry expired, while
    // Rampant Caller's `this_combat_only: false` entry remains until turn reset.
    assert_eq!(engine.state().per_turn.must_block_pairs.len(), 1);
    assert!(!engine.state().per_turn.must_block_pairs[0].this_combat_only);

    // On Seat 1's turn, `PerTurn::reset` clears all pair requirements.
    to_main(&mut engine, them());
    assert!(engine.state().per_turn.must_block_pairs.is_empty());
}

#[test]
fn declining_provoke_leaves_target_tapped_and_imposes_no_requirement() {
    let mut engine = bench(
        9402,
        cards(),
        [Seat::with(&[PROVOKER]), Seat::with(&[DEFENDER_BEAR])],
    );
    to_main(&mut engine, me());
    let provoker = the(&engine, ZoneLocation::Battlefield, PROVOKER);
    let bear = the(&engine, ZoneLocation::Battlefield, DEFENDER_BEAR);
    tap_permanents_in_main(&mut engine, them(), bear);

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(provoker, Defender::Player(them()))],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    assert!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "declining Provoke leaves the creature tapped"
    );
    assert!(engine.state().per_turn.must_block_pairs.is_empty());
}

#[test]
fn multiple_provoke_instances_trigger_separately_and_maximize_requirements() {
    let mut engine = bench(
        9403,
        cards(),
        [
            Seat::with(&[PROVOKER, SECOND_PROVOKER, GRANT_PROVOKE]),
            Seat::with(&[DEFENDER_WALL]),
        ],
    );
    to_main(&mut engine, me());
    let provoker = the(&engine, ZoneLocation::Battlefield, PROVOKER);
    let second = the(&engine, ZoneLocation::Battlefield, SECOND_PROVOKER);
    let banner = the(&engine, ZoneLocation::Battlefield, GRANT_PROVOKE);
    let wall = the(&engine, ZoneLocation::Battlefield, DEFENDER_WALL);

    // Grant a second instance of provoke to `provoker` (CR 702.39b).
    activate(&mut engine, me(), banner, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![provoker],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (provoker, Defender::Player(them())),
                    (second, Defender::Player(them())),
                ],
            },
        )
        .unwrap();

    // Three Provoke triggers total (2 from `provoker`, 1 from `second`).
    for _ in 0..3 {
        let Pending::ChooseTargets { .. } = engine.pending() else {
            panic!("expected 3 separate Provoke target prompts");
        };
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![wall],
                    players: vec![],
                },
            )
            .unwrap();
    }
    for _ in 0..3 {
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::YesNo { .. })
        });
        engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    }

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let pending = engine.pending().clone();
    // Blocking `second` obeys only 1 requirement while blocking `provoker`
    // obeys 2; CR 509.1c requires maximizing obeyed requirements.
    assert_eq!(
        pending.answer_fault(&PlayerAction::DeclareBlockers {
            blockers: vec![(wall, second)],
        }),
        Some(AnswerFault::MustBlock)
    );
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, provoker)],
            },
        )
        .unwrap();
}

#[test]
fn evasion_or_mana_tax_exempts_impossible_or_taxed_provoke_block() {
    let mut engine = bench(
        9404,
        cards(),
        [
            Seat::with(&[FLYING_PROVOKER, PROVOKER]),
            Seat::with(&[DEFENDER_BEAR, TAXED_BLOCKER]),
        ],
    );
    to_main(&mut engine, me());
    let flyer = the(&engine, ZoneLocation::Battlefield, FLYING_PROVOKER);
    let ground_provoker = the(&engine, ZoneLocation::Battlefield, PROVOKER);
    let bear = the(&engine, ZoneLocation::Battlefield, DEFENDER_BEAR);
    let taxed = the(&engine, ZoneLocation::Battlefield, TAXED_BLOCKER);

    tap_permanents_in_main(&mut engine, them(), bear);
    tap_permanents_in_main(&mut engine, them(), taxed);

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (flyer, Defender::Player(them())),
                    (ground_provoker, Defender::Player(them())),
                ],
            },
        )
        .unwrap();

    // Flyer provokes ground `bear`; `ground_provoker` provokes `taxed` blocker.
    for target in [bear, taxed] {
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![target],
                    players: vec![],
                },
            )
            .unwrap();
    }
    for _ in 0..2 {
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::YesNo { .. })
        });
        engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    }
    // Both creatures untapped on `YesNo(true)`.
    assert!(
        !engine
            .state()
            .object(bear)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(
        !engine
            .state()
            .object(taxed)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    // `bear` cannot block `flyer` (Flying) and `taxed` requires paying {1}
    // (CR 509.1c cost exemption), so declaring zero blockers is legal!
    engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("neither impossible block nor mana-taxed block is forced");
}

#[test]
fn multiplayer_provoke_targets_defending_player_and_respects_lki_and_blink() {
    let third = PlayerId::new(2);
    let mut engine = bench(
        9405,
        cards(),
        [
            Seat::with(&[PROVOKER, SECOND_PROVOKER]),
            Seat::with(&[DEFENDER_BEAR, DEFENDER_WALL]).holding(&[INSTANT_KILL, INSTANT_BLINK]),
            Seat::with(&[DEFENDER_BEAR]),
        ],
    );
    to_main(&mut engine, me());
    let provoker = the(&engine, ZoneLocation::Battlefield, PROVOKER);
    let second = the(&engine, ZoneLocation::Battlefield, SECOND_PROVOKER);
    let p1_bears: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == them() && o.card.is_some_and(|c| c.index.get() == DEFENDER_BEAR)
            })
        })
        .collect();
    let p1_bear = p1_bears[0];
    let wall = the(&engine, ZoneLocation::Battlefield, DEFENDER_WALL);
    let p2_bear = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == third && o.card.is_some_and(|c| c.index.get() == DEFENDER_BEAR)
            })
        })
        .unwrap();

    tap_permanents_in_main(&mut engine, them(), p1_bear);

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    // Both attackers attack Seat 1 (`them()`), NOT Seat 2 (`third`).
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (provoker, Defender::Player(them())),
                    (second, Defender::Player(them())),
                ],
            },
        )
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected Provoke target choice");
    };
    assert!(
        !options.contains(&p2_bear),
        "multiplayer Provoke only targets creatures controlled by the attacked defending player (CR 508.5a)"
    );
    assert_eq!(options, vec![p1_bear, wall]);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![p1_bear],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![wall],
                players: vec![],
            },
        )
        .unwrap();

    // Seat 0 passes priority; Seat 1 destroys `provoker` before its trigger resolves!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let kill = the(&engine, ZoneLocation::Hand(them()), INSTANT_KILL);
    engine
        .apply(them(), PlayerAction::CastSpell { card: kill })
        .unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![provoker],
                players: vec![],
            },
        )
        .unwrap();

    // Resolve `second`'s trigger on `wall`, then `provoker`'s trigger on `p1_bear`.
    for _ in 0..2 {
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::YesNo { .. })
        });
        engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    }
    assert!(
        !engine
            .state()
            .object(p1_bear)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "Provoke still untaps its target even when the attacking source died before resolution"
    );

    // Now blink `wall` before DeclareBlockers: the new incarnation has no requirement (CR 400.7)!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == them())
            && e.state().zones.stack_is_empty()
    });
    let blink = the(&engine, ZoneLocation::Hand(them()), INSTANT_BLINK);
    engine
        .apply(them(), PlayerAction::CastSpell { card: blink })
        .unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![wall],
                players: vec![],
            },
        )
        .unwrap();

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("destroyed attacker and blinked blocker leave zero active requirements");
}

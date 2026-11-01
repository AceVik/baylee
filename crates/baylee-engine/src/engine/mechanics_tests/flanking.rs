//! Separate flanking instances trigger separately and retain the blocking
//! incarnation (CR 702.25b, 400.7). Flanking does not target its blocker.

use super::*;
use baylee_cards_dsl::{Duration, Filter, Modifier, TargetSpec};
use baylee_core::ids::Defender;

const RIDER: u32 = 7400;
const BLOCKER: u32 = 7401;
const TRAINER: u32 = 7402;
const CREATURE: TargetSpec = TargetSpec::Object(&Filter::CREATURE);

static TRAINING: &[AbilityDef] = &[
    free(
        &[Effect::continuous(
            &Filter::This,
            Modifier::AddKeyword(KeywordSet::FLANKING),
            Duration::UntilEndOfTurn,
        )],
        Some(TargetReq::one(CREATURE)),
    ),
    free(
        &[Effect::continuous(
            &Filter::This,
            Modifier::RemoveKeyword(KeywordSet::FLANKING),
            Duration::UntilEndOfTurn,
        )],
        Some(TargetReq::one(CREATURE)),
    ),
    free(
        &[Effect::continuous(
            &Filter::This,
            Modifier::LoseAllAbilities,
            Duration::UntilEndOfTurn,
        )],
        Some(TargetReq::one(CREATURE)),
    ),
    free(
        &[Effect::blink_to_owner(CREATURE)],
        Some(TargetReq::one(CREATURE)),
    ),
];

fn fixture(native: KeywordSet, blocking: KeywordSet) -> (Bench, ObjectId, ObjectId, ObjectId) {
    let mut engine = bench(
        7400,
        vec![
            card(RIDER, creature_face("Rider", "{0}", 2, 12), native, &[]),
            card(
                BLOCKER,
                creature_face("Blocker", "{0}", 2, 12),
                blocking,
                &[],
            ),
            card(
                TRAINER,
                face("Trainer", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                TRAINING,
            ),
        ],
        [Seat::with(&[RIDER, TRAINER]), Seat::with(&[BLOCKER])],
    );
    to_main(&mut engine, me());
    let rider = the(&engine, ZoneLocation::Battlefield, RIDER);
    let blocker = the(&engine, ZoneLocation::Battlefield, BLOCKER);
    let trainer = the(&engine, ZoneLocation::Battlefield, TRAINER);
    (engine, rider, blocker, trainer)
}

fn train(engine: &mut Bench, trainer: ObjectId, creature: ObjectId, ability: u32) {
    activate(engine, me(), trainer, ability);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![creature],
                players: vec![],
            },
        )
        .unwrap();
    settle(engine, me());
}

fn block(engine: &mut Bench, rider: ObjectId, blocker: ObjectId) {
    walk_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(rider, Defender::Player(them()))],
            },
        )
        .unwrap();
    walk_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, rider)],
            },
        )
        .unwrap();
}

fn toughness(engine: &Bench, creature: ObjectId) -> i16 {
    engine
        .state()
        .object(creature)
        .unwrap()
        .characteristics()
        .toughness
        .unwrap()
}

#[test]
fn native_and_repeated_grants_each_shrink_the_blocker() {
    for (native, expected) in [(KeywordSet::EMPTY, 10), (KeywordSet::FLANKING, 9)] {
        let (mut engine, rider, blocker, trainer) = fixture(native, KeywordSet::EMPTY);
        train(&mut engine, trainer, rider, 0);
        train(&mut engine, trainer, rider, 0);
        block(&mut engine, rider, blocker);
        settle(&mut engine, me());
        assert_eq!(toughness(&engine, blocker), expected);
    }
}

#[test]
fn removal_discards_all_previous_instances_but_later_grants_work() {
    for removal in [1, 2] {
        for grant_again in [false, true] {
            let (mut engine, rider, blocker, trainer) =
                fixture(KeywordSet::FLANKING, KeywordSet::EMPTY);
            train(&mut engine, trainer, rider, 0);
            train(&mut engine, trainer, rider, removal);
            if grant_again {
                train(&mut engine, trainer, rider, 0);
            }
            block(&mut engine, rider, blocker);
            settle(&mut engine, me());
            assert_eq!(
                toughness(&engine, blocker),
                if grant_again { 11 } else { 12 }
            );
        }
    }
}

#[test]
fn a_flanking_blocker_is_exempt_from_every_instance() {
    let (mut engine, rider, blocker, trainer) = fixture(KeywordSet::FLANKING, KeywordSet::FLANKING);
    train(&mut engine, trainer, rider, 0);
    block(&mut engine, rider, blocker);
    settle(&mut engine, me());
    assert_eq!(toughness(&engine, blocker), 12);
}

#[test]
fn flanking_does_not_target_a_hexproof_blocker() {
    let (mut engine, rider, blocker, _) = fixture(KeywordSet::FLANKING, KeywordSet::HEXPROOF);
    block(&mut engine, rider, blocker);
    settle(&mut engine, me());
    assert_eq!(toughness(&engine, blocker), 11);
}

#[test]
fn flanking_does_not_shrink_a_blocker_that_left_and_returned_in_response() {
    let (mut engine, rider, blocker, trainer) = fixture(KeywordSet::FLANKING, KeywordSet::EMPTY);
    block(&mut engine, rider, blocker);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
    );
    let old_version = engine.state().object(blocker).unwrap().version;
    train(&mut engine, trainer, blocker, 3);
    assert!(engine.state().object(blocker).unwrap().version > old_version);
    assert_eq!(toughness(&engine, blocker), 12);
}

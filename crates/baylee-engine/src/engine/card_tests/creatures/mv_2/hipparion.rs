//! `cards/creatures/mv_2/hipparion.rs`, played.
//!
//! Oracle:
//! - "This creature can't block creatures with power 3 or greater unless you pay {1}."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hipparion() -> CardIndex {
    card_index("306fd9d2-5ded-4857-90f4-e62d73c6aed2")
}

#[test]
fn hipparion_casts_and_blocks_in_combat() {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], &[], &[hipparion()]);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(
        &mut engine,
        P1,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    lk::cast_at(&mut engine, P1, hipparion(), &[], &[]);
    let horse = lk::obj(&engine, P1, hipparion());
    assert_eq!(power_of(&engine, horse), Some(1));

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let offer = lk::declare(&mut engine, &[bears]);
    assert!(lk::may_block(&offer, horse, bears));
}

fn blocking_game(
    attacker: CardIndex,
    with_mana_source: bool,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut defender_board = vec![hipparion()];
    if with_mana_source {
        defender_board.push(index::FOREST);
    }
    let engine = lk::game(SEED, &[attacker], &defender_board, &[], &[]);
    let attacker = lk::obj(&engine, P0, attacker);
    let horse = lk::obj(&engine, P1, hipparion());
    (engine, horse, attacker)
}

#[test]
fn hipparion_blocks_power_two_or_less_without_paying() {
    let (mut engine, horse, bears) = blocking_game(index::GRIZZLY_BEARS, false);
    let offer = lk::attack(&mut engine, &[bears]);
    assert!(lk::may_block(&offer, horse, bears));

    lk::block(&mut engine, &[(horse, bears)]);
    assert!(engine.state().combat.is_blocking(horse, bears));
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
}

#[test]
fn hipparion_cannot_block_power_three_without_mana() {
    let (mut engine, horse, giant) = blocking_game(index::HILL_GIANT, false);
    let offer = lk::attack(&mut engine, &[giant]);
    assert!(
        !lk::may_block(&offer, horse, giant),
        "an unpayable conditional block is absent from every seat's offer"
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(horse, giant)],
                },
            )
            .is_err(),
        "the declaration is illegal even if a caller bypasses the offer"
    );
}

#[test]
fn hipparion_pays_one_to_block_power_three_or_greater() {
    let (mut engine, horse, giant) = blocking_game(index::HILL_GIANT, true);
    let offer = lk::attack(&mut engine, &[giant]);
    assert!(
        lk::may_block(&offer, horse, giant),
        "the Forest makes the conditional block payable in the offer"
    );

    lk::block(&mut engine, &[(horse, giant)]);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the conditional block opens a mana payment window");
    };
    for source in legal.mana_abilities {
        engine
            .apply(P1, PlayerAction::ActivateManaAbility { source })
            .expect("tap the Forest for the conditional block");
    }
    engine
        .apply(P1, PlayerAction::PassPriority)
        .expect("pay the one generic mana");
    assert!(engine.state().combat.is_blocking(horse, giant));
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
}

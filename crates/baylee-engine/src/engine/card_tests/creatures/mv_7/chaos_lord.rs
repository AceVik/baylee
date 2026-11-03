//! `cards/creatures/mv_7/chaos_lord.rs`, played.
//!
//! Oracle:
//! - "First strike"
//! - "At the beginning of your upkeep, target opponent gains control of this creature if the number of permanents is even."
//! - "This creature can attack as though it had haste unless it entered this turn."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "16d0aea2-4ca4-4a9d-8674-2863e085f514"
fn chaos_lord() -> CardIndex {
    card_index("16d0aea2-4ca4-4a9d-8674-2863e085f514")
}

/// First strike played through combat: a 6/4 Craw Wurm blocking Chaos Lord
/// dies in the first-strike damage step and never deals its 6. Without first
/// strike both would deal damage at once and the Lord would end combat with
/// 6 damage marked. The third permanent keeps the upkeep trigger from firing.
#[test]
fn chaos_lord_kills_its_blocker_before_the_blocker_deals_damage() {
    let mut engine = lk::game(
        SEED,
        &[chaos_lord(), forest()],
        &[index::CRAW_WURM],
        &[],
        &[],
    );
    let lord = lk::obj(&engine, P0, chaos_lord());
    let wurm = lk::obj(&engine, P1, index::CRAW_WURM);

    let _ = lk::attack(&mut engine, &[lord]);
    lk::block(&mut engine, &[(wurm, lord)]);
    lk::through_combat(&mut engine);

    assert!(
        on_battlefield(&engine, P1, index::CRAW_WURM).is_none(),
        "the blocker died to Chaos Lord's 7 damage"
    );
    assert_eq!(
        lk::damage_on(&engine, lord),
        0,
        "first strike: the Wurm was dead before regular combat damage"
    );
}

#[test]
fn chaos_lord_targets_an_opponent_and_changes_control_at_even_permanent_count() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[chaos_lord(), forest()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the even permanent count puts the targeted trigger on the stack")
    };
    assert_eq!(player, P0, "Chaos Lord's controller chooses the target");
    assert!(options.is_empty(), "this ability targets a player");
    assert_eq!(player_options, vec![P1], "only an opponent is legal");

    lk::aim(&mut engine, &[], &[P1]);
    lk::settle(&mut engine);

    let lord = lk::obj(&engine, P1, chaos_lord());
    let object = engine.state().object(lord).unwrap();
    assert_eq!(object.controller, P1, "the targeted opponent gains control");
    assert_eq!(object.owner, P0, "a control change never changes ownership");
}

#[test]
fn chaos_lord_does_not_trigger_at_an_odd_permanent_count() {
    let engine = lk::game(SEED, &[chaos_lord()], &[], &[], &[]);

    assert!(
        stack_is_empty(&engine),
        "the intervening-if is false when the Lord is the only permanent"
    );
    let lord = lk::obj(&engine, P0, chaos_lord());
    assert_eq!(engine.state().object(lord).unwrap().controller, P0);
}

#[test]
fn chaos_lord_rechecks_parity_when_its_trigger_resolves() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[chaos_lord(), grizzly_bears()])
        .hand(1, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    let lord = lk::obj(&engine, P0, chaos_lord());
    let bears = lk::obj(&engine, P0, grizzly_bears());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut engine, &[], &[P1]);

    // Respond to the trigger by reducing the count from two permanents to one.
    lk::pass_once(&mut engine);
    lk::float(&mut engine, P1, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P1, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, |e| {
        on_battlefield(e, P0, grizzly_bears()).is_none()
            && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    assert!(
        !stack_is_empty(&engine),
        "the upkeep trigger is still below Lightning Bolt"
    );
    assert_eq!(engine.state().object(lord).unwrap().controller, P0);

    lk::settle(&mut engine);
    assert_eq!(
        engine.state().object(lord).unwrap().controller,
        P0,
        "the intervening-if is false on resolution, so control does not change"
    );
}

#[test]
fn chaos_lord_cannot_attack_on_the_turn_it_enters() {
    let mut engine = lk::game(SEED, &[], &[], &[chaos_lord()], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 7)]);
    cast_with_floating(&mut engine, P0, chaos_lord());
    lk::settle(&mut engine);
    let lord = lk::obj(&engine, P0, chaos_lord());

    let attackers = lk::attackers_offered(&mut engine);
    assert!(
        !attackers.contains(&lord),
        "the exception applies only when Chaos Lord did not enter this turn"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(lord, Defender::Player(P1))],
                },
            )
            .is_err(),
        "an actual declaration with the newly entered Lord is illegal"
    );
}

#[test]
fn chaos_lord_can_attack_after_a_control_change_if_it_entered_on_an_earlier_turn() {
    let mut engine = lk::game(SEED, &[chaos_lord()], &[], &[], &[index::DOMINATE]);
    let lord = lk::obj(&engine, P0, chaos_lord());
    reach_their_main_phase(&mut engine, P1);

    // Dominate grants no haste: only Chaos Lord's own final sentence can make
    // this immediate attack legal after the change of control.
    lk::float(&mut engine, P1, &[(ManaColor::Blue, 10)]);
    cast_with_floating(&mut engine, P1, index::DOMINATE);
    engine.apply(P1, PlayerAction::ChooseNumber(7)).unwrap();
    lk::aim(&mut engine, &[lord], &[]);
    lk::settle(&mut engine);
    assert_eq!(engine.state().object(lord).unwrap().controller, P1);

    let _ = lk::attack(&mut engine, &[lord]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(
        lk::life(&engine, P0),
        13,
        "the Lord was declared as an attacker and dealt its 7 combat damage"
    );
}

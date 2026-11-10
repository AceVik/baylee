//! `cards/instants/mv_3/boiling_blood.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn boiling_blood() -> CardIndex {
    card_index("cd55a003-da0d-4522-b08a-d0731b1050c6")
}

/// `Boiling Blood` ({2}{R}, Instant) forces target creature to attack this turn
/// if able and draws a card.
#[test]
fn boiling_blood_forces_target_creature_to_attack_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), GRIZZLY_BEARS])
        .hand(0, &[boiling_blood()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, boiling_blood());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(hand_after, hand_before);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    assert!(
        engine
            .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err()
    );
}

/// "*Target* creature attacks": the one it names must attack, and a creature
/// of the same controller that it does not name may stay home.
#[test]
fn boiling_blood_forces_only_the_creature_it_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                GRIZZLY_BEARS,
                HILL_GIANT,
            ],
        )
        .hand(0, &[boiling_blood()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, boiling_blood());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(giant, Defender::Player(p1))],
                },
            )
            .is_err(),
        "the targeted Bears may not stay home while the Giant goes"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .expect("the Bears alone satisfy it: the Giant was not named");
}

//! `cards/creatures/mv_3/death_charmer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn death_charmer() -> CardIndex {
    card_index("491606d8-e9c1-4f89-bbef-b7590d310097")
}

#[test]
fn death_charmer_makes_damaged_creatures_controller_pay_two_or_lose_two_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[death_charmer()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let charmer = on_battlefield(&engine, p0, death_charmer()).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(charmer, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, charmer)],
            },
        )
        .unwrap();

    // Even though both creatures die in combat, Death Charmer's trigger resolves
    // and p1 declines (or cannot pay) {2}, losing 2 life.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 18);
}

#[test]
fn death_charmer_trigger_is_prevented_when_controller_pays_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[death_charmer()])
        .battlefield(1, &[index::WALL_OF_STONE, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let charmer = on_battlefield(&engine, p0, death_charmer()).unwrap();
    let wall = on_battlefield(&engine, p1, index::WALL_OF_STONE).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(charmer, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, charmer)],
            },
        )
        .unwrap();

    // Float {G}{G} for p1 when p1 gets priority before the trigger resolves, then pay {2}.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && !stack_is_empty(e)
    });
    tap_all_mana(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 20);
}

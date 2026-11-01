//! `cards/creatures/mv_3/orcish_farmer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn orcish_farmer() -> CardIndex {
    card_index("c3039d19-8c98-4953-8943-9922b6ab45ef")
}

/// Partial Orcish Farmer keeps its 2/2 body. The optional land-type change is
/// omitted until its duration can end before the affected controller's untap
/// determination, rather than after permanents have already untapped.
#[test]
fn orcish_farmer_attacks_for_two_without_the_unsupported_land_change() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[orcish_farmer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, orcish_farmer());
    pass_until(&mut engine, stack_is_empty);
    let farmer = on_battlefield(&engine, p0, orcish_farmer()).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !ability_offered(&engine, farmer, 0),
        "the optional land change is unavailable after summoning sickness ends"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: farmer,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the unsupported activation is refused"
    );
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(farmer, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the body deals 2 damage"
    );
    assert_eq!(engine.state().players[0].life, 20);
}

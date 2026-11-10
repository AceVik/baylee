//! `cards/creatures/artifacts/mv_5/darksteel_juggernaut.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn darksteel_juggernaut() -> CardIndex {
    card_index("4893a301-29a8-4a0e-845c-8ddb5cb8db25")
}

/// Darksteel Juggernaut — "power and toughness are each equal to the number
/// of artifacts you control": it counts itself and each other artifact of
/// its controller, and never the opponent's.
#[test]
fn darksteel_juggernaut_counts_the_artifacts_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[sol_ring()])
        .battlefield(0, &[darksteel_juggernaut(), sol_ring(), forest()])
        .battlefield(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let jugg = on_battlefield(&engine, p0, darksteel_juggernaut()).expect("seated");
    assert_eq!(
        pt(&engine, jugg),
        (2, 2),
        "itself and a Sol Ring; theirs is not counted"
    );

    cast_from_hand(&mut engine, p0, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, jugg), (3, 3), "a third artifact of mine");
}

/// "Indestructible" and "This creature attacks each combat if able": an empty
/// declaration is refused, the attack is accepted, and destruction does not
/// kill it.
#[test]
fn darksteel_juggernaut_is_indestructible_and_must_attack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[darksteel_juggernaut()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let jugg = on_battlefield(&engine, p0, darksteel_juggernaut()).expect("seated");
    assert!(keywords(&engine, jugg).contains(KeywordSet::INDESTRUCTIBLE));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    assert!(
        engine
            .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "it attacks each combat if able"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(jugg, Defender::Player(p1))],
            },
        )
        .expect("the forced attack is legal");
    through_combat(&mut engine);
    assert_eq!(life_of(&engine, 1), 19, "a 1/1 for the lone artifact");

    kill(&mut engine, jugg);
    assert_eq!(
        engine.state().object(jugg).map(|o| o.zone),
        Some(Zone::Battlefield),
        "destruction does not kill an indestructible creature"
    );
}

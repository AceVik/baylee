//! `cards/creatures/mv_2/kyren_glider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "6ae3a901-9f9a-4a99-94db-22c38fda7aef"
fn kyren_glider() -> CardIndex {
    card_index("6ae3a901-9f9a-4a99-94db-22c38fda7aef")
}

/// `Kyren Glider` (`Coverage::Generated`):
/// "Flying. This creature can't block."
///
/// Verifies that `Kyren Glider` has flying and `CANT_BLOCK`, can attack over ground
/// creatures, and cannot be declared as a blocker.
#[test]
fn kyren_glider_flies_and_cannot_block() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9404, mountain())
        .battlefield(0, &[mountain(), mountain(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[kyren_glider()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, kyren_glider());
    pass_until(&mut engine, stack_is_empty);

    let glider = on_battlefield(&engine, p0, kyren_glider()).expect("Kyren Glider on battlefield");
    assert!(keywords(&engine, glider).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, glider).contains(KeywordSet::CANT_BLOCK));

    // On P1's turn, P1 attacks with Grizzly Bears; Kyren Glider cannot block.
    assert!(walk_to_own_main(&mut engine, p1));
    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 bear");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(opp_bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(glider, opp_bear)],
                },
            )
            .is_err(),
        "Kyren Glider cannot block"
    );
}

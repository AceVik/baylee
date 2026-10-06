//! `cards/lands/utility/quicksand.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Quicksand: "{T}: Add {C}." / "{T}, Sacrifice this land: Target attacking creature without flying gets -1/-2 until end of turn."
/// When an attacking Llanowar Elves without flying is targeted, Quicksand is sacrificed to pay the cost.
/// On resolution, the -1/-2 continuous reduction reduces the 1/1 creature's toughness to -1, destroying it via state-based actions.
#[test]
fn quicksand_sacrifices_to_shrink_and_destroy_attacking_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(106, forest())
        .battlefield(0, &[quicksand(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    let __qs = on_battlefield(&engine, p0, quicksand()).expect("Quicksand deployed");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elves, Defender::Player(p1))],
            },
        )
        .unwrap();

    activate(&mut engine, p0, quicksand(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "attacking elves is legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(in_graveyard(&engine, p0, quicksand()).is_some());
}

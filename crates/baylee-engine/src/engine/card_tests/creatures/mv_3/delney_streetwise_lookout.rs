//! `cards/creatures/mv_3/delney_streetwise_lookout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Creatures you control with power 2 or less can't be blocked by
/// creatures with power 3 or greater." Delney (2/2) and a 4/3 attack: the
/// defender's 4/3 may block the 4/3 and not Delney; its 2/2 may block
/// either.
#[test]
fn delney_keeps_the_big_blockers_off_her_small_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[delney_streetwise_lookout(), thundering_giant()])
        .battlefield(1, &[thundering_giant(), steadfast_guard()])
        .start();
    keep_mulligans(&mut engine);
    let delney = on_battlefield(&engine, p0, delney_streetwise_lookout()).unwrap();
    let mine = on_battlefield(&engine, p0, thundering_giant()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { attackers, .. }
            if attackers.contains(&delney) && attackers.contains(&mine))
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(delney, Defender::Player(p1)), (mine, Defender::Player(p1))],
            },
        )
        .expect("both may attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let offered = |card: CardIndex| -> Vec<ObjectId> {
        let id = on_battlefield(&engine, p1, card).unwrap();
        blockers
            .iter()
            .find(|o| o.blocker == id)
            .map(|o| o.attackers.clone())
            .unwrap_or_default()
    };
    let giant = offered(thundering_giant());
    assert!(giant.contains(&mine), "a 4/3 on a 4/3");
    assert!(
        !giant.contains(&delney),
        "power 3 or greater on power 2 or less"
    );
    let guard = offered(steadfast_guard());
    assert!(
        guard.contains(&delney) && guard.contains(&mine),
        "a 2/2 may block either"
    );
}

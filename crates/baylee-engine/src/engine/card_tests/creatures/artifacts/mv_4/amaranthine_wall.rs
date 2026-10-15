//! `cards/creatures/artifacts/mv_4/amaranthine_wall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn amaranthine_wall() -> CardIndex {
    card_index("3c9fcda6-3ebc-4f9d-9f73-2bb4252d9420")
}

/// Amaranthine Wall — {4} 0/6 with defender and "{2}: This creature gains
/// indestructible until end of turn." Defender keeps it off the attackers
/// list beside a Grizzly Bears that is on it; paying {2} grants
/// indestructible, which the end of the turn takes away.
#[test]
fn amaranthine_wall_defends_and_buys_indestructible() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[amaranthine_wall(), grizzly_bears(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, amaranthine_wall()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, wall), (0, 6));
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));
    assert!(!keywords(&engine, wall).contains(KeywordSet::INDESTRUCTIBLE));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, amaranthine_wall(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, wall).contains(KeywordSet::INDESTRUCTIBLE));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{2}} paid");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for this");
    };
    assert!(attackers.contains(&bears), "the Bears may attack");
    assert!(!attackers.contains(&wall), "defender: the Wall may not");

    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert!(
        !keywords(&engine, wall).contains(KeywordSet::INDESTRUCTIBLE),
        "until end of turn"
    );
}

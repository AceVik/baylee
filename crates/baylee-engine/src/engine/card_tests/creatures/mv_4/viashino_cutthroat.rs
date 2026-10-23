//! `cards/creatures/mv_4/viashino_cutthroat.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn viashino_cutthroat() -> CardIndex {
    card_index("6dc1305a-d6ca-4280-ba5d-5ed91f78ced0")
}

#[test]
fn viashino_cutthroat_has_haste_and_returns_to_hand_at_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[viashino_cutthroat()])
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, viashino_cutthroat());
    pass_until(&mut engine, stack_is_empty);

    let cutthroat = on_battlefield(&engine, p0, viashino_cutthroat()).unwrap();
    assert!(keywords(&engine, cutthroat).contains(KeywordSet::HASTE));
    assert_eq!(pt(&engine, cutthroat), (5, 3));
    crate::engine::card_tests::legends_kit::attack(&mut engine, &[cutthroat]);

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, viashino_cutthroat()).is_none());
    assert!(in_hand(&engine, p0, viashino_cutthroat()).is_some());
    assert_eq!(
        engine.state().players[1].life,
        15,
        "haste allowed the cast-turn attack"
    );
}

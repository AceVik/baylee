//! `cards/lands/utility/alchemist_s_refuge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn alchemist_s_refuge_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(123, forest())
        .battlefield(0, &[alchemist_s_refuge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let refuge = on_battlefield(&engine, p0, alchemist_s_refuge()).expect("Refuge deployed");

    activate(&mut engine, p0, alchemist_s_refuge(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, refuge));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == refuge && *ai == 1),
        "the tapped land cannot pay the tap cost"
    );
}

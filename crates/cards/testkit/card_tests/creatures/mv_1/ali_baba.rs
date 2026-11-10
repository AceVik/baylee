//! `cards/creatures/mv_1/ali_baba.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ali Baba` prints `{{R}}: Tap target Wall.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Ali Baba` and a Mountain, while seat 1 controls an untapped Wall and an untapped Elf.
/// Activating the ability targets only the Wall — filtering out the Elf and `Ali Baba` itself — and upon resolution taps the Wall.
#[test]
fn ali_baba_targets_and_taps_a_wall() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let wall_card = card_index("5ccb57e1-ca94-4b5a-8e5f-b8b5e692cfb9");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ali_baba(), mountain()])
        .battlefield(1, &[wall_card, llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ali = on_battlefield(&engine, p0, ali_baba()).expect("ali baba seated");
    let wall = on_battlefield(&engine, p1, wall_card).expect("wall seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("elf seated");

    assert!(!is_tapped(&engine, wall));
    assert!(!is_tapped(&engine, elf));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, ali_baba(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&wall),
        "the Wall is offered as a legal target"
    );
    assert!(
        !options.contains(&elf),
        "the Elf is not a Wall and cannot be targeted"
    );
    assert!(
        !options.contains(&ali),
        "`Ali Baba` is not a Wall and cannot be targeted"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wall],
            },
        )
        .expect("target Wall chosen");

    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, wall),
        "the target Wall is now tapped upon resolution"
    );
    assert!(
        !is_tapped(&engine, elf),
        "the bystander Elf remains untapped"
    );
}

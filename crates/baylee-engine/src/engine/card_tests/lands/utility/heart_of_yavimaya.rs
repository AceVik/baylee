//! `cards/lands/utility/heart_of_yavimaya.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Heart of Yavimaya prints `If this land would enter, sacrifice a Forest instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.`, `{{T}}: Add {{G}}.`, and `{{T}}: Target creature gets +1/+1 until end of turn.`
///
/// Under `Coverage::Partial`, the printed Forest-sacrifice entry replacement is omitted because replacement effects of this shape cannot be expressed.
/// When played from hand with no Forest on the battlefield, it enters untapped without being sacrificed.
/// Activating ability 1 pumps target creature (`llanowar_elves()`) by +1/+1 until end of turn and taps the land.
#[test]
fn heart_of_yavimaya_enters_without_sacrifice_and_pumps_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[llanowar_elves()])
        .hand(0, &[heart_of_yavimaya()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf on battlefield");
    assert_eq!(pt(&engine, elf), (1, 1));

    let land = play_land(&mut engine, p0, heart_of_yavimaya());
    assert!(!is_tapped(&engine, land), "enters untapped");
    assert!(
        on_battlefield(&engine, p0, heart_of_yavimaya()).is_some(),
        "heart of yavimaya landed without sacrificing a forest"
    );

    activate(&mut engine, p0, heart_of_yavimaya(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf), (2, 2), "elf received +1/+1 pump");
    assert!(is_tapped(&engine, land));
}

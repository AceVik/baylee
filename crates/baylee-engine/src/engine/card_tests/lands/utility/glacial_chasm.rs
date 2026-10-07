//! `cards/lands/utility/glacial_chasm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Glacial Chasm: "Cumulative upkeep—Pay 2 life." / "When this land enters, sacrifice a land." / "Creatures you control can't attack." / "Prevent all damage that would be dealt to you."
/// Under `Coverage::Partial`, cumulative upkeep, attack prevention, and damage prevention are omitted.
/// When Glacial Chasm enters the battlefield, its enters-trigger forces its controller to sacrifice a land.
#[test]
fn glacial_chasm_sacrifices_a_land_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(225, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[glacial_chasm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forest_obj = on_battlefield(&engine, p0, forest()).expect("Forest deployed");
    let chasm = play_land(&mut engine, p0, glacial_chasm());
    assert!(!entered_tapped(&engine, chasm));

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "unexpected while waiting for trigger: {:?}",
                engine.pending()
            );
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }

    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected land sacrifice choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert!(options.contains(&forest_obj));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![forest_obj],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert!(on_battlefield(&engine, p0, glacial_chasm()).is_some());
}

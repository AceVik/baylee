//! `cards/lands/check/agna_qel_a.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Agna Qel'a: "This land enters tapped unless you control a basic land." / "{T}: Add {U}." / "{2}{U}, {T}: Draw a card, then discard a card."
/// Controlling a basic Island allows Agna Qel'a to enter untapped from hand.
/// Paying {2}{U} with the three Islands activates its looting ability, drawing a card and asking for a card to discard.
#[test]
fn agna_qel_a_enters_untapped_with_basic_and_loots() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(124, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[agna_qel_a()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let agna = play_land(&mut engine, p0, agna_qel_a());
    assert!(
        !entered_tapped(&engine, agna),
        "enters untapped with basic Island"
    );

    tap_mana_except(&mut engine, p0, agna);
    activate(&mut engine, p0, agna_qel_a(), 1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected discard choice");
    };

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, agna));
}

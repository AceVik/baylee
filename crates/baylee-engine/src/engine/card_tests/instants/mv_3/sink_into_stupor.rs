//! `cards/instants/mv_3/sink_into_stupor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sink into Stupor // Soporific Springs (`Coverage::Implemented`): "Return
/// target spell or nonland permanent an opponent controls to its owner's hand.
/// // As this land enters, you may pay 3 life. If you don't, it enters tapped.
/// {T}: Add {U}."
///
/// The front face returns a nonland permanent an opponent controls to hand.
/// The test casts Sink into Stupor targeting the opponent's Llanowar Elves,
/// confirms that an opponent land is not offered as a valid target, and
/// verifies the bounced creature returns to the opponent's hand.
#[test]
fn sink_into_stupor_returns_opponent_nonland_permanent_to_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(46, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[sink_into_stupor()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent's Elf");
    let opponent_land = on_battlefield(&engine, p1, forest()).expect("opponent's Forest");

    cast_from_hand(&mut engine, p0, sink_into_stupor());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "target nonland permanent an opponent controls — the Elf qualifies"
    );
    assert!(
        !options.contains(&opponent_land),
        "an opponent's land is not a nonland permanent"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the bounced creature is no longer on the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "the bounced creature was returned to its owner's hand"
    );
    assert!(
        in_graveyard(&engine, p0, sink_into_stupor()).is_some(),
        "resolved spell moves to the graveyard"
    );
}

//! `cards/lands/utility/memorial_to_folly.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Memorial to Folly: "This land enters tapped." / "{T}: Add {B}." / "{2}{B}, {T}, Sacrifice this land: Return target creature card from your graveyard to your hand."
/// Under `Coverage::Implemented`, Memorial to Folly enters tapped when played from hand.
/// Activating ability 1 sacrifices the land and returns a target creature card from the graveyard to the hand.
#[test]
fn memorial_to_folly_returns_creature_from_graveyard_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(130, quiet_creature())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[memorial_to_folly()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, memorial_to_folly());
    assert!(entered_tapped(&engine, land));

    seed_graveyard(&mut engine, p0, 1);
    let elf = in_graveyard(&engine, p0, quiet_creature()).expect("creature in graveyard");

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, memorial_to_folly(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    assert!(in_graveyard(&engine, p0, memorial_to_folly()).is_some());

    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, quiet_creature()).is_some());
}

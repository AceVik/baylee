//! `cards/creatures/mv_3/necratog.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, NECRATOG};

/// `Necratog` ({1}{B}{B}, Creature — Atog 1/2, Partial) exiles a creature card
/// from your graveyard to get +2/+2 until end of turn.
#[test]
fn necratog_exiles_creature_from_graveyard_for_pt_boost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[NECRATOG])
        .hand(0, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let necratog = on_battlefield(&engine, p0, NECRATOG).unwrap();
    let bears = hand_to_graveyard(&mut engine, p0, GRIZZLY_BEARS);
    activate(&mut engine, p0, NECRATOG, 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, necratog), (3, 4));
    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_none());
}

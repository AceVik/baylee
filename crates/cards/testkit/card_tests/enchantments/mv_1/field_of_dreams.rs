//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn field_of_dreams_reveals_both_players_only_while_present() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::FIELD_OF_DREAMS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let source = on_battlefield(&e, p0, index::FIELD_OF_DREAMS).unwrap();
    for player in [p0, PlayerId::new(1)] {
        assert!(e.state().library_top_revealed(player));
    }
    e.dev_state_mut(p0).unwrap().phase_out(&[source]);
    for player in [p0, PlayerId::new(1)] {
        assert!(!e.state().library_top_revealed(player));
    }
    e.dev_state_mut(p0).unwrap().phase_in(source);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    for player in [p0, PlayerId::new(1)] {
        assert!(e.state().library_top_revealed(player));
    }
    bury(&mut e, &[source]);
    for player in [p0, PlayerId::new(1)] {
        assert!(!e.state().library_top_revealed(player));
    }
}

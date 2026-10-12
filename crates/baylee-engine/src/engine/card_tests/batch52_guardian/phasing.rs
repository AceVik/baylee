//! Child module of `batch52_guardian`; transparent between-instruction fixture.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn phased_out_guardian_stops_guarding_before_the_next_static_sweep() {
    let mut e = prepared(
        &[index::POWER_ARTIFACT],
        &[index::GUARDIAN_BEAST, index::SOL_RING],
        &[],
    );
    let beast = obj(&e, P0, index::GUARDIAN_BEAST);
    let ring = obj(&e, P0, index::SOL_RING);
    let aura = in_hand(&e, P0, index::POWER_ARTIFACT).unwrap();
    assert!(!crate::permanent_guards::can_gain_control(
        &e.state, ring, P1
    ));
    assert!(!crate::permanent_guards::permits_new_enchantment(
        &e.state, ring, aura
    ));

    // Deliberately stop between resolving instructions: phase_out is the real
    // instruction's state operation, with no Engine/static sweep afterward.
    // The registered physical Guardian and its old static records remain.
    e.state.phase_out(&[beast]);
    assert!(
        e.state
            .object(beast)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );
    assert!(
        crate::permanent_guards::can_gain_control(&e.state, ring, P1),
        "a phased-out source cannot prohibit the next control instruction"
    );
    assert!(
        crate::permanent_guards::permits_new_enchantment(&e.state, ring, aura),
        "a phased-out source cannot prohibit the next Aura instruction"
    );
}

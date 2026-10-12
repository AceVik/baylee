#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn phase_out_uses_one_controlled_set_and_direct_aura_can_leave_opponents_host_present() {
    let mut e = prepared(
        &[index::HOLY_STRENGTH, index::TEFERI_S_PROTECTION],
        &[index::GRIZZLY_BEARS, index::SOL_RING],
        &[index::GRIZZLY_BEARS],
    );
    let theirs = obj(&e, P1, index::GRIZZLY_BEARS);
    let mine = obj(&e, P0, index::GRIZZLY_BEARS);
    cast_target(&mut e, index::HOLY_STRENGTH, theirs);
    let aura = obj(&e, P0, index::HOLY_STRENGTH);
    let incarnation = e.state.object(aura).unwrap().version;
    let sol = obj(&e, P0, index::SOL_RING);
    cast(&mut e, P0, index::TEFERI_S_PROTECTION);
    pass_until(&mut e, stack_is_empty);
    for id in [aura, mine, sol] {
        assert!(
            e.state
                .object(id)
                .unwrap()
                .status
                .contains(crate::object::Status::PHASED_OUT)
        );
    }
    assert!(
        !e.state
            .object(theirs)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );
    next_main(&mut e, P1);
    assert!(
        e.state
            .object(aura)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT),
        "opponent's turn does not end the duration or phase in your Aura"
    );
    next_main(&mut e, P0);
    assert_eq!(e.state.object(aura).unwrap().version, incarnation);
    assert_eq!(e.state.object(aura).unwrap().attached_to, Some(theirs));
    assert!(
        !e.state
            .object(aura)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );
}

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn magosi_mandatory_skip_coexists_with_accepting_or_declining_time_vault() {
    for accept in [false, true] {
        let mut e = prepared(&[], &[index::MAGOSI_THE_WATERVEIL, index::TIME_VAULT], &[]);
        let magosi = on_battlefield(&e, P0, index::MAGOSI_THE_WATERVEIL).unwrap();
        e.state.set_tapped(magosi, false);
        magosi_skip(&mut e);
        let vault = on_battlefield(&e, P0, index::TIME_VAULT).unwrap();
        e.state.set_tapped(vault, true);
        e.refresh_offer();
        let mut yes = accept;
        assert_eq!(next_actual_main(&mut e, &mut yes), P1);
        assert_eq!(
            next_actual_main(&mut e, &mut yes),
            P1,
            "one P0 occurrence was skipped"
        );
        if accept {
            assert!(!is_tapped(&e, vault));
            assert_eq!(
                next_actual_main(&mut e, &mut yes),
                P1,
                "Vault replacement cannot consume mandatory Magosi skip too"
            );
        } else {
            assert!(is_tapped(&e, vault), "declining Vault does not untap it");
        }
        assert_eq!(
            next_actual_main(&mut e, &mut yes),
            P0,
            "all owed skips consumed exactly once"
        );
    }
}

#[test]
fn two_magosi_skips_consume_two_queued_extra_turns_then_normal_order_continues() {
    let mut e = prepared(
        &[index::TWIDDLE, index::TIME_WALK, index::TIME_WALK],
        &[index::MAGOSI_THE_WATERVEIL],
        &[],
    );
    let magosi = on_battlefield(&e, P0, index::MAGOSI_THE_WATERVEIL).unwrap();
    e.state.set_tapped(magosi, false);
    magosi_skip(&mut e);
    turn_cast(&mut e, P0, index::TWIDDLE);
    targets(&mut e, vec![magosi]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);
    magosi_skip(&mut e);
    turn_walk(&mut e);
    turn_walk(&mut e);
    let mut no = false;
    assert_eq!(
        next_actual_main(&mut e, &mut no),
        P1,
        "both extra P0 turns skipped independently"
    );
    assert_eq!(
        next_actual_main(&mut e, &mut no),
        P0,
        "normal P0 turn no longer skipped"
    );
}

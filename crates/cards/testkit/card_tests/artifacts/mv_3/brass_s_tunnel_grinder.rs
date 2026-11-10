//! `cards/artifacts/mv_3/brass_s_tunnel_grinder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Brass's Tunnel-Grinder` // `Tecutlan, the Searing Rift` (`Coverage::Partial`):
/// "When `Brass's Tunnel-Grinder` enters, discard any number of cards, then draw that many
/// cards plus one. At the beginning of your end step, if you descended this turn, put a
/// bore counter on `Brass's Tunnel-Grinder`. Then if there are three or more bore counters
/// on it, remove those counters and transform it. // `{{T}}`: Add `{{R}}`. Whenever you cast a
/// permanent spell using mana produced by `Tecutlan`, discover X, where X is that spell's mana value."
///
/// Entry chooses zero discards and still draws one card. The front remains a
/// legendary artifact without activated abilities; Batch48 exercises its full
/// transform/provenance clauses separately.
#[test]
fn brass_s_tunnel_grinder_empty_discard_draws_one_and_front_has_no_activated_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(301, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[brass_s_tunnel_grinder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lib_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, brass_s_tunnel_grinder());
    pass_until(&mut engine, stack_is_empty);

    let grinder = on_battlefield(&engine, p0, brass_s_tunnel_grinder())
        .expect("Brass's Tunnel-Grinder resolved to the battlefield");
    assert_eq!(
        engine.state().object(grinder).map(|o| o.face_index),
        Some(0),
        "grinder is on face 0"
    );

    let t = types(&engine, grinder);
    assert!(t.contains(TypeSet::ARTIFACT), "grinder is an artifact");
    assert!(!t.contains(TypeSet::LAND), "grinder is not a land");
    assert!(
        engine
            .state()
            .object(grinder)
            .expect("grinder exists")
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY),
        "grinder is legendary"
    );

    assert_eq!(
        library_size(&engine, p0),
        lib_before - 1,
        "zero discarded cards still draws one"
    );
    assert!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len() == 1,
        "the entry trigger draws exactly one with empty discard"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two remaining mountains floated mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == grinder),
        "grinder offers no activated abilities on face 0 with mana floating"
    );
}

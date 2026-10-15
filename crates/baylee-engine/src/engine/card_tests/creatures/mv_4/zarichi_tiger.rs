//! Oracle checklist: pay {1}{W} and {T}; gain exactly 2 life; tap prevents repeated use in the same turn.
#[allow(clippy::wildcard_imports)]
use super::*;
fn zarichi_tiger() -> CardIndex {
    card_index("0749e79f-e61d-4037-bb39-75f1ebf46c5d")
}

#[test]
fn tiger_pays_one_white_and_one_generic_to_gain_two_life() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[zarichi_tiger(), plains(), plains()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let tiger = on_battlefield(&e, p0, zarichi_tiger()).unwrap();
    tap_all_mana(&mut e, p0);
    let life = e.state().players[0].life;
    activate(&mut e, p0, zarichi_tiger(), 0);
    assert!(is_tapped(&e, tiger));
    assert_eq!(
        e.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{W}} is paid"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, life + 2);
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == tiger),
        "the tap cost prevents a second use"
    );
}

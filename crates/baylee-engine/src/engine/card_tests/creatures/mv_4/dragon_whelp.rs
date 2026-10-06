//! `cards/creatures/mv_4/dragon_whelp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dragon Whelp — "Flying. {R}: This creature gets +1/+0 until end of turn.
/// If this ability has been activated four or more times this turn,
/// sacrifice this creature at the beginning of the next end step." Four
/// activations make it a 6/3 flier for the turn, and it is still there as
/// the end step begins; the delayed trigger sacrifices it from the stack.
#[test]
fn dragon_whelp_pumps_and_is_sacrificed_after_four_activations() {
    let p0 = PlayerId::new(0);
    let whelp_card = card_index("705a1985-ed39-4a4b-812e-a677170b596e");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[whelp_card, mountain(), mountain(), mountain(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let whelp = on_battlefield(&engine, p0, whelp_card).expect("the Whelp is out");
    assert_eq!(pt(&engine, whelp), (2, 3));
    assert!(keywords(&engine, whelp).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    for _ in 0..4 {
        activate(&mut engine, p0, whelp_card, 0);
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, whelp), (6, 3), "+1/+0 four times");

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    assert_eq!(
        engine.state().object(whelp).map(|o| o.zone),
        Some(Zone::Battlefield),
        "nothing happens before the trigger resolves"
    );
    assert!(!stack_is_empty(&engine), "the sacrifice is on the stack");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(whelp).map(|o| o.zone),
        Some(Zone::Graveyard),
        "sacrificed at the beginning of the end step"
    );
}

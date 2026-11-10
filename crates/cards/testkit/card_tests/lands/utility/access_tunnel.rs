//! `cards/lands/utility/access_tunnel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Access Tunnel: "{T}: Add {C}." / "{3}, {T}: Target creature with power 3
/// or less can't be blocked this turn."
///
/// The size restriction is on the **target** (CR 115.3), so the 6/6 across
/// the board is never in the menu rather than being refused after being
/// named — and it is the bystander that makes the menu mean something: a
/// list holding only the Elves could otherwise be a board with only one
/// creature on it.
#[test]
fn access_tunnel_makes_a_small_creature_unblockable_and_is_offered_no_large_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(122, forest())
        .battlefield(
            0,
            &[
                access_tunnel(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                rootbreaker_wurm(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tunnel = on_battlefield(&engine, p0, access_tunnel()).expect("Access Tunnel deployed");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the 1/1 is seated");
    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the 6/6 is seated");

    tap_mana_except(&mut engine, p0, tunnel);
    activate(&mut engine, p0, access_tunnel(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![elf],
        "\"power 3 or less\" — the 6/6 is not a legal target"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf came out of the menu");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elf).contains(KeywordSet::UNBLOCKABLE),
        "the creature the ability named can't be blocked this turn"
    );
    assert!(
        !keywords(&engine, wurm).contains(KeywordSet::UNBLOCKABLE),
        "and the one it could not name is untouched"
    );
    assert!(is_tapped(&engine, tunnel));
}

//! `cards/instants/mv_1/revitalizing_repast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Revitalizing Repast, front face: a counter **and** indestructible, which
/// only a destruction can tell apart from the counter alone.
///
/// The card is `Coverage::Implemented` and the second clause is the one a
/// test that stopped at the counter would never read — so the Elf is shot at
/// after it is pumped, and survives.
#[test]
fn revitalizing_repast_leaves_its_target_indestructible() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(414, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .hand(0, &[revitalizing_repast()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(1, &[hero_s_downfall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, revitalizing_repast());
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "the counter is on"
    );

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, hero_s_downfall());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the Elf is a legal target");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the removal resolves"
    );

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "\"It gains indestructible until end of turn\" — Destroy does nothing \
         (CR 702.12b), and a card that had only put the counter on would have \
         lost the Elf here"
    );
}

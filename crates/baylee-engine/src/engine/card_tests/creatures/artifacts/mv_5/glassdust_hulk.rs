//! `cards/creatures/artifacts/mv_5/glassdust_hulk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn glassdust_hulk() -> CardIndex {
    card_index("a1379c25-6daf-4efa-a517-6e6e25c31875")
}

/// Glassdust Hulk — "Whenever another artifact you control enters, this
/// creature gets +1/+1 until end of turn and can't be blocked this turn."
#[test]
fn glassdust_hulk_grows_and_evades_when_another_artifact_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[sol_ring()])
        .battlefield(0, &[glassdust_hulk(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hulk = on_battlefield(&engine, p0, glassdust_hulk()).expect("seated");
    assert_eq!(pt(&engine, hulk), (3, 4));
    assert!(!keywords(&engine, hulk).contains(KeywordSet::UNBLOCKABLE));

    cast_from_hand(&mut engine, p0, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, hulk), (4, 5), "+1/+1");
    assert!(
        keywords(&engine, hulk).contains(KeywordSet::UNBLOCKABLE),
        "can't be blocked"
    );

    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    assert_eq!(pt(&engine, hulk), (3, 4), "until end of turn");
    assert!(!keywords(&engine, hulk).contains(KeywordSet::UNBLOCKABLE));
}

/// "Another": a second Hulk entering triggers the first, not itself.
#[test]
fn glassdust_hulk_does_not_trigger_on_its_own_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[glassdust_hulk()])
        .battlefield(
            0,
            &[
                glassdust_hulk(),
                plains(),
                island(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, glassdust_hulk());
    pass_until(&mut engine, stack_is_empty);
    let mut bodies: Vec<_> = all_on_battlefield(&engine, p0, glassdust_hulk())
        .into_iter()
        .map(|h| pt(&engine, h))
        .collect();
    bodies.sort_unstable();
    assert_eq!(bodies, vec![(3, 4), (4, 5)], "only the first Hulk grew");
}

/// "Cycling {W/U}": discarding it from hand for one white (or blue) mana
/// draws a card.
#[test]
fn glassdust_hulk_cycles_for_a_hybrid_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[glassdust_hulk()])
        .battlefield(0, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 1);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, glassdust_hulk(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, glassdust_hulk()).is_some(),
        "discarded"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "a card was drawn in its place"
    );
    assert!(in_hand(&engine, p0, glassdust_hulk()).is_none());
}

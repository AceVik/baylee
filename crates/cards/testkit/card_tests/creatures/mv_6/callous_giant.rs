//! `cards/creatures/mv_6/callous_giant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "7a2336ff-c8ed-4daa-b182-403c21bc8133"
fn callous_giant() -> CardIndex {
    card_index("7a2336ff-c8ed-4daa-b182-403c21bc8133")
}

/// Three damage is at the printed threshold, so the whole event is prevented:
/// no damage is marked and no `DamageDealt` result reaches the journal.
#[test]
fn callous_giant_prevents_a_lightning_bolt_without_a_damage_result() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[callous_giant(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p0, callous_giant()).expect("the Giant is seated");
    assert_eq!(pt(&engine, giant), (4, 4));
    let journal_start = engine.journal().entries().len();

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(giant).unwrap().damage, 0);
    assert!(
        damage_events(&engine, journal_start).is_empty(),
        "prevented damage produces no dealt-damage result"
    );
    assert!(on_battlefield(&engine, p0, callous_giant()).is_some());
}

/// Four damage exceeds the threshold. It is neither capped to three nor
/// reduced to one: all four are dealt, which is lethal to the printed 4/4.
#[test]
fn callous_giant_takes_all_four_lightning_blast_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                callous_giant(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_blast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p0, callous_giant()).expect("the Giant is seated");
    let journal_start = engine.journal().entries().len();

    cast_from_hand(&mut engine, p0, lightning_blast());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    let events = damage_events(&engine, journal_start);
    assert_eq!(events.len(), 1, "the Blast has one damage result");
    assert_eq!(
        events[0].1,
        baylee_engine::event::DamageTarget::Object(giant)
    );
    assert_eq!(events[0].2, 4, "nothing is prevented above the threshold");
    assert!(!events[0].3, "Lightning Blast is noncombat damage");
    assert!(
        in_graveyard(&engine, p0, callous_giant()).is_some(),
        "four marked damage destroys the 4/4 Giant"
    );
}

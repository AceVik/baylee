//! `cards/lands/utility/detection_tower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Detection Tower prints `{{T}}: Add {{C}}.` and `{{1}}, {{T}}: Until end of turn, your
/// opponents and creatures your opponents control with hexproof can be the targets of spells
/// and abilities you control as though they didn't have hexproof.`
///
/// Under `Coverage::Partial`, the creature half is implemented by stripping hexproof from
/// opponent creatures outright until end of turn, while targeting opponents themselves is omitted.
/// With an opponent controlling `sylvan_caryatid()` (which has hexproof), activating ability index 1
/// for `{{1}}, {{T}}` off a `forest()` removes `KeywordSet::HEXPROOF` from the opponent's creature.
#[test]
fn detection_tower_removes_hexproof_from_opponent_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[detection_tower(), forest()])
        .battlefield(1, &[sylvan_caryatid()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tower = on_battlefield(&engine, p0, detection_tower()).expect("tower on battlefield");
    let caryatid = on_battlefield(&engine, p1, sylvan_caryatid()).expect("caryatid on battlefield");
    assert!(keywords(&engine, caryatid).contains(KeywordSet::HEXPROOF));

    // Float {1} green mana from the Forest while keeping Detection Tower untapped.
    tap_mana_except(&mut engine, p0, tower);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(!is_tapped(&engine, tower));

    activate(&mut engine, p0, detection_tower(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(!keywords(&engine, caryatid).contains(KeywordSet::HEXPROOF));
    assert!(is_tapped(&engine, tower));
}

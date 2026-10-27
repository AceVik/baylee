//! Clockwork Vorrac — `{5}` 0/0 Artifact Creature — Boar Beast (MRD).
//! Oracle: "Trample. This creature enters with four +1/+1 counters on it. Whenever this creature attacks or blocks, remove a +1/+1 counter from it at end of combat. {T}: Put a +1/+1 counter on this creature."

use super::*;

fn clockwork_vorrac() -> CardIndex {
    card_index("e1a64bba-eaff-4f59-b85d-7df0b13e2e6b")
}

#[test]
fn clockwork_vorrac_enters_with_four_counters_taps_to_add_one_and_loses_one_after_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let vorrac = clockwork_vorrac();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[vorrac])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vorrac_id = cast_resolved(&mut engine, p0, vorrac);
    assert_eq!(pt(&engine, vorrac_id), (4, 4));
    assert!(keywords(&engine, vorrac_id).contains(KeywordSet::TRAMPLE));

    // Next turn: tap to add a fifth +1/+1 counter.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    activate(&mut engine, p0, vorrac, 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, vorrac_id), (5, 5));

    // Following turn: attack and lose one counter at end of combat.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    attack_alone(&mut engine, p0, vorrac_id, p1);
    through_combat(&mut engine);
    assert_eq!(pt(&engine, vorrac_id), (4, 4));
}

//! Goblin Dirigible — `{6}` 4/4 Artifact Creature — Construct (MRD).
//! Oracle: "Flying. This creature doesn't untap during your untap step. At the beginning of your upkeep, you may pay {4}. If you do, untap this creature."

use super::*;

fn goblin_dirigible() -> CardIndex {
    card_index("8e69b322-4e66-4d5b-ab46-f002c40a913f")
}

#[test]
fn goblin_dirigible_flies_stays_tapped_and_untaps_on_upkeep_when_paid_four() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let dirigible = goblin_dirigible();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dirigible, forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);

    // Decline turn 1 upkeep untap trigger before reaching main phase.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);

    let dirigible_id = on_battlefield(&engine, p0, dirigible).unwrap();
    assert!(keywords(&engine, dirigible_id).contains(KeywordSet::FLYING));
    attack_alone(&mut engine, p0, dirigible_id, p1);
    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, dirigible_id));

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(is_tapped(&engine, dirigible_id));
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana_but(&mut engine, p0, None);
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, dirigible_id));
}

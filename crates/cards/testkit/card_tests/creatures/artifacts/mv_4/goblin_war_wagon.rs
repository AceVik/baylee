//! Goblin War Wagon — `{4}` 3/3 Artifact Creature — Juggernaut (MRD).
//! Oracle: "This creature doesn't untap during your untap step. At the beginning of your upkeep, you may pay {2}. If you do, untap this creature."

use super::*;

fn goblin_war_wagon() -> CardIndex {
    card_index("81a38340-9558-49d3-8de6-d7d01babfd97")
}

#[test]
fn goblin_war_wagon_stays_tapped_at_untap_step_and_untaps_on_upkeep_when_paid_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let wagon = goblin_war_wagon();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wagon, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);

    // Decline turn 1 upkeep untap trigger before reaching main phase.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);

    let wagon_id = on_battlefield(&engine, p0, wagon).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wagon_id, Defender::Player(p1))],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, wagon_id));

    // Advance to p0's upkeep: Goblin War Wagon is still tapped until p0 pays {2}.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(is_tapped(&engine, wagon_id));
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana_but(&mut engine, p0, None);
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, wagon_id));
}

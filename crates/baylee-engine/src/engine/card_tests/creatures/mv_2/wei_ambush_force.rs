//! `cards/creatures/mv_2/wei_ambush_force.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wei Ambush Force is a printed 1/1 for `{1}{B}` whose entire text is
/// "Whenever this creature attacks, it gets +2/+0 until end of turn." The
/// card *is* the gap between the body standing on the battlefield and the
/// body after the attack declaration, so the same permanent is read three
/// times: a printed `(1, 1)` before it has ever attacked, `(3, 1)` once it
/// has been declared an attacker and its trigger has resolved, and `(1, 1)`
/// again on the following turn — the last reading being the only thing
/// "until end of turn" can mean, since a permanent pump would still be a
/// 3/1 there. Nothing grants it haste, so the attack the trigger is about is
/// only legal after a whole turn cycle (CR 302.6).
#[test]
fn wei_ambush_force_pumps_itself_for_attacking_and_only_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[wei_ambush_force()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, wei_ambush_force());
    pass_until(&mut engine, stack_is_empty);
    let soldier = on_battlefield(&engine, p0, wei_ambush_force()).expect("the Soldier resolved");
    assert_eq!(
        pt(&engine, soldier),
        (1, 1),
        "the printed body, on a creature that has not attacked"
    );

    // A whole turn cycle: nothing gives it haste, so the declaration below is
    // the first one that may legally name the creature (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&soldier),
        "an untapped 1/1 past summoning sickness may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(soldier, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, soldier),
        (3, 1),
        "\"Whenever this creature attacks, it gets +2/+0\""
    );

    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, soldier),
        (1, 1),
        "and the printed duration is \"until end of turn\": the next turn's \
         main phase is past the cleanup step that ends it (CR 514.2)"
    );
}

//! `cards/creatures/mv_2/lurking_nightstalker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lurking Nightstalker prints a single line: "Whenever this creature
/// attacks, it gets +2/+0 until end of turn." The filter is `Filter::This`,
/// so a second Nightstalker stands idle next to it and must not pump itself
/// — only this way can the two readings "the attacker" and "each
/// Nightstalker" be told apart. That p1 takes three instead of one combat
/// damage shows that the +2/+0 applied when attacking and were not merely
/// projected, and the controller's own main phase in the next turn reads the
/// second half of the sentence: "until end of turn" is not a lasting type.
#[test]
fn lurking_nightstalker_pumps_itself_when_it_attacks_and_not_its_idle_twin() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(1837, swamp())
        .battlefield(0, &[lurking_nightstalker(), lurking_nightstalker()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stalkers = all_on_battlefield(&engine, p0, lurking_nightstalker());
    assert_eq!(
        stalkers.len(),
        2,
        "two Nightstalkers, one of them stays home"
    );
    let (attacker, bystander) = (stalkers[0], stalkers[1]);
    assert_eq!(pt(&engine, attacker), (1, 1), "die gedruckte 1/1");
    assert_eq!(pt(&engine, bystander), (1, 1), "die gedruckte 1/1");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until hält nur an der Angriffserklärung")
    };
    assert!(
        attackers.contains(&attacker),
        "an untapped 1/1 may attack, even though its text prints nothing but \
         a trigger: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Der Angriffs-Trigger geht mit der Erklärung auf den Stapel (CR 603.2)
    // und löst auf, lange bevor irgendwer Schaden nimmt.
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        pt(&engine, attacker),
        (3, 1),
        "+2/+0 für den, der angegriffen hat"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "`Filter::This` pumps the attacker and not every Nightstalker in the game"
    );

    // Three combat damage instead of one: the +2/+0 applied when it
    // struck and was not a mere projection.
    pass_until(&mut engine, |e| e.state().players[1].life < 20);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "drei Kampfschaden von der gepumpten 1/1"
    );

    // "until end of turn" is the second half of the line: next turn
    // the pump is gone, without anyone having to remove it.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, attacker),
        (1, 1),
        "the pump ends with the turn and not with the attack"
    );
}

//! `cards/creatures/mv_2/veteran_cavalier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Veteran Cavalier costs {W}{W}, arrives as a 2/2 Human Knight, and its
/// entire printed text is `Vigilance`: "Attacking doesn't cause this
/// creature to tap" (CR 702.20). That is only readable in combat — the
/// creature is still standing after the attack declaration, and next to it
/// stands a Llanowar Elves without the keyword, which the very same attack
/// does tap. That the combat damage arrives at the opponent (two plus one)
/// proves that the combat step really took place and not merely a board
/// that missed it.
#[test]
fn veteran_cavalier_attacks_without_tapping_while_the_elf_beside_it_does() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4411, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .hand(0, &[veteran_cavalier()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {W}{W} from the two Plains, and the Elf explicitly remains standing:
    // it is the control in combat below, and a creature tapped for mana
    // may not attack anymore.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Plains, and two white mana"
    );
    cast_with_floating(&mut engine, p0, veteran_cavalier());
    pass_until(&mut engine, stack_is_empty);
    let cavalier =
        on_battlefield(&engine, p0, veteran_cavalier()).expect("der Cavalier ist gelandet");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("der Elf steht noch");
    assert_eq!(pt(&engine, cavalier), (2, 2), "der gedruckte Körper");
    assert!(
        keywords(&engine, cavalier).contains(KeywordSet::VIGILANCE),
        "das gedruckte Keyword erreicht das Permanent"
    );

    // One turn later: if cast this turn, the Cavalier would still be
    // summoning sick in its own combat step (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, cavalier) && !is_tapped(&engine, elves),
        "der Enttappschritt hat beide aufgestellt"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until hält nur bei der Angriffserklärung an")
    };
    assert_eq!(player, p0, "the active player declares their attackers");
    assert!(
        attackers.contains(&cavalier) && attackers.contains(&elves),
        "both are untapped and not summoning sick: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (cavalier, Defender::Player(p1)),
                    (elves, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    assert!(
        !is_tapped(&engine, cavalier),
        "Vigilance: the attack did not tap the Cavalier"
    );
    assert!(
        is_tapped(&engine, elves),
        "und der Elf daneben, ohne das Keyword, ist für seinen Angriff \
         getappt — ohne ihn läse der Test oben nur ein Brett, auf dem gar \
         nichts angreift"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "zwei Schaden vom Cavalier und einer vom Elf: der Angriff ist wirklich \
         durchgekommen"
    );
}

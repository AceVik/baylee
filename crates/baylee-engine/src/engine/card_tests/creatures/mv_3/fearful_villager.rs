//! `cards/creatures/mv_3/fearful_villager.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fearful Villager — {2}{R}, a 2/3 Human Werewolf whose printed keywords are
/// menace and daybound.
///
/// Menace is a restriction on the *declaration* and nowhere on the board, so
/// the test reads it in the only place it exists: with the Villager attacking,
/// the lone untapped Elf across the table is refused as its blocker, while that
/// same Elf is accepted as the lone blocker of the Elf attacking beside it. The
/// second block is the control that separates "this attacker may not be blocked
/// alone" from "this engine declines every block", and the walk to the end step
/// afterwards is what says the refused answer left the attack standing rather
/// than dropping the Villager out of combat.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn fearful_villagers_menace_refuses_the_lone_blocker_that_the_attack_beside_it_takes() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[fearful_villager(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let villager = on_battlefield(&engine, p0, fearful_villager()).expect("the Villager is out");
    let plain = on_battlefield(&engine, p0, llanowar_elves()).expect("the second attacker is out");
    let blocker = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    assert_eq!(
        pt(&engine, villager),
        (2, 3),
        "the printed 2/3 body of the day face"
    );
    assert!(
        keywords(&engine, villager).contains(KeywordSet::MENACE),
        "menace is the front face's second printed keyword, beside daybound: {:?}",
        keywords(&engine, villager)
    );
    assert!(
        !keywords(&engine, plain).contains(KeywordSet::MENACE),
        "and the Elf attacking beside it prints no evasion at all, which is what it is here for"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on the attack declaration")
    };
    assert_eq!(player, p0, "the active seat declares its attackers");
    assert!(
        attackers.contains(&villager) && attackers.contains(&plain),
        "neither arrived this turn, so both may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (villager, Defender::Player(p1)),
                    (plain, Defender::Player(p1)),
                ],
            },
        )
        .expect("both attackers came out of the list that offered them");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on the declare-blockers question")
    };
    assert_eq!(player, p1, "the defending seat is the one asked");
    assert!(
        blockers.iter().any(|o| o.blocker == blocker),
        "the Elf is offered as a blocker at all, so the refusal below is about \
         the attacker it was aimed at and not about a creature the engine never \
         considered: {blockers:?}"
    );

    let refused = engine.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, villager)],
        },
    );
    assert!(
        refused.is_err(),
        "\"This creature can't be blocked except by two or more creatures\": \
         one Elf alone is no legal block on the Villager: {refused:?}"
    );

    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, plain)],
            },
        )
        .expect(
            "the control: the very same lone-blocker declaration is legal \
             against the attacker that prints no menace",
        );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the Villager was never blocked, so its 2 power reached the defending \
         seat: the refused answer left the attack standing instead of removing \
         the creature from combat"
    );
    assert!(
        on_battlefield(&engine, p0, fearful_villager()).is_some(),
        "and the Villager is still the permanent that attacked"
    );
}

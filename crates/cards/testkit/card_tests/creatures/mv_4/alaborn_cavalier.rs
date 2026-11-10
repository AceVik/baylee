//! `cards/creatures/mv_4/alaborn_cavalier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Alaborn Cavalier is a `{2}{W}{W}` 2/2 with one printed line: "Whenever
/// this creature attacks, you may tap target creature." Both halves of that
/// sentence need a witness on the board. The trigger has to come off a real
/// attack declaration — so the Cavalier is *cast* and then waits a turn for
/// CR 302.6 rather than being seated mid-combat — and the tap has to land on
/// the creature the question named: two Elves stand across the table, so one
/// of them tapped while its twin stays standing is what tells a target from a
/// board-wide sweep. The "you may" is answered rather than assumed, because a
/// `MayDo` nobody is asked resolves into nothing.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn alaborn_cavalier_taps_the_creature_its_attack_trigger_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[alaborn_cavalier()])
        // Two Elves: the victim and the control that says only one was named.
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its first main phase"
    );

    // Four Plains pay {2}{W}{W}: the card arrives the way the card arrives.
    cast_from_hand(&mut engine, p0, alaborn_cavalier());
    pass_until(&mut engine, stack_is_empty);
    let cavalier = on_battlefield(&engine, p0, alaborn_cavalier()).expect("the Cavalier resolved");
    assert_eq!(pt(&engine, cavalier), (2, 2), "the printed body");

    // CR 302.6: a creature that arrived this turn cannot attack, so the
    // combat this test reads is the Cavalier's next one.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p1, quiet_creature());
    assert_eq!(elves.len(), 2, "two Elves across the table");
    let (victim, bystander) = (elves[0], elves[1]);
    assert!(
        !is_tapped(&engine, victim) && !is_tapped(&engine, bystander),
        "nothing has tapped either of them yet"
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
    assert_eq!(player, p0, "the Cavalier's controller declares");
    assert!(
        attackers.contains(&cavalier),
        "untapped and past its summoning sickness, the Cavalier may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(cavalier, Defender::Player(p1))],
            },
        )
        .expect("a legal attacker against a surviving opponent");

    // The trigger's two questions are one "you may" and one target, and the
    // engine may ask them in either order: the target is chosen as the
    // ability goes on the stack (CR 603.3d), the yes/no when it resolves.
    let mut saw_may = false;
    let mut saw_targets = false;
    for _ in 0..30 {
        if saw_targets && stack_is_empty(&engine) && is_tapped(&engine, victim) {
            break;
        }
        match engine.pending().clone() {
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            } => {
                saw_may = true;
                engine
                    .apply(player, PlayerAction::YesNo(true))
                    .expect("the offered answer");
            }
            Pending::ChooseTargets {
                player, options, ..
            } => {
                saw_targets = true;
                assert_eq!(player, p0, "the attacking seat aims it");
                assert!(
                    options.contains(&victim) && options.contains(&bystander),
                    "\"target creature\" is any creature, on either side of the \
                     table: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![victim],
                        },
                    )
                    .expect("a creature the question offered");
            }
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("priority passes");
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .expect("no blockers are declared");
            }
            // A fresh combat step means this one has been left behind, and
            // the assertions below say what did not happen.
            Pending::ChooseAttackers { .. } => break,
            other => panic!("unexpected while the attack trigger resolves: {other:?}"),
        }
    }

    assert!(
        saw_may,
        "\"you may tap target creature\" is a question, and an attack that \
         never asks it is a trigger that never fired"
    );
    assert!(
        saw_targets,
        "the ability targets a creature, so it has to be offered one"
    );
    assert!(
        is_tapped(&engine, victim),
        "the Elf the trigger named is the one it tapped"
    );
    assert!(
        !is_tapped(&engine, bystander),
        "and a trigger that tapped the board rather than its target would have \
         taken this one down with it"
    );
}

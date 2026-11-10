//! `cards/creatures/mv_4/flanking_troops.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flanking Troops — {2}{W}{W} 2/2 Human Soldier: "Whenever this creature
/// attacks, you may tap target creature."
///
/// The whole card is one trigger and it is a *question* as much as an effect,
/// so both halves are played: the first attack answers the printed "you may"
/// with no — the only reading that tells an optional tap from a forced one —
/// and the second says yes and names an Elf across the table. An Elf of mine
/// and a second Elf behind that one stand on the same board, because "target
/// creature" names no side of the battlefield and no seat: the tap has to
/// reach the creature that was named and neither of the two that were not.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn flanking_troops_asks_before_it_taps_and_reaches_only_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .hand(0, &[flanking_troops()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The card arrives the way the card arrives: {2}{W}{W} off an open board.
    // Nothing can attack the turn it entered (CR 302.6), so the two attacks
    // below are taken on the Soldier's next turns, which is also what puts
    // every Elf on this board back on its feet before the first of them.
    cast_from_hand(&mut engine, p0, flanking_troops());
    pass_until(&mut engine, stack_is_empty);
    let troops = on_battlefield(&engine, p0, flanking_troops()).expect("the Soldier resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(theirs.len(), 2, "two Elves across the table");
    let (victim, bystander) = (theirs[0], theirs[1]);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let mut asked = 0;
    let mut menu: Vec<ObjectId> = Vec::new();
    for attack in 0..2 {
        if attack == 1 {
            reach_their_main_phase(&mut engine, p1);
            reach_their_main_phase(&mut engine, p0);
            assert!(
                !is_tapped(&engine, victim),
                "the untap step stood the Elf the first attack declined back up"
            );
        }

        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
            unreachable!("pass_until stopped on nothing else")
        };
        assert!(
            attackers.contains(&troops),
            "an untapped, unsick Soldier is offered as an attacker: {attackers:?}"
        );
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(troops, Defender::Player(p1))],
                },
            )
            .expect("the Soldier came out of the list that offered it");

        // The trigger's two questions, in whichever order the engine asks
        // them: the "you may" (CR 603.3b) and the creature it names
        // (CR 603.3d). Answered here rather than through `pass_until`, whose
        // own `MayDo` arm would take the option for this test and hide it.
        for _ in 0..12 {
            match engine.pending().clone() {
                Pending::ChooseTargets {
                    player,
                    options,
                    min,
                    max,
                    ..
                } => {
                    assert_eq!(player, p0, "the attacking seat names the target");
                    assert_eq!((min, max), (1, 1), "one creature, and it asks once");
                    menu = options;
                    engine
                        .apply(
                            p0,
                            PlayerAction::ChooseObjects {
                                objects: vec![victim],
                            },
                        )
                        .expect("the Elf was one of the options the question enumerated");
                }
                Pending::YesNo {
                    player,
                    prompt: YesNoPrompt::MayDo,
                    ..
                } => {
                    assert_eq!(player, p0, "the attacking seat answers its own trigger");
                    asked += 1;
                    engine
                        .apply(p0, PlayerAction::YesNo(attack == 1))
                        .expect("a yes/no question takes either answer");
                }
                Pending::Priority { .. } if stack_is_empty(&engine) => break,
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                other => panic!("unexpected while the attack trigger resolves: {other:?}"),
            }
        }

        if attack == 0 {
            assert!(
                !is_tapped(&engine, victim),
                "\"you may tap target creature\" — declining leaves the board alone"
            );
        }
    }

    assert_eq!(
        asked, 2,
        "the printed \"you may\" is a question this seat answers, once per attack"
    );
    assert!(
        menu.contains(&victim),
        "the Elf the trigger tapped was on the menu it published: {menu:?}"
    );
    assert!(
        menu.contains(&mine),
        "\"target creature\" names no side of the battlefield: {menu:?}"
    );
    assert!(
        menu.contains(&bystander),
        "and no seat, so either Elf across the table is as legal as mine: {menu:?}"
    );
    assert!(
        menu.contains(&troops),
        "the attacking Soldier is a creature too, and its own trigger may name it: {menu:?}"
    );
    assert!(
        is_tapped(&engine, victim),
        "the creature the second attack named is the one the trigger tapped"
    );
    assert!(
        !is_tapped(&engine, bystander),
        "the Elf nobody named was not tapped: the trigger targets, it does not sweep"
    );
    assert!(
        !is_tapped(&engine, mine),
        "and neither was a creature of my own"
    );
    assert!(
        on_battlefield(&engine, p0, flanking_troops()).is_some(),
        "the trigger costs the Soldier nothing but its attack"
    );
}

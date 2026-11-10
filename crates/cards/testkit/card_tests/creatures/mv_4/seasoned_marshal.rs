//! `cards/creatures/mv_4/seasoned_marshal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seasoned Marshal is the printed 2/2 Human Soldier for {2}{W}{W} whose whole
/// text is "Whenever this creature attacks, you may tap target creature."
///
/// The word "may" is the card, so two Marshals attack together and each trigger
/// names a different one of the opponent's two Elves while the two may-do
/// questions are answered differently. Exactly one Elf is tapped afterwards,
/// which is what says the decline did nothing — a Marshal that tapped
/// unconditionally would leave both Elves down — and "target creature" is read
/// off the menu the question publishes, because it carries neither "you
/// control" nor "you don't".
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn seasoned_marshal_taps_the_creature_it_names_and_only_when_its_controller_says_so() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[seasoned_marshal(), seasoned_marshal()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let marshals = all_on_battlefield(&engine, p0, seasoned_marshal());
    assert_eq!(
        marshals.len(),
        2,
        "two Marshals, so the trigger fires twice"
    );
    assert_eq!(
        pt(&engine, marshals[0]),
        (2, 2),
        "the body the card prints, on a board with nothing pumping it"
    );
    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one per Marshal's may-do");
    let (first, second) = (elves[0], elves[1]);
    assert!(
        !is_tapped(&engine, first) && !is_tapped(&engine, second),
        "both Elves start untapped, so a tap is something the trigger did"
    );

    // The combat step is driven by hand: `pass_until` would answer the very
    // yes/no this test is about.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the walk stopped on the attack declaration it was waiting for")
    };
    assert!(
        attackers.contains(&marshals[0]) && attackers.contains(&marshals[1]),
        "both untapped 2/2s may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (marshals[0], Defender::Player(p1)),
                    (marshals[1], Defender::Player(p1)),
                ],
            },
        )
        .expect("both came out of the list that offered them");

    // Two triggers, each with its own target question and its own may-do. The
    // questions arrive in the engine's order rather than in the order a reader
    // expects, so they are answered as they come and counted.
    let mut named = 0usize;
    let mut may_dos = 0usize;
    for _ in 0..24 {
        if named == 2 && may_dos == 2 && stack_is_empty(&engine) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the attacking seat names the target");
                assert_eq!(
                    (min, max),
                    (1, 1),
                    "one creature, and the trigger asks once"
                );
                assert!(
                    options.contains(&first) && options.contains(&second),
                    "\"target creature\" is any creature, on either side of the \
                     table: {options:?}"
                );
                let pick = if named == 0 { first } else { second };
                named += 1;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![pick],
                        },
                    )
                    .expect("the Elf the question enumerated is a legal answer");
            }
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            } => {
                assert_eq!(player, p0, "the attacking seat answers its own may-do");
                let answer = may_dos == 0;
                may_dos += 1;
                engine
                    .apply(p0, PlayerAction::YesNo(answer))
                    .expect("both answers are legal for a may-do");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Marshals' attack triggers resolve: {other:?}"),
        }
    }
    assert_eq!(
        named, 2,
        "each attacking Marshal asks which creature to tap"
    );
    assert_eq!(
        may_dos, 2,
        "\"you may\" is a question: a Marshal that tapped unconditionally would \
         ask none and would have tapped both Elves by now"
    );

    assert_ne!(
        is_tapped(&engine, first),
        is_tapped(&engine, second),
        "one may-do was accepted and one declined, so exactly one of the two \
         named Elves is tapped — the accepted trigger tapped what it named and \
         the declined one left its own target standing"
    );
}

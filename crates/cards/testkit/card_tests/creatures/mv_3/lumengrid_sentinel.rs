//! `cards/creatures/mv_3/lumengrid_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lumengrid Sentinel — {2}{U}, a 1/2 Human Wizard with flying and "Whenever
/// an artifact you control enters, you may tap target permanent."
///
/// Both ends of the trigger are played. The seat's own Sol Ring entering is
/// what fires it, and the two questions it asks are taken in the order the
/// rules put them: whether the tap happens at all, and what it aims at. The
/// offer is where "target permanent" is legible — a land, the artifact that
/// just triggered the ability and an Elf across the table are one menu — and
/// the tap lands on the Elf alone, so the permanents still standing afterwards
/// say which one was named. The quiet turn after it is the other half of "you
/// control": an artifact entering under the other seat asks nothing, and that
/// ring is checked onto the battlefield so the silence is the filter and not a
/// spell that never resolved.
#[test]
#[allow(clippy::too_many_lines)] // one trigger, both of its questions, and the turn that must not ask
fn lumengrid_sentinel_may_tap_any_permanent_when_your_artifact_enters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lumengrid_sentinel(), island(), island()])
        .hand(0, &[quiet_artifact()])
        // Across the table: a plain 1/1 whose whole text is one mana ability,
        // plus the mana the other seat needs to cast an artifact of its own
        // in the control turn below.
        .battlefield(1, &[llanowar_elves(), island(), island()])
        .hand(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sentinel = on_battlefield(&engine, p0, lumengrid_sentinel()).expect("the Sentinel is out");
    let mine = on_battlefield(&engine, p0, island()).expect("an Island is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, sentinel), (1, 2), "a printed 1/2");
    assert!(
        keywords(&engine, sentinel).contains(KeywordSet::FLYING),
        "and flying is a keyword the layers project onto it"
    );

    // {1} off the two Islands: what enters is an artifact under this seat.
    cast_from_hand(&mut engine, p0, quiet_artifact());

    let mut saw_may_do = false;
    let mut aimed = false;
    for _ in 0..30 {
        if is_tapped(&engine, theirs) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            } => {
                saw_may_do = true;
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::ChooseTargets {
                player, options, ..
            } => {
                let ring = on_battlefield(&engine, p0, quiet_artifact())
                    .expect("the Sol Ring that triggered the ability is on the battlefield");
                assert!(
                    options.contains(&ring),
                    "\"target permanent\" includes the artifact that triggered it: {options:?}"
                );
                assert!(options.contains(&mine), "and a land beside it: {options:?}");
                assert!(
                    options.contains(&theirs),
                    "and a permanent across the table, which is what `any` means: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![theirs],
                        },
                    )
                    .expect("the Elf was one of the options it published");
                aimed = true;
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected between the Sol Ring entering and the tap: {other:?}"),
        }
    }
    assert!(
        saw_may_do,
        "\"you may tap target permanent\" is a question the trigger asks"
    );
    assert!(
        aimed,
        "and the permanent it taps is chosen out of the offer it published"
    );
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring resolved");
    assert!(
        is_tapped(&engine, theirs),
        "the Elf across the table is the permanent that was named, and it is tapped"
    );
    assert!(
        !is_tapped(&engine, ring),
        "while the artifact that triggered the ability was never the one it spent"
    );

    // The control: the same artifact entering under the *other* seat.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, quiet_artifact());

    let mut asked = false;
    for _ in 0..20 {
        if on_battlefield(&engine, p1, quiet_artifact()).is_some()
            && stack_is_empty(&engine)
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1)
        {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
            | Pending::ChooseTargets { .. } => {
                asked = true;
                break;
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected while the other seat's artifact entered: {other:?}"),
        }
    }
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the Sol Ring really did enter under the other seat, so the silence \
         below is the filter `you control` and not a spell that never resolved"
    );
    assert!(
        !asked,
        "the Sentinel watches artifacts *you* control: the artifact entering \
         across the table asked nothing"
    );
}

//! `cards/creatures/mv_3/reclamation_sage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "032ec6e2-6cc3-4a97-9cc7-3233f5e11904"
// oracle_id = "90076bf5-aa9a-4a6e-9035-9aa97fd5561e"
/// Luminarch Ascension, the second half of the Sage's target filter.
///
/// A plain enchantment whose only trigger is on an end step this scenario
/// never reaches, so it sits on the table as a legal target and answers
/// nothing on the way.
/// Reclamation Sage — {2}{G} 2/1 Elf Shaman: "When this creature enters, you
/// may destroy target artifact or enchantment."
///
/// Both printed halves are read off one resolution. The target question is
/// what the filter produces, so the Sol Ring and the Ascension across the
/// table are on it and the Elf and the Forest beside them are not — an arm
/// dropped from `ARTIFACT_OR_ENCHANTMENT`, or a filter that fell back to
/// `Any`, changes that list. And the "you may" is answered yes: the named
/// artifact goes to its owner's graveyard while the enchantment the same
/// trigger could equally have named stays exactly where it was, which is what
/// separates "destroys the target it was given" from "destroys everything
/// legal".
#[test]
#[allow(clippy::too_many_lines)] // a whole game, as every test in this file is
fn reclamation_sage_destroys_the_artifact_it_names_and_leaves_the_rest_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let enchantment = their_enchantment();
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[reclamation_sage()])
        // Two halves of the filter, plus the two permanents it must decline:
        // a creature and a land.
        .battlefield(
            1,
            &[quiet_artifact(), enchantment, llanowar_elves(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let ascension = on_battlefield(&engine, p1, enchantment).expect("their Ascension is out");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    cast_from_hand(&mut engine, p0, reclamation_sage());

    // The trigger is answered where it arrives rather than where it is
    // expected: the "you may" and the target choice are one resolution, and
    // which of the two is asked first is the engine's business, not the
    // test's.
    let mut aimed: Option<ObjectId> = None;
    for _ in 0..30 {
        if aimed.is_some()
            && stack_is_empty(&engine)
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0)
        {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, p0, "the Sage's controller aims its own trigger");
                assert!(
                    options.contains(&ring) && options.contains(&ascension),
                    "\"target artifact or enchantment\" offers both halves of \
                     the printed filter: {options:?}"
                );
                assert_eq!(
                    options.len(),
                    2,
                    "and nothing else on either side of the table is either: {options:?}"
                );
                assert!(
                    !options.contains(&elves),
                    "a creature is neither an artifact nor an enchantment: {options:?}"
                );
                assert!(
                    !options.contains(&land),
                    "and neither is a land: {options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![ring],
                        },
                    )
                    .expect("the artifact was one of the options");
                aimed = Some(ring);
            }
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            } => {
                engine
                    .apply(player, PlayerAction::YesNo(true))
                    .expect("`you may` is a question with a yes");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Sage's trigger resolves: {other:?}"),
        }
    }

    assert_eq!(aimed, Some(ring), "the trigger asked for a target");
    assert!(
        stack_is_empty(&engine),
        "the trigger resolved and left nothing behind: {:?}",
        engine.pending()
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat that aimed it holds priority again, got {:?}",
        engine.pending()
    );

    assert!(
        on_battlefield(&engine, p0, reclamation_sage()).is_some(),
        "the Sage itself resolved onto the battlefield and stays"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "\"destroy target artifact\" — the named artifact is in its owner's \
         graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, enchantment).is_some(),
        "the Ascension was offered and not named, so it stands: the trigger \
         destroys the one target it was given and not every legal one"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some()
            && on_battlefield(&engine, p1, forest()).is_some(),
        "and the permanents the filter never offered were never touched"
    );
}

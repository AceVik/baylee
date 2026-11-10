//! `cards/instants/mv_6/banishing_stroke.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Banishing Stroke: its miracle is offered only over something to target.
///
/// "Put target artifact, creature, or enchantment on the bottom of its
/// owner's library." Choosing that target is a step of casting it (CR
/// 601.2c), and a spell that cannot take the step cannot be cast (CR 601.2):
/// drawn onto a board with no artifact, creature or enchantment, the miracle
/// could only be declined, so it is not asked. It was, and the house said
/// yes to it in r001's games 1581, 3288 and 3554; the engine refused the
/// "yes" after spending the offer, and none of the three could be replayed.
///
/// Every card in both libraries is a Banishing Stroke, so every first draw of
/// a turn is one. With a creature on the table each of those draws offers
/// the miracle, which is what keeps the empty board's silence from being a
/// test that sees nothing.
#[test]
fn banishing_stroke_offers_its_miracle_only_over_a_target() {
    for (board, targets) in [(vec![], false), (vec![ondu_cleric()], true)] {
        let mut engine = Duel::new(31, banishing_stroke())
            .battlefield(1, &board)
            .start();
        keep_mulligans(&mut engine);
        let mut offers = 0;
        for _ in 0..200 {
            if engine.state().turn.number > 4 {
                break;
            }
            match engine.pending().clone() {
                Pending::YesNo {
                    player,
                    prompt: YesNoPrompt::Miracle { .. },
                    ..
                } => {
                    offers += 1;
                    engine.apply(player, PlayerAction::YesNo(false)).unwrap();
                }
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
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
                other => panic!("unexpected question: {other:?}"),
            }
        }
        assert!(engine.state().turn.number > 4, "the game stalled");
        if targets {
            assert!(
                offers >= 3,
                "a draw over a creature was not offered: {offers}"
            );
        } else {
            assert_eq!(offers, 0, "a miracle with nothing to target was offered");
        }
    }
}

/// Banishing Stroke: "Put target artifact, creature, or enchantment on the
/// bottom of its owner's library." The success path: the Elves leave the
/// battlefield and are the library's last card, under the library that was
/// already there.
#[test]
fn banishing_stroke_puts_its_target_on_the_bottom_of_its_owners_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4704, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[banishing_stroke()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves");
    let before = library_size(&engine, p1);
    cast_from_hand(&mut engine, p0, banishing_stroke());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elves are on offer");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert_eq!(library_size(&engine, p1), before + 1);
    let library = engine.state().zones.list(ZoneLocation::Library(p1)).clone();
    let bottom_candidates = [library[0], library[library.len() - 1]];
    let is_elves = |id: ObjectId| {
        engine
            .state()
            .object(id)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))
    };
    // The library's top is its last list element (as `seed_graveyard`
    // reads it), so the bottom is the first.
    assert!(
        is_elves(bottom_candidates[0]),
        "the Elves are at the bottom"
    );
    assert!(!is_elves(bottom_candidates[1]), "and not on top");
    assert!(in_graveyard(&engine, p0, banishing_stroke()).is_some());
}

//! `cards/creatures/artifacts/mv_4/architects_of_will.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ArrangePrompt;

fn architects_of_will() -> CardIndex {
    card_index("79d290ea-c095-4135-9711-f3ce089dae43")
}

/// Architects of Will — {2}{U}{B} 3/3: "When this creature enters, look at
/// the top three cards of target player's library, then put them back in any
/// order." Aimed at the opponent, the player is shown exactly their top three
/// and puts them back reversed: the opponent's library now shows the old
/// third card first.
#[test]
fn architects_of_will_reorders_the_top_three_of_a_target_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), swamp(), forest(), forest()])
        .hand(0, &[architects_of_will()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let before = library_top(&engine, p1, 3);
    assert_eq!(before.len(), 3);

    cast_from_hand(&mut engine, p0, architects_of_will());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("any player is a legal target");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for this");
    };
    assert_eq!(player, p0, "the Architects' controller arranges");
    assert_eq!(prompt, ArrangePrompt::Order);
    assert_eq!(cards, before, "the top three of the target's library");

    let reversed: Vec<ObjectId> = before.iter().rev().copied().collect();
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![reversed.clone()],
            },
        )
        .expect("any order is legal");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_top(&engine, p1, 3), reversed);
    assert_eq!(
        library_top(&engine, p0, 3).len(),
        3,
        "their own library was not the target"
    );
}

/// Cycling {U/B}: from hand, for one blue mana, the card is discarded and a
/// card is drawn; the Architects land in the graveyard and never enter.
#[test]
fn architects_of_will_cycles_for_one_hybrid_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[architects_of_will()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library_before = library_size(&engine, p0);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 1);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, architects_of_will(), 0);
    answer_until_settled(&mut engine);

    assert!(in_graveyard(&engine, p0, architects_of_will()).is_some());
    assert!(on_battlefield(&engine, p0, architects_of_will()).is_none());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "one out, one drawn"
    );
    assert!(in_hand(&engine, p0, architects_of_will()).is_none());
    assert_eq!(library_size(&engine, p0), library_before - 1);
}

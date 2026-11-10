//! `cards/enchantments/mv_6/mind_over_matter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn mind_over_matter() -> CardIndex {
    card_index("656afde2-cdf3-4907-a0fc-7ac93f5d3e03")
}

/// Activates the ability at `target`, answering the discard out of the
/// hand, the target as given and the "you may" with `use_it`.
#[track_caller]
fn activate_at(engine: &mut Engine<RegistryLookup>, target: ObjectId, use_it: bool) {
    let p0 = PlayerId::new(0);
    activate(engine, p0, mind_over_matter(), 0);
    for _ in 0..40 {
        if at_rest(engine, p0) {
            return;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                assert!(options.contains(&target), "the target is on the menu");
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![target],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            } => {
                engine.apply(p0, PlayerAction::YesNo(use_it)).unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            _ => {
                let (player, action) = answer_one(engine).expect("an answerable question");
                engine.apply(player, action).unwrap();
            }
        }
    }
    panic!("the activation never finished: {:?}", engine.pending());
}

/// "Discard a card: You may tap or untap target artifact, creature, or
/// land." The first use taps the opposing Ogre, the second untaps it; each
/// costs a card from my hand.
#[test]
fn mind_over_matter_taps_then_untaps_by_discarding() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mind_over_matter()])
        .hand(0, &[forest(), plains()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    assert!(!is_tapped(&engine, ogre));

    activate_at(&mut engine, ogre, true);
    assert!(is_tapped(&engine, ogre), "tapped");
    assert_eq!(hand_len(&engine, p0), 1, "one card discarded");

    activate_at(&mut engine, ogre, true);
    assert!(!is_tapped(&engine, ogre), "untapped again");
    assert_eq!(hand_len(&engine, p0), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2
    );
}

/// "You may": the card is discarded as a cost either way, but declining
/// leaves the Ogre untapped.
#[test]
fn mind_over_matter_may_leave_the_target_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mind_over_matter()])
        .hand(0, &[forest()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    activate_at(&mut engine, ogre, false);
    assert!(!is_tapped(&engine, ogre));
    assert_eq!(hand_len(&engine, p0), 0, "the cost was paid");
}

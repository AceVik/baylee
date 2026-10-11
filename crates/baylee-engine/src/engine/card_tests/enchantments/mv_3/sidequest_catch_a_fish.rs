//! `cards/enchantments/mv_3/sidequest_catch_a_fish.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The upkeep looks even when a land is on top: acknowledging that private
/// information leaves the front face unchanged and adds no card to hand.
#[test]
fn sidequest_catch_a_fish_looks_at_an_ineligible_land_without_transforming() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(108, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[sidequest_catch_a_fish()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lib_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, sidequest_catch_a_fish());
    pass_until(&mut engine, stack_is_empty);

    let quest = on_battlefield(&engine, p0, sidequest_catch_a_fish())
        .expect("Sidequest: Catch a Fish on battlefield");
    assert_eq!(
        engine.state().object(quest).map(|o| o.face_index),
        Some(0),
        "Sidequest is on face 0"
    );

    let t = types(&engine, quest);
    assert!(
        t.contains(TypeSet::ENCHANTMENT),
        "Sidequest is an enchantment"
    );
    assert!(!t.contains(TypeSet::LAND), "Sidequest is not a land");

    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == quest),
        "front face offers no activated abilities with floating mana"
    );

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::PutIntoHand,
                ..
            }
        )
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == crate::turn::Step::Main
    });

    assert_eq!(
        library_size(&engine, p0),
        lib_before - 1,
        "two turns pass, so p0 draws once for its own draw step (CR 504.1) — \
         the ineligible top land does not add another draw"
    );
    assert_eq!(
        engine.state().object(quest).map(|o| o.face_index),
        Some(0),
        "Sidequest remains on face 0 in the following turn"
    );
}

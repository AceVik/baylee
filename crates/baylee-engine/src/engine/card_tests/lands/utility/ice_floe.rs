//! `cards/lands/utility/ice_floe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ice Floe prints "You may choose not to untap this land during your untap
/// step." and "{T}: Tap target creature without flying that's attacking you.
/// It doesn't untap during its controller's untap step for as long as this
/// land remains tapped."
///
/// Turn two: the opponent attacks with an Elf and a Serra Angel (flying) and
/// keeps a Gray Ogre home; I own a Gray Ogre too. The attackers are declared
/// and my Floe's ability is on the menu. Returns the engine with the Floe
/// activated at the Elf and resolved.
fn floe_on_the_elf() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ice_floe(), gray_ogre()])
        .battlefield(1, &[llanowar_elves(), serra_angel(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let floe = on_battlefield(&engine, p0, ice_floe()).expect("Ice Floe");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("Elf");
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("Angel");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!("expected choose attackers, got {:?}", engine.pending());
    };
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, defenders[0]), (angel, defenders[0])],
            },
        )
        .unwrap();
    if let Pending::Priority { player, .. } = engine.pending().clone()
        && player == p1
    {
        engine.apply(p1, PlayerAction::PassPriority).unwrap();
    }
    activate(&mut engine, p0, ice_floe(), 1);
    let options = aim_at(&mut engine, p0, elf);
    assert_eq!(
        options,
        vec![elf],
        "only the attacking non-flyer: {options:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, floe), "{{T}} was paid");
    assert!(is_tapped(&engine, elf));
    (engine, floe, elf)
}

/// Walks until the untap step asks which permanents stay tapped, handing the
/// question back unanswered (`answer_one` would untap).
#[track_caller]
fn to_the_untap_question(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    for _ in 0..200 {
        if let Pending::ChooseCards {
            options,
            prompt: ChoicePrompt::LeaveTapped,
            ..
        } = engine.pending().clone()
        {
            return options;
        }
        let (player, action) = answer_one(engine).expect("a rest on the way");
        engine.apply(player, action).expect("legal");
    }
    panic!("the untap step never asked");
}

/// While Floe stays tapped (I choose not to untap it) the Elf stays tapped
/// through its controller's next untap step.
#[test]
fn ice_floe_keeps_the_attacker_down_through_its_untap_step_while_it_stays_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, floe, elf) = floe_on_the_elf();
    let options = to_the_untap_question(&mut engine);
    assert!(options.contains(&floe), "the Floe may stay down");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![floe],
            },
        )
        .unwrap();
    reach_main_phase(&mut engine, p0);
    assert!(is_tapped(&engine, floe), "stayed tapped into my main phase");
    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, elf), "the lock held in its untap step");
}

/// Once the Floe untaps, the Elf untaps normally in its next untap step.
#[test]
fn ice_floe_releases_the_attacker_after_it_untaps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, floe, elf) = floe_on_the_elf();
    to_the_untap_question(&mut engine);
    let (player, action) = answer_one(&engine).expect("the question");
    engine.apply(player, action).unwrap();
    reach_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, floe), "untapped");
    assert!(is_tapped(&engine, elf), "the Elf waits for its own step");
    reach_their_main_phase(&mut engine, p1);
    assert!(!is_tapped(&engine, elf), "released: untapped normally");
}

//! Oracle checklist: pay {W} and {T}; target any creature; resolution taps exactly the selected creature; source cannot activate again while tapped.
#[allow(clippy::wildcard_imports)]
use super::*;
fn akroan_mastiff() -> CardIndex {
    card_index("6039b9b7-e49a-4ea3-887c-a17dd502d63f")
}

#[test]
fn mastiff_spends_white_and_taps_the_target_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[akroan_mastiff(), plains()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let mastiff = on_battlefield(&e, p0, akroan_mastiff()).unwrap();
    let plains_land = on_battlefield(&e, p0, plains()).unwrap();
    let targets = all_on_battlefield(&e, p1, llanowar_elves());
    assert_eq!(targets.len(), 2);
    tap_all_mana(&mut e, p0);
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::White),
        1
    );
    activate(&mut e, p0, akroan_mastiff(), 0);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = e.pending().clone()
    else {
        panic!("expected creature target prompt: {:?}", e.pending());
    };
    assert_eq!((min, max), (1, 1));
    assert_eq!(
        options.len(),
        3,
        "both opponent creatures and the Mastiff itself are creatures"
    );
    assert!(targets.iter().all(|id| options.contains(id)));
    assert!(
        options.contains(&mastiff),
        "the Oracle text does not restrict controller"
    );
    assert!(
        !options.contains(&plains_land),
        "target creature excludes a land"
    );
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![targets[0]],
        },
    )
    .unwrap();
    assert!(is_tapped(&e, mastiff), "tap is a cost");
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::White),
        0,
        "white mana was paid"
    );
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, targets[0]));
    assert!(!is_tapped(&e, targets[1]));
}

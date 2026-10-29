//! `cards/creatures/mv_3/rebel_informer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn rebel_informer() -> CardIndex {
    card_index("85ef10fd-4a48-4a37-baad-f663fb5439e0")
}

/// Rebel Informer ({2}{B} 1/2 Creature — Human Mercenary Rebel):
/// "This creature can't be the target of white spells or abilities from white sources."
/// "{3}: Put target nontoken Rebel on the bottom of its owner's library."
#[test]
fn rebel_informer_puts_target_nontoken_rebel_on_bottom_of_owners_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = rebel_informer();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[subject, swamp(), swamp(), swamp()])
        .battlefield(1, &[index::RAMOSIAN_SERGEANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rebel = on_battlefield(&engine, p1, index::RAMOSIAN_SERGEANT).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, subject, 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rebel],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::RAMOSIAN_SERGEANT).is_none());
    let lib = engine.state().zones.list(ZoneLocation::Library(p1));
    let bottom = *lib.first().expect("library not empty");
    assert_eq!(
        engine.state().object(bottom).unwrap().card.map(|c| c.index),
        Some(index::RAMOSIAN_SERGEANT)
    );
}

/// Rebel Informer cannot be targeted by white spells (such as Swords to Plowshares),
/// but can be targeted by nonwhite spells (such as Lightning Bolt).
#[test]
fn rebel_informer_cannot_be_targeted_by_white_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = rebel_informer();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), mountain(), index::GRIZZLY_BEARS])
        .battlefield(1, &[subject])
        .hand(0, &[index::SWORDS_TO_PLOWSHARES, index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let informer = on_battlefield(&engine, p1, subject).unwrap();
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, index::SWORDS_TO_PLOWSHARES);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Swords to Plowshares target");
    };
    assert!(
        !options.contains(&informer),
        "white spell must not be able to target Rebel Informer"
    );
    assert!(options.contains(&bears));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, index::LIGHTNING_BOLT);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![informer],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, subject).is_some());
}

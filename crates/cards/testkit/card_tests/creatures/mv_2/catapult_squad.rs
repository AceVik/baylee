//! `cards/creatures/mv_2/catapult_squad.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::turn::Step;

fn catapult_squad() -> CardIndex {
    card_index("f1ff7a67-1add-4de1-bc25-62f63626c43a")
}

/// Catapult Squad ({1}{W} 2/1 Human Soldier):
/// "Tap two untapped Soldiers you control: This creature deals 2 damage to
/// target attacking or blocking creature."
#[test]
fn catapult_squad_taps_two_soldiers_you_control_to_hit_attacking_or_blocking_creature() {
    let mut e = lk::game(
        SEED,
        &[catapult_squad(), benalish_hero()],
        &[grizzly_bears(), pearled_unicorn()],
        &[],
        &[],
    );
    let squad = lk::obj(&e, lk::P0, catapult_squad());
    let hero = lk::obj(&e, lk::P0, benalish_hero());
    let bear = lk::obj(&e, lk::P1, grizzly_bears());
    let unicorn = lk::obj(&e, lk::P1, pearled_unicorn());

    assert!(
        !lk::offered(&e, catapult_squad(), lk::P0, 0),
        "no legal target outside combat"
    );

    reach_their_main_phase(&mut e, lk::P1);
    let _ = lk::attack(&mut e, &[bear]);
    lk::block(&mut e, &[]);
    lk::to_step(&mut e, lk::P0, Step::DeclareBlockers);

    activate(&mut e, lk::P0, catapult_squad(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!(
            "expected attacking/blocking target choice, got {:?}",
            e.pending()
        );
    };
    assert_eq!(
        options,
        vec![bear],
        "non-attacking Unicorn is not a legal target"
    );
    assert!(!options.contains(&unicorn));
    lk::aim(&mut e, &[bear], &[]);

    let Pending::ChooseCards { options: taps, .. } = e.pending().clone() else {
        panic!("expected TapN Soldier choice, got {:?}", e.pending());
    };
    assert!(taps.contains(&squad) && taps.contains(&hero));
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![squad, hero],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert!(lk::tapped(&e, squad) && lk::tapped(&e, hero));
    assert!(on_battlefield(&e, lk::P1, grizzly_bears()).is_none());
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P0), 20);
}

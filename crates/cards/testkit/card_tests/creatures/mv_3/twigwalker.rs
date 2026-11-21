//! `cards/creatures/mv_3/twigwalker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn twigwalker() -> CardIndex {
    card_index("ee90b4c5-f646-47a9-8aa8-393c86e35536")
}

/// Twigwalker ({2}{G} 2/2 Insect):
/// "{1}{G}, Sacrifice this creature: Two target creatures each get +2/+2 until
/// end of turn."
#[test]
fn twigwalker_requires_two_target_creatures_and_pumps_both_until_end_of_turn() {
    // Alone on the battlefield (< 2 creatures total), Twigwalker cannot activate.
    let mut e_solo = lk::game(SEED, &[twigwalker()], &[], &[], &[]);
    lk::float(&mut e_solo, lk::P0, &[(ManaColor::Green, 2)]);
    assert!(!lk::offered(&e_solo, twigwalker(), lk::P0, 0));

    let mut e = lk::game(
        SEED,
        &[twigwalker(), grizzly_bears(), benalish_hero()],
        &[],
        &[],
        &[],
    );
    let bear = lk::obj(&e, lk::P0, grizzly_bears());
    let hero = lk::obj(&e, lk::P0, benalish_hero());

    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 2)]);
    activate(&mut e, lk::P0, twigwalker(), 0);
    let Pending::ChooseTargets { min, max, .. } = e.pending().clone() else {
        panic!("expected 2-creature target choice, got {:?}", e.pending());
    };
    assert_eq!((min, max), (2, 2));
    lk::aim(&mut e, &[bear, hero], &[]);
    lk::settle(&mut e);

    assert!(on_battlefield(&e, lk::P0, twigwalker()).is_none());
    assert!(in_graveyard(&e, lk::P0, twigwalker()).is_some());
    assert_eq!(pt(&e, bear), (4, 4));
    assert_eq!(pt(&e, hero), (3, 3));

    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(pt(&e, bear), (2, 2));
    assert_eq!(pt(&e, hero), (1, 1));
}

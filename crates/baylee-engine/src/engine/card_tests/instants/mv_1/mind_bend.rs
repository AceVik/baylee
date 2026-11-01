//! `cards/instants/mv_1/mind_bend.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

// oracle_id = "d8377869-f8f7-4b4b-93ad-3ad3c4826b1a"
fn mind_bend() -> CardIndex {
    card_index("d8377869-f8f7-4b4b-93ad-3ad3c4826b1a")
}

#[test]
fn mind_bend_changes_color_word_on_target_permanent_indefinitely() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                forest(),
                index::BLACK_KNIGHT,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[mind_bend(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let knight = on_battlefield(&engine, p0, index::BLACK_KNIGHT).unwrap();
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, mind_bend());
    aim_at(&mut engine, p0, knight);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    // Mode 0: replace a color word with another.
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    // WUBRG order: white (0) -> green (4) is 0 * 4 + (4 - 1) = 3.
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, giant_growth());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending())
    };
    assert!(
        !options.contains(&knight),
        "Black Knight now has protection from green"
    );
    assert!(options.contains(&bears));
}

/// The other half of the sentence, "one basic land type with another": Island
/// to Forest turns Merfolk Raiders' islandwalk into forestwalk, so a defender
/// with a Forest and no Island cannot block it with Grizzly Bears. Mind Bend's
/// target menu holds permanents only.
#[test]
fn mind_bend_replaces_a_basic_land_type_word_on_a_keyword() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = lk::game(
        SEED,
        &[index::MERFOLK_RAIDERS],
        &[forest(), index::GRIZZLY_BEARS],
        &[mind_bend()],
        &[],
    );
    let raiders = lk::obj(&engine, p0, index::MERFOLK_RAIDERS);
    let bears = lk::obj(&engine, p1, index::GRIZZLY_BEARS);

    // Merfolk Raiders is phased out through turn 1 and back in on turn 3.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    lk::float(&mut engine, p0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, p0, mind_bend());
    let menu = aim_at(&mut engine, p0, raiders);
    for id in &menu {
        assert_eq!(
            engine.state().object(*id).unwrap().zone,
            Zone::Battlefield,
            "Mind Bend targets a permanent"
        );
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    // Mode 1: replace a basic land type word.
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    // WUBRG order: Island (1) -> Forest (4) is 1 * 4 + (4 - 1) = 7.
    engine.apply(p0, PlayerAction::ChooseNumber(7)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let offer = lk::attack(&mut engine, &[raiders]);
    assert!(
        !lk::may_block(&offer, bears, raiders),
        "forestwalk: the defender controls a Forest"
    );
}

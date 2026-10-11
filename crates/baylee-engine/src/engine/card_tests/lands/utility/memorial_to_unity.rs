//! `cards/lands/utility/memorial_to_unity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn memorial_to_unity_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(336, forest())
        .hand(0, &[memorial_to_unity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, memorial_to_unity());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, memorial_to_unity(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn memorial_inspects_the_printed_count_and_reveals_only_the_matching_card() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::MEMORIAL_TO_UNITY, plains()])
        .hand(0, &[index::GRIZZLY_BEARS, index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, index::MEMORIAL_TO_UNITY).unwrap();
    let chosen = hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
    hand_to_library_top(&mut e, p0, index::LIGHTNING_BOLT);
    let s = e.dev_state_mut(p0).unwrap();
    s.set_tapped(land, false);
    s.players[0].mana_pool.add(ManaColor::Green, 3);
    e.refresh_offer();
    activate(&mut e, p0, index::MEMORIAL_TO_UNITY, 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert_eq!(options, &vec![chosen]);
    assert_eq!((*min, *max), (0, 1));
    assert_eq!(e.inspected_cards().len(), 5);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![chosen],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(chosen).unwrap().zone, Zone::Hand);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Graveyard);
}

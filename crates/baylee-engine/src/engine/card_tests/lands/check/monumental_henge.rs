//! `cards/lands/check/monumental_henge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn monumental_henge_enters_untapped_with_plains_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(207, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[monumental_henge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, monumental_henge());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, monumental_henge(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn henge_inspects_the_printed_count_and_reveals_only_the_matching_card() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::MONUMENTAL_HENGE, plains()])
        .hand(0, &[index::MIND_STONE, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, index::MONUMENTAL_HENGE).unwrap();
    let chosen = hand_to_library_top(&mut e, p0, index::MIND_STONE);
    hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
    let s = e.dev_state_mut(p0).unwrap();
    s.set_tapped(land, false);
    s.players[0].mana_pool.add(ManaColor::White, 4);
    e.refresh_offer();
    activate(&mut e, p0, index::MONUMENTAL_HENGE, 1);
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
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
}

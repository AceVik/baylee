//! `cards/lands/utility/lupinflower_village.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn lupinflower_village_adds_restricted_creature_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(228, forest())
        .battlefield(0, &[lupinflower_village()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let village = on_battlefield(&engine, p0, lupinflower_village()).expect("Village deployed");
    activate(&mut engine, p0, lupinflower_village(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::White);
    assert!(is_tapped(&engine, village));
}

#[test]
fn lupinflower_inspects_the_printed_count_and_reveals_only_the_matching_card() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::LUPINFLOWER_VILLAGE, plains()])
        .hand(0, &[index::STORM_CROW, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, index::LUPINFLOWER_VILLAGE).unwrap();
    let chosen = hand_to_library_top(&mut e, p0, index::STORM_CROW);
    hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
    let s = e.dev_state_mut(p0).unwrap();
    s.set_tapped(land, false);
    s.players[0].mana_pool.add(ManaColor::White, 2);
    e.refresh_offer();
    activate(&mut e, p0, index::LUPINFLOWER_VILLAGE, 2);
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
    assert_eq!(e.inspected_cards().len(), 6);
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

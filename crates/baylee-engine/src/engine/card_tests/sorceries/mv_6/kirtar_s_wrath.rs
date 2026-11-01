//! Oracle scenarios for Kirtar's Wrath.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn kirtar_s_wrath() -> CardIndex {
    card_index("4f66d82a-492f-4638-9f77-190d4a33ad7f")
}

#[test]
fn kirtar_s_wrath_without_threshold_destroys_all_creatures_and_creates_no_tokens() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(
            0,
            &[
                index::PLAINS,
                index::PLAINS,
                index::PLAINS,
                index::PLAINS,
                index::PLAINS,
                index::PLAINS,
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::GRAY_OGRE])
        .hand(0, &[kirtar_s_wrath()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(bear)
        .unwrap()
        .regeneration_shields = 1;
    cast_from_hand(&mut e, p0, kirtar_s_wrath());
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&e, p1, index::GRAY_OGRE).is_some());
    assert!(tokens_of(&e, p0).is_empty());
}

#[test]
fn kirtar_s_wrath_with_threshold_destroys_all_creatures_and_creates_two_spirits() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![index::PLAINS; 6];
    board.extend_from_slice(&[index::FOREST; 7]);
    board.push(index::GRIZZLY_BEARS);
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(0, &board)
        .battlefield(1, &[index::GRAY_OGRE])
        .hand(0, &[kirtar_s_wrath()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(bear)
        .unwrap()
        .regeneration_shields = 1;
    let forests = all_on_battlefield(&e, p0, index::FOREST);
    for &f in &forests {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                f,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();

    cast_from_hand(&mut e, p0, kirtar_s_wrath());
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&e, p1, index::GRAY_OGRE).is_some());
    let spirits = tokens_of(&e, p0);
    assert_eq!(spirits.len(), 2);
    for &s in &spirits {
        assert_eq!(pt(&e, s), (1, 1));
        assert!(keywords(&e, s).contains(KeywordSet::FLYING));
        let characteristics = e.state().object(s).unwrap().characteristics();
        assert!(
            characteristics
                .colors
                .contains(baylee_core::color::Color::White)
        );
        assert!(
            characteristics
                .subtypes
                .contains(baylee_core::generated::subtypes::creature::SPIRIT)
        );
    }
}

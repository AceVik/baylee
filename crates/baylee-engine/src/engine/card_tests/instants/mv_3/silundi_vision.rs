//! `cards/instants/mv_3/silundi_vision.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn silundi_vision_privately_inspects_six_but_only_reveals_an_optional_spell() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for choose in [false, true] {
        let mut e = Duel::new(45, island())
            .battlefield(0, &[island(), island(), island()])
            .hand(
                0,
                &[
                    silundi_vision(),
                    index::LIGHTNING_BOLT,
                    index::GRIZZLY_BEARS,
                ],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let bolt = hand_to_library_top(&mut e, p0, index::LIGHTNING_BOLT);
        let bear = hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
        let looked: Vec<_> = e
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .iter()
            .rev()
            .take(6)
            .copied()
            .collect();
        cast_from_hand(&mut e, p0, silundi_vision());
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards {
            options, min, max, ..
        } = e.pending()
        else {
            unreachable!()
        };
        assert_eq!(options, &vec![bolt]);
        assert_eq!((*min, *max), (0, 1));
        assert_eq!(e.inspected_cards(), looked);
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bear]
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
        let seq = e.state().journal.last_seq();
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: if choose { vec![bolt] } else { vec![] },
            },
        )
        .unwrap();
        assert!(
            !matches!(e.pending(), Pending::Arrange { .. }),
            "random order asks nobody"
        );
        pass_until(&mut e, stack_is_empty);
        assert!(e.inspected_cards().is_empty());
        assert_eq!(
            e.state().object(bolt).unwrap().zone,
            if choose { Zone::Hand } else { Zone::Library }
        );
        let reveals: Vec<_> = e
            .state()
            .journal
            .entries()
            .iter()
            .filter(|entry| entry.seq > seq)
            .filter_map(|entry| {
                if let crate::event::GameEvent::Revealed { cards, .. } = &entry.event {
                    Some(cards.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(reveals, if choose { vec![vec![bolt]] } else { vec![] });
        let bottom = &e.state().zones.list(ZoneLocation::Library(p0))[..if choose { 5 } else { 6 }];
        assert!(
            looked
                .iter()
                .filter(|id| !choose || **id != bolt)
                .all(|id| bottom.contains(id))
        );
    }
}

#[test]
fn silundi_with_a_short_library_and_no_eligible_card_still_shows_every_inspected_card() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[silundi_vision()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let library = e.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &library[..library.len() - 2] {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                *id,
                ZoneLocation::Exile(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    cast_from_hand(&mut e, p0, silundi_vision());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert!(options.is_empty());
    assert_eq!((*min, *max), (0, 0));
    assert_eq!(e.inspected_cards().len(), 2);
    e.apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().zones.list(ZoneLocation::Library(p0)).len(), 2);
}

//! Isochron Scepter casts a free copy while the imprint stays in exile.

use super::*;

fn isochron_scepter() -> CardIndex {
    card_index("ddd649f4-dbaf-48ec-8ff7-95581257772d")
}

#[test]
fn isochron_scepter_casts_a_copy_and_leaves_the_original_exiled() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[isochron_scepter(), lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, isochron_scepter());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let bolt = in_hand(&e, p0, lightning_bolt()).unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bolt],
        },
    )
    .unwrap();
    reach_main_phase(&mut e, p0);
    assert_eq!(e.state().object(bolt).unwrap().zone, Zone::Exile);

    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, isochron_scepter(), 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected the linked exile offer")
    };
    assert_eq!(options, vec![bolt]);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bolt],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, 17);
    assert_eq!(
        e.state().object(bolt).unwrap().zone,
        Zone::Exile,
        "the original never moved"
    );
}

/// "An instant card with mana value 2 or less": a sorcery of mana value 1
/// and an instant of mana value 3 are not offered for the imprint; the Bolt
/// beside them is.
#[test]
fn isochron_scepter_offers_only_cheap_instants_to_imprint() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .hand(
            0,
            &[
                isochron_scepter(),
                lightning_bolt(),
                index::ASSAULT_STROBE,
                index::OVERRIDE,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, isochron_scepter());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected the imprint choice, got {:?}", e.pending())
    };
    let bolt = in_hand(&e, p0, lightning_bolt()).unwrap();
    let sorcery = in_hand(&e, p0, index::ASSAULT_STROBE).unwrap();
    let expensive = in_hand(&e, p0, index::OVERRIDE).unwrap();
    assert!(options.contains(&bolt));
    assert!(!options.contains(&sorcery), "a sorcery is not an instant");
    assert!(!options.contains(&expensive), "mana value 3 is more than 2");
}

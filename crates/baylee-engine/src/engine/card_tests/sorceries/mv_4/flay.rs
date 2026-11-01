//! `cards/sorceries/mv_4/flay.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn flay() -> CardIndex {
    card_index("987b74f4-02cb-4d47-94e5-d58a3c8e92e4")
}

/// Flay ({3}{B} Sorcery):
/// "Target player discards a card at random. Then that player discards another card at random unless they pay {1}."
#[test]
fn flay_makes_target_player_discard_two_cards_when_they_do_not_pay() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[flay()])
        .hand(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, flay());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
}

/// If the targeted player pays {1}, Flay only causes them to discard the first
/// card at random and spares the second.
#[test]
fn flay_makes_target_player_discard_only_one_card_when_they_pay_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[flay()])
        .battlefield(1, &[swamp()])
        .hand(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, flay());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let in_gy = usize::from(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some())
        + usize::from(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
    let in_hd = usize::from(in_hand(&engine, p1, index::GRIZZLY_BEARS).is_some())
        + usize::from(in_hand(&engine, p1, index::HILL_GIANT).is_some());
    assert_eq!(in_gy, 1);
    assert_eq!(in_hd, 1);
}

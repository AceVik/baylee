//! `cards/sorceries/mv_3/coercion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn coercion() -> CardIndex {
    card_index("68413337-ddb9-46f8-8c8e-f3d2d672c652")
}

/// Coercion ({2}{B} sorcery): "Target opponent reveals their hand. You choose a card from it.
/// That player discards that card." The caster, not the opponent, chooses, from the whole hand;
/// exactly the chosen card is discarded and the other stays in the hand.
#[test]
fn coercion_reveals_target_opponents_hand_and_discards_the_card_the_caster_chooses() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[coercion()])
        .hand(1, &[GRIZZLY_BEARS, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_bear = in_hand(&engine, p1, GRIZZLY_BEARS).unwrap();
    let opp_bolt = in_hand(&engine, p1, lightning_bolt()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, coercion());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p0, "the caster chooses, not the opponent");
    assert!(
        options.contains(&opp_bear) && options.contains(&opp_bolt),
        "any card of the revealed hand may be chosen"
    );
    assert_eq!((min, max), (1, 1), "exactly one card");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![opp_bolt],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, lightning_bolt()).is_some());
    assert!(
        in_hand(&engine, p1, GRIZZLY_BEARS).is_some(),
        "the card that was not chosen stays in the hand"
    );
    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_none());
}

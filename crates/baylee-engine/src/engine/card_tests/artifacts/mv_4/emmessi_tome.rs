//! `cards/artifacts/mv_4/emmessi_tome.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn emmessi_tome() -> CardIndex {
    card_index("9b7634bf-067b-4b5c-be72-ff1b80247969")
}

/// "{5}, {T}: Draw two cards, then discard a card." Two cards come in, and
/// the discard is the player's own choice from the enlarged hand.
#[test]
fn the_tome_draws_two_then_the_player_discards_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[emmessi_tome(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tome = on_battlefield(&engine, p0, emmessi_tome()).expect("out");
    let library = library_size(&engine, p0);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let graveyard = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, emmessi_tome(), 0);
    assert!(is_tapped(&engine, tome));
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
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1), "exactly one card");
    assert_eq!(
        options.len(),
        hand + 2,
        "the discard is chosen after both draws"
    );
    assert!(options.contains(&in_hand(&engine, p0, plains()).unwrap()));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a listed card");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), library - 2);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        graveyard + 1
    );
}

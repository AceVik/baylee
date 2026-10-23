//! `cards/artifacts/mv_3/horn_of_greed.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn horn_of_greed() -> CardIndex {
    card_index("b8181d53-1954-4f46-8670-8696440208e8")
}

/// Horn of Greed (`b8181d53-1954-4f46-8670-8696440208e8`) — {3} Artifact (Implemented):
/// "Whenever a player plays a land, that player draws a card."
#[test]
fn horn_of_greed_draws_a_card_when_a_player_plays_a_land() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[horn_of_greed()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    play_land(&mut engine, P0, forest());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before,
        "playing a land (-1) and drawing from Horn of Greed (+1) leaves hand size unchanged"
    );
}

#[test]
fn both_players_draw_only_for_land_plays_not_putting_a_land_onto_battlefield() {
    use baylee_core::generated::index;
    let mut e = lk::game(
        SEED,
        &[horn_of_greed()],
        &[],
        &[forest(), index::RAMPANT_GROWTH],
        &[island()],
    );
    let before = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    play_land(&mut e, P0, forest());
    lk::settle(&mut e);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), before);
    lk::float(&mut e, P0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, P0, index::RAMPANT_GROWTH);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    e.apply(
        player,
        PlayerAction::ChooseObjects {
            objects: vec![options[0]],
        },
    )
    .unwrap();
    lk::settle(&mut e);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        before - 1,
        "putting a land onto the battlefield is not playing it"
    );
    reach_their_main_phase(&mut e, lk::P1);
    let mine = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    let theirs = e.state().zones.list(ZoneLocation::Hand(lk::P1)).len();
    play_land(&mut e, lk::P1, island());
    lk::settle(&mut e);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(lk::P1)).len(),
        theirs
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), mine);
}

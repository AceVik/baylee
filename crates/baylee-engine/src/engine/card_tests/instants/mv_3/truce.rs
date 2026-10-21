//! Truce — `{2}{W}` Instant (Homelands)
//!
//! Each player may draw up to two cards. For each card less than two a player draws this way,
//! that player gains 2 life.

#[allow(clippy::wildcard_imports)]
use super::*;

fn truce() -> CardIndex {
    card_index("fe4aa72b-f176-461a-aca0-1519721609b2")
}

#[test]
fn truce_has_each_player_draw_two_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[truce()])
        .battlefield(0, &[plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let truce_card = in_hand(&engine, p0, truce()).unwrap();
    let p0_hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: truce_card })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        p0_hand_before - 1 + 2,
        "p0 cast Truce and drew 2 cards"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before + 2,
        "p1 drew 2 cards from Truce"
    );
}

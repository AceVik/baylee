//! `cards/lands/utility/sun_blessed_peak.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Sun-Blessed Peak` is a dual land under `Coverage::Implemented` that enters tapped, produces `{R}` or `{W}`, and sacrifices itself for four mana to draw a card.
/// Playing the land from hand puts it onto the battlefield in a tapped state.
/// On the following turn after untapping, activating its sacrifice ability pays `{4}`, moves the land to the graveyard, and draws a card.
#[test]
fn sun_blessed_peak_enters_tapped_and_sacrifices_to_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[sun_blessed_peak()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, sun_blessed_peak());
    assert!(
        entered_tapped(&engine, land),
        "Sun-Blessed Peak enters tapped"
    );

    // Advance to p0's next turn so the land untaps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, land),
        "Sun-Blessed Peak untaps on the next turn"
    );

    // Float {4} from Mountains without tapping Sun-Blessed Peak.
    tap_all_mana_but(&mut engine, p0, Some(sun_blessed_peak()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains produce four red mana"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "with {{4}} floating the sacrifice ability is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, sun_blessed_peak(), 1);

    assert!(
        on_battlefield(&engine, p0, sun_blessed_peak()).is_none(),
        "sacrificing the land is part of the cost, so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, sun_blessed_peak()).is_some(),
        "sacrificed land moved to graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{4}} was spent from the pool"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card was drawn from the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "hand size increased by one card"
    );
}

//! `cards/lands/manlands/lavaclaw_reaches.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lavaclaw Reaches enters tapped, prints `{{T}}: Add {{B}} or {{R}}`, and
/// `{1}{B}{R}: Until end of turn, this land becomes a 2/2 black and red
/// Elemental creature with "{X}: This creature gets +X/+0 until end of
/// turn." It's still a land.`
///
/// Under `Coverage::Partial`, the granted pump ability is dropped because a
/// granted ability is activated with X fixed at 0. This test plays the
/// land tapped, untaps on the following turn, pays `{1}{B}{R}` to animate it
/// into a 2/2 black and red Elemental creature that remains a land, and confirms
/// it offers only its printed abilities.
#[test]
fn lavaclaw_reaches_enters_tapped_and_animates_into_an_elemental() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        // Twice the {1}{B}{R}, for the same reason as Wandering Fumarole
        // below: the offer is read after the animation and `can_afford`
        // reads the pool, not the board.
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[lavaclaw_reaches()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, lavaclaw_reaches());
    assert!(entered_tapped(&engine, land), "enters tapped");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps in next untap step");

    tap_all_mana_but(&mut engine, p0, Some(lavaclaw_reaches()));
    activate(&mut engine, p0, lavaclaw_reaches(), 1);
    pass_until(&mut engine, stack_is_empty);

    let chars = engine
        .state()
        .object(land)
        .expect("land exists")
        .characteristics();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.types.contains(TypeSet::LAND), "it's still a land");
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ELEMENTAL)
    );
    assert!(chars.colors.contains(baylee_core::color::Color::Black));
    assert!(chars.colors.contains(baylee_core::color::Color::Red));
    assert_eq!(pt(&engine, land), (2, 2));

    // Three of each colour rather than two: the engine paid the generic
    // half of the first {1}{B}{R} out of a Swamp, so a board of exactly two
    // prices left black at nought and the second activation was unpayable
    // for a reason that had nothing to do with the card.
    let (black, red) = {
        let pool = &engine.state().players[0].mana_pool;
        (
            pool.available(ManaColor::Black),
            pool.available(ManaColor::Red),
        )
    };
    assert!(
        black >= 1 && red >= 1 && engine.state().players[0].mana_pool.total() >= 3,
        "a second {{1}}{{B}}{{R}} is still floating: B={black} R={red}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.abilities.iter().filter(|(id, _)| *id == land).count(),
        2,
        "only the printed mana and animate abilities, no granted +X/+0 ability"
    );
}

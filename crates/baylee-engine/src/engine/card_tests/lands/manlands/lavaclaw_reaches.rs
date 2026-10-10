//! `cards/lands/manlands/lavaclaw_reaches.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Animation adds creaturehood, colors and the Elemental subtype until end of turn.
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
        3,
        "the two printed abilities and the granted +X/+0 ability"
    );
}

#[test]
fn lavaclaw_granted_x_pump_announces_pays_and_expires() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut e = kit::game(981, &[lavaclaw_reaches()], &[], &[], &[]);
    let land = kit::obj(&e, kit::P0, lavaclaw_reaches());
    // The animation replaces an existing color rather than accumulating it.
    e.state.object_mut(land).unwrap().base_mut().colors =
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Green);
    kit::float(
        &mut e,
        kit::P0,
        &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 4),
        ],
    );
    kit::use_ability(&mut e, kit::P0, lavaclaw_reaches(), 1, &[], &[]);
    assert_eq!(pt(&e, land), (2, 2));
    assert!(
        !e.state
            .object(land)
            .unwrap()
            .characteristics()
            .colors
            .contains(baylee_core::color::Color::Green)
    );
    let ability = crate::choice::granted_ability(0);
    assert!(priority_offer(&e).abilities.contains(&(land, ability)));
    e.apply(
        kit::P0,
        PlayerAction::ActivateAbility {
            source: land,
            ability_index: ability,
        },
    )
    .unwrap();
    assert!(matches!(e.pending(), Pending::ChooseNumber { .. }));
    e.apply(kit::P0, PlayerAction::ChooseNumber(3)).unwrap();
    assert_eq!(kit::pool_total(&e, kit::P0), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, land), (5, 2));
    assert!(types(&e, land).contains(TypeSet::LAND));
    reach_their_main_phase(&mut e, kit::P1);
    assert!(!types(&e, land).contains(TypeSet::CREATURE));
    assert!(
        crate::effects::granted_activated(e.state(), land)
            .next()
            .is_none()
    );
}

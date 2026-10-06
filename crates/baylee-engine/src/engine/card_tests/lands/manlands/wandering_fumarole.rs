//! `cards/lands/manlands/wandering_fumarole.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wandering Fumarole enters tapped, prints `{{T}}: Add {{U}} or {{R}}`, and
/// `{2}{U}{R}: Until end of turn, this land becomes a 1/4 blue and red
/// Elemental creature with "{0}: Switch this creature's power and toughness
/// until end of turn." It's still a land.`
///
/// Under `Coverage::Partial`, the granted switch ability is unsupported and
/// omitted. This test plays the land tapped, untaps on the following turn,
/// animates it into a 1/4 blue and red Elemental creature that remains a land,
/// and verifies that no granted `{0}` ability is offered.
#[test]
fn wandering_fumarole_enters_tapped_and_animates_into_an_elemental() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        // Twice the {2}{U}{R} the animation costs: the offer read at the end
        // is filtered by `can_afford`, so a count taken on the pool the
        // activation emptied would be a fact about the price and not about
        // the abilities this land has.
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[wandering_fumarole()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, wandering_fumarole());
    assert!(entered_tapped(&engine, land), "enters tapped");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps on next turn");

    tap_all_mana_but(&mut engine, p0, Some(wandering_fumarole()));
    activate(&mut engine, p0, wandering_fumarole(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (1, 4));
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
    assert!(chars.colors.contains(baylee_core::color::Color::Blue));
    assert!(chars.colors.contains(baylee_core::color::Color::Red));

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the second {{2}}{{U}}{{R}} is still floating, so the count below is \
         about the abilities and not about the price"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.abilities.iter().filter(|(id, _)| *id == land).count(),
        3,
        "the two printed abilities and the granted {{0}} switch the animation \
         hands the land — this count was 2 for as long as the grant was dropped"
    );
}

/// Wandering Fumarole enters tapped, prints `{{T}}: Add {{U}} or {{R}}`, and
/// `{{2}}{{U}}{{R}}: Until end of turn, this land becomes a 1/4 blue and red Elemental creature with "{{0}}: Switch this creature's power and toughness until end of turn." It's still a land.`
/// Under `Coverage::Implemented`, the granted switch ability is implemented via `Modifier::GrantActivated`.
/// This test plays the land tapped, passes to the next turn so it untaps, floats `{{2}}{{U}}{{R}}` with `tap_all_mana_but`,
/// animates the land into a 1/4 blue and red Elemental land creature, and activates the granted `{{0}}` ability at
/// `crate::choice::GRANTED_ABILITY` to switch its power and toughness to 4/1 and back to 1/4.
#[test]
fn wandering_fumarole_enters_tapped_animates_and_switches_pt() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), mountain(), mountain()])
        .hand(0, &[wandering_fumarole()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, wandering_fumarole());
    assert!(
        entered_tapped(&engine, land),
        "Wandering Fumarole enters tapped"
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps on next turn");

    // Float {2}{U}{R} while keeping Wandering Fumarole untapped.
    tap_all_mana_but(&mut engine, p0, Some(wandering_fumarole()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2
    );

    // Activate ability 1 ({2}{U}{R} animation).
    activate(&mut engine, p0, wandering_fumarole(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (1, 4), "animated land is a 1/4");
    let chars = engine
        .state()
        .object(land)
        .expect("land exists")
        .characteristics();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.types.contains(TypeSet::LAND), "it is still a land");
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ELEMENTAL),
        "gained Elemental subtype"
    );
    assert!(chars.colors.contains(baylee_core::color::Color::Blue));
    assert!(chars.colors.contains(baylee_core::color::Color::Red));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal
            .abilities
            .iter()
            .any(|(id, index)| *id == land && *index == crate::choice::GRANTED_ABILITY),
        "the granted {{0}} switch ability is offered"
    );

    // Activate the granted {0} switch ability.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .expect("the switch ability activates");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, land),
        (4, 1),
        "power and toughness switched to 4/1"
    );

    // Activate the granted {0} switch ability again to switch back to 1/4.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .expect("the switch ability activates a second time");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, land),
        (1, 4),
        "power and toughness switched back to 1/4"
    );
}

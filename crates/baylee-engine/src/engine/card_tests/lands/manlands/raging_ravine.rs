//! `cards/lands/manlands/raging_ravine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Raging Ravine enters tapped, prints `{{T}}: Add {{R}} or {{G}}`, and
/// `{2}{R}{G}: Until end of turn, this land becomes a 3/3 red and green
/// Elemental creature with "Whenever this creature attacks, put a +1/+1
/// counter on it." It's still a land.`
///
/// Under `Coverage::Implemented`, all printed characteristics are fully
/// realized. This test plays the land tapped, untaps on the following turn,
/// animates it into a 3/3 red and green Elemental creature, attacks with it,
/// and asserts that its attack trigger resolves to place a +1/+1 counter on it.
#[test]
fn raging_ravine_enters_tapped_animates_and_triggers_on_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), forest(), forest()])
        .hand(0, &[raging_ravine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, raging_ravine());
    assert!(entered_tapped(&engine, land), "enters tapped");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps on next turn");

    tap_all_mana_but(&mut engine, p0, Some(raging_ravine()));
    activate(&mut engine, p0, raging_ravine(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (3, 3));
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
    assert!(chars.colors.contains(baylee_core::color::Color::Red));
    assert!(chars.colors.contains(baylee_core::color::Color::Green));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on ChooseAttackers")
    };
    assert!(attackers.contains(&land), "animated land may attack");
    let defender = defenders
        .into_iter()
        .next()
        .expect("the opponent is attackable");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(land, defender)],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, land, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, land), (4, 4));
}

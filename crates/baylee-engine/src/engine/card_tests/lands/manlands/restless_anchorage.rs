//! `cards/lands/manlands/restless_anchorage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Anchorage enters tapped, prints `{{T}}: Add {{W}} or {{U}}`,
/// `{1}{W}{U}: Until end of turn, this land becomes a 2/3 white and blue
/// Bird creature with flying. It's still a land.`, and `Whenever this land
/// attacks, create a Map token.`
///
/// Under `Coverage::Partial`, the attack trigger is omitted because Map tokens
/// are not modeled in the token definitions. This test plays the land tapped,
/// untaps on the following turn, animates it into a 2/3 flying Bird creature,
/// and verifies that attacking creates no trigger on the stack.
#[test]
fn restless_anchorage_enters_tapped_and_animates_into_a_bird() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), island()])
        .hand(0, &[restless_anchorage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_anchorage());
    assert!(entered_tapped(&engine, land), "enters tapped");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps on next turn");

    tap_all_mana_but(&mut engine, p0, Some(restless_anchorage()));
    activate(&mut engine, p0, restless_anchorage(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (2, 3));
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
            .contains(baylee_core::generated::subtypes::creature::BIRD)
    );
    assert!(chars.colors.contains(baylee_core::color::Color::White));
    assert!(chars.colors.contains(baylee_core::color::Color::Blue));
    assert!(keywords(&engine, land).contains(KeywordSet::FLYING));

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

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(
        stack_is_empty(&engine),
        "no Map token trigger is placed on the stack"
    );
}

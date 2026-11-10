//! `cards/lands/manlands/celestial_colonnade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Celestial Colonnade — "This land enters tapped", "{T}: Add {W} or {U}",
/// and "{3}{W}{U}: Until end of turn, this land becomes a 4/4 white and blue
/// Elemental creature with flying and vigilance. It's still a land."
///
/// The land arrives through a real `PlayLand` rather than a seeded board, so
/// the tapped entry is the replacement effect being read — a
/// `starting_battlefield` placement is no entry at all and would come in
/// standing. The animation then waits a turn: an animated manland is a
/// creature with summoning sickness in its arrival turn (CR 302.6), and the
/// six basics pay {3}{W}{U} with the Colonnade named as the source to leave
/// standing, because it animates without tapping and a manland tapped for its
/// own mana could not attack for four. The attack is what separates a
/// projected 4/4 from a creature, and that it is still standing afterwards is
/// the vigilance the same printed line grants.
#[test]
fn celestial_colonnade_enters_tapped_and_wakes_as_a_four_four_vigilant_flier() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(238, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), island(), island(), island()],
        )
        .hand(0, &[celestial_colonnade()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let colonnade = play_land(&mut engine, p0, celestial_colonnade());
    assert!(
        entered_tapped(&engine, colonnade),
        "\"This land enters tapped\""
    );

    // A land that arrives tapped gives nothing this turn, and an animated one
    // is summoning sick in the turn it arrived: the whole card is played on
    // the next turn, where neither is in the way.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "and again on its controller's next turn"
    );
    assert!(
        !is_tapped(&engine, colonnade),
        "the untap step stood it back up"
    );

    // {3}{W}{U} out of the six basics, with the Colonnade kept back.
    tap_all_mana_but(&mut engine, p0, Some(celestial_colonnade()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "three Plains and three Islands, and not the Colonnade"
    );

    // Ability 0 is the printed mana ability; ability 1 is the animation.
    activate(&mut engine, p0, celestial_colonnade(), 1);
    assert!(
        !stack_is_empty(&engine),
        "the animation is no mana ability, so it waits on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    let kinds = types(&engine, colonnade);
    assert!(
        kinds.contains(TypeSet::CREATURE) && kinds.contains(TypeSet::LAND),
        "\"It's still a land\" — the creature type is added, not swapped: {kinds:?}"
    );
    assert_eq!(pt(&engine, colonnade), (4, 4), "the printed 4/4 body");
    let granted = keywords(&engine, colonnade);
    assert!(granted.contains(KeywordSet::FLYING), "and flying");
    assert!(granted.contains(KeywordSet::VIGILANCE), "and vigilance");

    // The body the combat step itself offers, and the keyword read off the
    // battlefield rather than off the card: it attacked and never tapped.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&colonnade),
        "an animated Colonnade may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(colonnade, Defender::Player(p1))],
            },
        )
        .unwrap();
    // Not `stack_is_empty`: the stack is already empty the moment attackers
    // are declared, and the end step is past combat damage (CR 510.2).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        16,
        "a 4/4 flier nobody blocked deals four"
    );
    assert!(
        !is_tapped(&engine, colonnade),
        "\"with flying and vigilance\" — attacking did not tap it"
    );
}

//! `cards/instants/mv_1/sandstorm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sandstorm — {G} instant: "Sandstorm deals 1 damage to each attacking
/// creature."
///
/// The board walks into a real combat, because "attacking" is a state the
/// declaration creates (CR 508.1): p1 attacks with a 1/1 and a 1/3 and p0
/// answers in the declare attackers step, at their own priority window after
/// the declaration (CR 508.2), with no blocker yet declared. The 1/1 dies
/// (CR 704.5g); the 1/3 survives with the point marked; p0's untapped 2/2 —
/// a creature that could have blocked but is not attacking — takes nothing.
///
/// The journal is read beside the board, because marked damage alone cannot
/// tell two events of one from one event of two: exactly two `DamageDealt`
/// events leave the spell, one at each attacker, and neither is combat
/// damage nor points at the defender.
#[test]
fn sandstorm_deals_one_damage_to_each_attacking_creature_and_not_the_defender() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), grizzly_bears()])
        .hand(0, &[sandstorm()])
        .battlefield(1, &[llanowar_elves(), canopy_spider()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1), "p1 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the 1/1 attacker");
    let spider = on_battlefield(&engine, p1, canopy_spider()).expect("the 1/3 attacker");
    let defender = on_battlefield(&engine, p0, grizzly_bears()).expect("p0's creature");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p0)), (spider, Defender::Player(p0))],
            },
        )
        .expect("both creatures attack p0");

    // The active player's priority after the declaration is passed; the
    // defending player's own window is where an instant answers a declared
    // attack, with both creatures attacking and no block declared.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is untapped");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .expect("a Forest pays {G}");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green floats for the spell and nothing else is on the board for it"
    );

    let before = engine.journal().entries().len();
    let storm = in_hand(&engine, p0, sandstorm()).expect("Sandstorm is in hand");
    cast_with_floating(&mut engine, p0, sandstorm());
    pass_until(&mut engine, stack_is_empty);

    let events = damage_events(&engine, before);
    assert_eq!(
        events.len(),
        2,
        "one event per attacking creature: {events:?}"
    );
    for (target, label) in [(elf, "the attacking 1/1"), (spider, "the attacking 1/3")] {
        assert!(
            events.contains(&(storm, crate::event::DamageTarget::Object(target), 1, false)),
            "{label} was dealt exactly the one non-combat damage: {events:?}"
        );
    }
    assert!(
        !events
            .iter()
            .any(|(_, target, _, _)| *target == crate::event::DamageTarget::Object(defender)),
        "the defender is not attacking, and no event reaches it: {events:?}"
    );

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the 1/1 took 1 damage and died (CR 704.5g)"
    );
    assert_eq!(
        engine
            .state()
            .object(spider)
            .expect("the 1/3 survived")
            .damage,
        1,
        "the 1/3 survives with the single point marked"
    );
    assert_eq!(
        engine
            .state()
            .object(defender)
            .expect("p0's creature is still there")
            .damage,
        0,
        "the defending creature was never dealt anything"
    );
    assert_eq!(
        pt(&engine, defender),
        (2, 2),
        "and is otherwise untouched: \"each attacking creature\" is a set and \
         not the whole board"
    );
}

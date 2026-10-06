//! `cards/creatures/mv_3/werefox_bodyguard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Werefox Bodyguard: "When this creature enters, exile up to one other
/// target non-Fox creature until this creature leaves the battlefield."
///
/// The return is not a triggered ability. CR 610.3 makes it a second
/// one-shot effect, created "immediately after the specified event", so
/// nothing happens between the Bodyguard leaving and the Elves coming back:
/// they are on the battlefield again the moment the Bodyguard is sacrificed
/// to pay for its own last ability, while that ability, the only object on
/// the stack, has not resolved. They come back under their owner's control
/// (CR 610.3c), as a new object (CR 400.7).
///
/// The card exiled its target with nothing to bring it back, and the Elves
/// stayed in exile for the rest of the game.
#[test]
fn werefox_bodyguard_holds_a_creature_until_it_leaves_the_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, elves, spare) = a_bodyguard_aimed_at_their_elves(614);
    pass_until(&mut engine, stack_is_empty);
    let exiled = engine.state().object(elves).expect("the Elves");
    assert_eq!(exiled.zone, Zone::Exile, "exiled by the enters trigger");
    let version = exiled.version;
    assert!(
        on_battlefield(&engine, p0, werefox_bodyguard()).is_some(),
        "and held for as long as the Bodyguard stays"
    );

    // {1}{W}, Sacrifice this creature: You gain 2 life.
    let life = engine.state().players[0].life;
    tap_mana_where(&mut engine, p0, |id| spare.contains(&id));
    activate(&mut engine, p0, werefox_bodyguard(), 1);
    assert!(
        in_graveyard(&engine, p0, werefox_bodyguard()).is_some(),
        "sacrificed to pay the cost"
    );
    let stack = engine.state().zones.list(ZoneLocation::Stack).clone();
    assert_eq!(
        stack.len(),
        1,
        "the life-gain ability and nothing else: the return is no ability \
         and uses no stack: {stack:?}"
    );
    let back = engine.state().object(elves).expect("the Elves");
    assert_eq!(
        back.zone,
        Zone::Battlefield,
        "back before the ability the sacrifice paid for has resolved"
    );
    assert_eq!(
        (back.owner, back.controller, back.base_controller),
        (p1, p1, p1),
        "under their owner's control (CR 610.3c)"
    );
    assert_ne!(back.version, version, "a new object (CR 400.7)");
    assert!(
        !back
            .riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::Linked { .. })),
        "and linked to nothing any more"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, life + 2);
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_some());
}

/// The same Bodyguard, sacrificed while its enters trigger is still on the
/// stack. The event the exile lasts until has already happened when the
/// exile would, so the Elves do not move at all (CR 610.3b). The card
/// exiled them anyway, and with nothing left to bring them back, for good.
#[test]
fn werefox_bodyguard_gone_before_its_trigger_resolves_exiles_nothing() {
    let p0 = PlayerId::new(0);
    let (mut engine, elves, spare) = a_bodyguard_aimed_at_their_elves(615);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the trigger waits on the stack and its controller holds priority: {:?}",
        engine.pending()
    );
    assert!(
        !stack_is_empty(&engine),
        "the enters trigger is on the stack"
    );
    let version = engine.state().object(elves).expect("the Elves").version;

    let life = engine.state().players[0].life;
    tap_mana_where(&mut engine, p0, |id| spare.contains(&id));
    activate(&mut engine, p0, werefox_bodyguard(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, life + 2, "both resolved");
    let obj = engine.state().object(elves).expect("the Elves");
    assert_eq!(obj.zone, Zone::Battlefield, "never exiled");
    assert_eq!(obj.version, version, "never moved: the same object");
}

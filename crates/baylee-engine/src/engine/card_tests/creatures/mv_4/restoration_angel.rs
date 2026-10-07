//! `cards/creatures/mv_4/restoration_angel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restoration Angel: "When this creature enters, you may exile target
/// non-Angel creature you control, then return that card to the battlefield
/// under your control."
///
/// The owner played it on a creature they had stolen, and the creature went
/// home. It must not: the card that returns is a new object (CR 400.7), no
/// control effect reaches it, and it enters under the control the sentence
/// names — "your", the Angel's controller's (CR 110.2a). The engine sent
/// every blinked card to its owner, reading CR 610.3c, which is about a card
/// that comes back after an "until" event.
///
/// And it changes **control only**. Seat 1 still owns the Elves (CR 108.3),
/// so Aminatou's "another target permanent you own" does not offer them to
/// seat 0, and when they die they go to seat 1's graveyard (CR 400.3).
///
/// The steal is Song-Mad Treachery's, until end of turn, and the test walks
/// past that end into seat 1's turn before it looks again: a creature kept
/// only by the Treachery would have gone back in the cleanup step.
#[test]
#[allow(clippy::too_many_lines)] // one game across two turns, told in order
fn restoration_angel_keeps_a_stolen_creature_and_its_owner_keeps_owning_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(612, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
                aminatou(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), mountain()])
        .hand(0, &[song_mad_treachery(), restoration_angel()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("p1's Elves");
    steal_with_song_mad_treachery(&mut engine, p0, elves);

    // The Angel, off the four Plains.
    let plains_ids: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == plains()))
        })
        .collect();
    tap_mana_where(&mut engine, p0, |id| plains_ids.contains(&id));
    cast_with_floating(&mut engine, p0, restoration_angel());
    let options = pass_until_targets(&mut engine, p0);
    let angel = on_battlefield(&engine, p0, restoration_angel()).expect("the Angel entered");
    assert!(
        options.contains(&elves),
        "a stolen creature is a non-Angel creature you control: {options:?}"
    );
    assert!(!options.contains(&angel), "not an Angel: {options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("the Elves are targeted");
    pass_until(&mut engine, stack_is_empty);

    let obj = engine.state().object(elves).expect("the Elves came back");
    assert_eq!(obj.zone, Zone::Battlefield, "exiled and returned");
    assert_eq!(
        (obj.controller, obj.base_controller),
        (p0, p0),
        "returned under the Angel's controller's control, not its owner's"
    );
    assert_eq!(obj.owner, p1, "control changed and ownership did not");

    // "Another target permanent you own" reads the owner: the Elves are
    // seat 0's to use and not seat 0's to flicker with Aminatou.
    let plains = on_battlefield(&engine, p0, plains()).expect("a Plains seat 0 owns");
    activate(&mut engine, p0, aminatou(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Aminatou's −1 asks for a target: {:?}", engine.pending())
    };
    assert!(
        !options.contains(&elves),
        "the Elves are not a permanent seat 0 owns: {options:?}"
    );
    assert!(options.contains(&plains), "a Plains is: {options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![plains],
                players: vec![],
            },
        )
        .expect("the Plains is targeted");
    pass_until(&mut engine, stack_is_empty);

    // Past the Treachery's end of turn and into seat 1's: nothing holds the
    // Elves any more, and seat 0 still controls them.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !engine
            .state()
            .effects
            .iter()
            .any(|fx| fx.modifier == baylee_cards_dsl::Modifier::GainControl),
        "the Treachery's control effect has ended"
    );
    let obj = engine
        .state()
        .object(elves)
        .expect("still on the battlefield");
    assert_eq!((obj.controller, obj.owner), (p0, p1));

    // Seat 1 bolts them, and they die into seat 1's graveyard.
    cast_from_hand(&mut engine, p1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Bolt asks for a target: {:?}", engine.pending())
    };
    assert!(options.contains(&elves));
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("the Elves are targeted");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "a creature dies into its owner's graveyard"
    );
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_none());
}

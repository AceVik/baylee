//! `cards/instants/mv_1/ephemerate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ephemerate: "Exile target creature you control, then return it to the
/// battlefield under its owner's control."
///
/// The other half of what Restoration Angel's test holds. The same stolen
/// Elves, the same Song-Mad Treachery, and the sentence names the owner, so
/// the new object enters under seat 1's control (CR 400.7) and seat 0's
/// borrowed creature goes home at once rather than at end of turn.
#[test]
fn ephemerate_returns_a_stolen_creature_to_its_owner() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(613, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                plains(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[song_mad_treachery(), ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("p1's Elves");
    steal_with_song_mad_treachery(&mut engine, p0, elves);

    cast_from_hand(&mut engine, p0, ephemerate());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Ephemerate asks for a target: {:?}", engine.pending())
    };
    assert!(
        options.contains(&elves),
        "a stolen creature is a creature you control: {options:?}"
    );
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
        (obj.owner, obj.controller, obj.base_controller),
        (p1, p1, p1),
        "returned under its owner's control"
    );
}

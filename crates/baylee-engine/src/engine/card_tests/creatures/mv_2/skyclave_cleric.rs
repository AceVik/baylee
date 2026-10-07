//! `cards/creatures/mv_2/skyclave_cleric.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The creature front: "When this creature enters, you gain 2 life."
///
/// Cast off two Plains, which is the printed `{1}{W}` exactly, so nothing but
/// the card's own sentence can move a life total on this board. Three things
/// are asserted because there are three ways to be wrong: the card arrives as
/// **face 0** — `abilities_for_face` falls back to the card-level list on
/// face 0 alone, so a card that had landed on its land back would carry no
/// trigger at all — the controller gains 2 and not some other number, and the
/// opponent gains nothing, because "you gain 2 life" names the player who
/// controls the ability and not the table.
#[test]
fn skyclave_clerics_creature_front_gains_its_controller_two_life_on_entry() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[skyclave_cleric()])
        .start();
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    let mine_before = engine.state().players[0].life;
    let theirs_before = engine.state().players[1].life;
    cast_from_hand(&mut engine, p0, skyclave_cleric());
    // The trigger goes on the stack as the creature enters, so "the stack is
    // empty again *and* the life total moved" is the moment it has resolved.
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && e.state().players[0].life > mine_before
    });

    let cleric = on_battlefield(&engine, p0, skyclave_cleric()).expect("the Cleric landed");
    let obj = engine.state().object(cleric).expect("it is still there");
    assert_eq!(obj.face_index, 0, "a cast takes the creature front");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Skyclave Cleric"
    );
    assert_eq!(pt(&engine, cleric), (1, 3), "the printed 1/3 stands there");
    assert_eq!(
        engine.state().players[0].life,
        mine_before + 2,
        "the enters-trigger gained exactly the 2 life the card prints",
    );
    assert_eq!(
        engine.state().players[1].life,
        theirs_before,
        "\"you gain 2 life\" is the controller and nobody else",
    );
}

/// The land back: "This land enters tapped." and "{T}: Add {W}."
///
/// Only one of the two faces is a land, so this card takes the Glasspool
/// Shore path and not the pathway one: the engine switches to face 1 itself
/// and asks no face question, there being only one face to play (CR 712.12).
/// Four printed sentences are then held against the board — it arrives as the
/// *land* Skyclave Basilica and not as a creature, it arrives tapped, it
/// triggers nothing (a back face carries only the abilities it prints, and
/// the enters-trigger is printed on the face that stayed down), and once it
/// untaps it makes exactly one white mana and nothing else.
///
/// The third of those is asserted on the **stack** and not on a life total
/// alone. A card-level trigger wrongly queued for this face would be sitting
/// on the stack unresolved the instant the land drop hands priority back, and
/// both players are on 20 life either way; a test that only read the total
/// there would pass against an engine that had fired it.
#[test]
fn skyclave_basilica_is_the_land_back_played_without_a_face_question_entering_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, forest())
        .hand(0, &[skyclave_cleric()])
        .start();
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, skyclave_cleric());
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "one land face, so no face question is asked: {:?}",
        engine.pending()
    );
    // The stack and not the life total is what says the trigger did not fire:
    // a trigger queued by this entry would be sitting here unresolved, and a
    // life total read at this instant is still 20 either way.
    assert!(
        stack_is_empty(&engine),
        "the back face prints no trigger, so the land drop put nothing on the stack",
    );

    let obj = engine.state().object(land).expect("it is on the table");
    assert_eq!(obj.zone, Zone::Battlefield, "the land drop landed");
    assert_eq!(obj.face_index, 1, "it is on the table as its land back");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Skyclave Basilica"
    );
    assert!(types(&engine, land).contains(TypeSet::LAND));
    assert!(
        !types(&engine, land).contains(TypeSet::CREATURE),
        "the creature front is not what was played",
    );
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" is the back face's own modifier",
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "the Cleric's enters-trigger is printed on the face that stayed down",
    );

    // Tapped, so it pays for nothing this turn; untapped next turn it is the
    // only mana source p0 has, which is what makes the pool a measurement.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a land drop hands priority back")
    };
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "a land that entered tapped cannot pay {{T}} the turn it arrived",
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn comes back round and the untap step frees the land"
    );
    assert!(!is_tapped(&engine, land), "it untapped");
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "and two turns of resolutions later still no life has been gained",
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("{T}: Add {W}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the Basilica taps for exactly one white",
    );
    assert_eq!(pool.total(), 1, "and for nothing else besides");
}

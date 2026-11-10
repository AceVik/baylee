//! `cards/enchantments/mv_3/blanket_of_night.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blanket of Night — {1}{B}{B} enchantment: "Each land is a Swamp in
/// addition to its other land types."
///
/// A Forest is the reading no board can give by accident: left alone it taps
/// for {G} and for nothing else, so black mana out of one can only be the
/// Swamp the Blanket added — and the question asked on the way is **two**
/// colours wide, which is the "in addition to" half, since a Forest that had
/// merely *become* a Swamp would offer black alone. The Forest across the
/// table is the word "each": it is not a land this seat controls, and it is
/// asked the same two-colour question on p1's own turn.
#[test]
fn blanket_of_night_makes_a_forest_a_swamp_on_both_sides_of_the_table() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), forest()])
        .battlefield(1, &[forest()])
        .hand(0, &[blanket_of_night()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, forest()).expect("my Forest is out");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    // Three Swamps pay {1}{B}{B}, and the Forest is named as the thing kept
    // back: it is the permanent whose tap this test goes on to read.
    tap_all_mana_but(&mut engine, p0, Some(forest()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Swamps, three black, and the Forest contributed nothing"
    );
    cast_with_floating(&mut engine, p0, blanket_of_night());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, blanket_of_night()).is_some(),
        "the Blanket resolved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and its {{1}}{{B}}{{B}} came out of the pool"
    );
    assert!(!is_tapped(&engine, mine), "the Forest is still standing");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(
        !legal.mana_abilities.contains(&mine),
        "the intrinsic entry already offers both basic types: {:?}",
        legal.mana_abilities
    );

    // The intrinsic entry groups the two basic types into a colour choice,
    // preserving the Forest's green beside the Swamp the Blanket added.
    assert!(
        legal.abilities.contains(&(mine, 0)),
        "the intrinsic entry is offered by index: {:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mine,
                ability_index: 0,
            },
        )
        .expect("the tap the offer named is the one it pays");
    let Pending::ChooseColor { options, .. } = engine.pending() else {
        panic!("the two basic types ask which colour")
    };
    assert_eq!(options, &[ManaColor::Black, ManaColor::Green]);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("the added Swamp's colour is offered");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "black mana out of a Forest is the Swamp the Blanket added, and \
         nothing else on this board could have made it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one tap, one mana"
    );
    assert!(is_tapped(&engine, mine), "and it cost the Forest its tap");

    // "Each land" and not "each land you control": the Forest across the
    // table answers the same two-colour question on p1's own turn.
    reach_their_main_phase(&mut engine, p1);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("p1 holds its own main phase: {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: theirs,
                ability_index: 0,
            },
        )
        .expect("their Forest is offered the same tap");
    let Pending::ChooseColor { options, .. } = engine.pending() else {
        panic!("the opponent's Forest asks the same colour question")
    };
    assert_eq!(options, &[ManaColor::Black, ManaColor::Green]);
    engine
        .apply(p1, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("the opponent's added Swamp colour is offered");
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "\"each land\": the opponent's Forest is a Swamp as well"
    );
}

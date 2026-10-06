//! `cards/lands/vesuva.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vesuva: "You may have this land enter tapped as a copy of any land on the
/// battlefield." The copy is what this plays, and the card is
/// `Coverage::Partial` for the "tapped" half — `AbilityDef::CopyOnEnter`
/// carries no enter modifier, so what arrives is the copy untapped.
///
/// A **Forest** is the land to copy, because the copy has to be readable
/// from two sides that a wrong answer separates: the types it now has, and
/// the mana its intrinsic ability makes (CR 305.6, which is a land's type
/// line and not an ability anybody copied). A Vesuva that took the name and
/// not the type line would tap for nothing.
#[test]
fn vesuva_enters_as_a_copy_of_a_land_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[vesuva()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let model = on_battlefield(&engine, p0, forest()).expect("a Forest to copy");
    engine
        .apply(
            p0,
            PlayerAction::PlayLand {
                card: in_hand(&engine, p0, vesuva()).expect("Vesuva is in hand"),
            },
        )
        .expect("a land drop");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the copy asks which land, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&model),
        "\"any land on the battlefield\" — the Forest is one"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![model],
                players: vec![],
            },
        )
        .expect("a land the ability offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    // The object is still Vesuva's card — a copy effect changes an object's
    // characteristics and not which card it came off (CR 707.2), so it is
    // found by its own index and read through the layer system.
    let copy = on_battlefield(&engine, p0, vesuva()).expect("Vesuva is on the battlefield");
    assert_ne!(copy, model, "and it is not the Forest it copied");
    let chars = engine
        .state()
        .object(copy)
        .expect("the copy exists")
        .characteristics();
    assert_eq!(
        engine.state().names.get(chars.name),
        "Forest",
        "a copy takes the copiable values, name included (CR 707.2)"
    );

    assert_eq!(
        types(&engine, copy),
        types(&engine, model),
        "and the whole type line with it"
    );

    activate(&mut engine, p0, vesuva(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "and taps for green off the type line it copied (CR 305.6)"
    );
}

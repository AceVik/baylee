//! `cards/creatures/mv_1/rogue_elephant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rogue Elephant — {G}, 3/3: "When this creature enters, sacrifice it unless
/// you sacrifice a Forest."
///
/// The word that pays is the subtype, so the two boards differ by exactly
/// that. With a Forest under the same seat the entry question has a price:
/// paying it leaves a 3/3 standing and puts the Forest in the graveyard. With
/// the {G} coming off a Llanowar Elves instead there is no Forest on the
/// table, so no price is payable at all and the Elephant puts itself into its
/// owner's graveyard — the Elves being the control, a green source that is no
/// Forest, so a reading of "a Forest" as "any green permanent" would have kept
/// the Elephant alive there too.
#[test]
fn rogue_elephant_eats_a_forest_to_stay_and_dies_when_there_is_none() {
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(64, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[rogue_elephant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Forest is the whole board and it pays the {{G}}"
    );
    cast_with_floating(&mut engine, p0, rogue_elephant());
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the entry question is answerable with a Forest on the table, so it \
         is asked and answered out of what it enumerated"
    );

    let elephant =
        on_battlefield(&engine, p0, rogue_elephant()).expect("the Elephant paid and stayed");
    assert_eq!(pt(&engine, elephant), (3, 3), "the body the card prints");
    assert_eq!(
        in_graveyard(&engine, p0, forest()),
        Some(land),
        "the very Forest the {{G}} came from is what the Elephant ate, tapped \
         and all"
    );

    let mut engine = Duel::new(64, forest())
        .battlefield(0, &[llanowar_elves()])
        .hand(0, &[rogue_elephant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let _elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana off the Elves, which is a green source and no Forest"
    );
    cast_with_floating(&mut engine, p0, rogue_elephant());
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "no Forest, no price: the trigger resolves itself"
    );

    assert!(
        on_battlefield(&engine, p0, rogue_elephant()).is_none(),
        "\"sacrifice it unless you sacrifice a Forest\" — with no Forest the \
         Elephant is what goes"
    );
    assert!(
        in_graveyard(&engine, p0, rogue_elephant()).is_some(),
        "and a sacrifice puts it in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elves are a creature and no land, so they were never a legal \
         price: the trigger still eats the Elephant and leaves them"
    );
}

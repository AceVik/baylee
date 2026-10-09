//! `cards/sorceries/mv_2/regrowth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Regrowth: "Return target card from your graveyard to your hand." Read
/// off an instant, not a creature — "any card" is the whole point beside
/// Raise Dead.
#[test]
fn regrowth_returns_any_card_from_the_graveyard_not_only_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[regrowth(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p0, lightning_bolt());
    let bolt_in_gy = in_graveyard(&engine, p0, lightning_bolt()).expect("in the graveyard");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, regrowth());
    aim_at(&mut engine, p0, bolt_in_gy);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, lightning_bolt()).is_some(),
        "\"any card\" — an instant, not only a creature"
    );
}

/// "Target card from **your** graveyard": a land card comes back too (any
/// card), the opponent's graveyard is not on the menu, and with only the
/// opponent's graveyard filled the spell is withheld.
#[test]
fn regrowth_takes_a_land_but_never_a_card_from_the_opponents_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[regrowth(), plains(), lightning_bolt()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p1, llanowar_elves());
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, regrowth()).expect("in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&spell),
        "our graveyard is empty and theirs does not count"
    );

    let land = hand_to_graveyard(&mut engine, p0, plains());
    let bolt = hand_to_graveyard(&mut engine, p0, lightning_bolt());
    let theirs = in_graveyard(&engine, p1, llanowar_elves()).expect("their card");
    cast_with_floating(&mut engine, p0, regrowth());
    let mut menu = aim_at(&mut engine, p0, land);
    menu.sort();
    let mut ours = vec![land, bolt];
    ours.sort();
    assert_eq!(menu, ours, "exactly our graveyard, whatever the card");
    assert!(!menu.contains(&theirs));
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, plains()).is_some(),
        "the land card is back in hand"
    );
    assert!(in_graveyard(&engine, p0, lightning_bolt()).is_some());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
}

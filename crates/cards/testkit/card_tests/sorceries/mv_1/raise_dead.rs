//! `cards/sorceries/mv_1/raise_dead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Raise Dead: "Return target creature card from your graveyard to your
/// hand." Hand, not the battlefield — the whole difference from
/// Resurrection.
#[test]
fn raise_dead_returns_a_creature_card_from_the_graveyard_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[raise_dead(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p0, quiet_creature());
    let elf_in_gy = in_graveyard(&engine, p0, quiet_creature()).expect("in the graveyard");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, raise_dead());
    aim_at(&mut engine, p0, elf_in_gy);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, quiet_creature()).is_some(),
        "\"to your hand\""
    );
    assert!(in_graveyard(&engine, p0, quiet_creature()).is_none());
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "hand, not the battlefield"
    );
}

/// "Target creature card from **your** graveyard": a creature card in the
/// opponent's graveyard and a non-creature card in ours are both refused. With
/// only those two around the spell is not even castable (mana floating, so it
/// is the missing target that withholds it); once a creature card of ours is
/// there the menu holds exactly that one.
#[test]
fn raise_dead_refuses_the_opponents_graveyard_and_a_card_that_is_no_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[raise_dead(), lightning_bolt(), quiet_creature()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p0, lightning_bolt());
    hand_to_graveyard(&mut engine, p1, llanowar_elves());
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, raise_dead()).expect("in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&spell),
        "no creature card in our graveyard: Raise Dead has no target and is withheld"
    );

    let ours = hand_to_graveyard(&mut engine, p0, quiet_creature());
    let theirs = in_graveyard(&engine, p1, llanowar_elves()).expect("their creature card");
    let bolt = in_graveyard(&engine, p0, lightning_bolt()).expect("our instant");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "a creature card of ours is a target: castable"
    );
    cast_with_floating(&mut engine, p0, raise_dead());
    let menu = aim_at(&mut engine, p0, ours);
    assert_eq!(menu, vec![ours], "only our creature card: {menu:?}");
    assert!(!menu.contains(&theirs) && !menu.contains(&bolt));
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, quiet_creature()).is_some());
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the opponent's graveyard is untouched"
    );
}

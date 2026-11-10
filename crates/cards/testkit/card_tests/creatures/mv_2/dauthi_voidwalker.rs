//! `cards/creatures/mv_2/dauthi_voidwalker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "{T}, Sacrifice Dauthi Voidwalker: Choose an exiled card an opponent owns
/// with a void counter on it. You may play it this turn without paying its
/// mana cost." — the opponent's Elf, exiled by the Voidwalker's own
/// replacement, is cast from their exile by a seat with no land at all, and
/// arrives under that seat's control.
#[test]
fn dauthi_voidwalker_casts_an_opponents_voided_card_for_free() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dauthi_voidwalker()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    bury(&mut engine, &[elf]);
    engine.refresh_offer();
    let elf = voided(&engine, p1, llanowar_elves());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&elf),
        "a voided card is not castable before the Voidwalker says so"
    );

    void_walk(&mut engine, p0, elf);
    assert!(
        in_graveyard(&engine, p0, dauthi_voidwalker()).is_some(),
        "the Voidwalker was sacrificed"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&elf),
        "castable from the owner's exile with no mana at all"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: elf })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elf is the caster's"
    );
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
}

/// "Play" covers a land: the opponent's voided Forest is played from their
/// exile as the seat's land drop for the turn (CR 305.2).
#[test]
fn dauthi_voidwalker_plays_an_opponents_voided_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[dauthi_voidwalker()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p1, forest()).unwrap();
    bury(&mut engine, &[land]);
    engine.refresh_offer();
    let land = voided(&engine, p1, forest());

    void_walk(&mut engine, p0, land);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(legal.lands.contains(&land), "offered as a land to play");
    assert!(!legal.castable.contains(&land), "and not as a spell");
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .unwrap();
    assert!(on_battlefield(&engine, p0, forest()).is_some());
    assert_eq!(
        engine.state().players[0].lands_played_this_turn,
        1,
        "it was the turn's land drop"
    );
}

/// "This turn": a permission not used by the end of the turn is gone, and
/// the voided card stays in its owner's exile.
#[test]
fn dauthi_voidwalkers_permission_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dauthi_voidwalker()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    bury(&mut engine, &[elf]);
    engine.refresh_offer();
    let elf = voided(&engine, p1, llanowar_elves());
    void_walk(&mut engine, p0, elf);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(!legal.castable.contains(&elf), "the permission lapsed");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&elf)
    );
}

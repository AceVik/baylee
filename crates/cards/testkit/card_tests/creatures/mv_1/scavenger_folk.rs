//! `cards/creatures/mv_1/scavenger_folk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scavenger Folk — {G}, a 1/1 Human with "{G}, {T}, Sacrifice this creature:
/// Destroy target artifact."
///
/// Three parts of one price, and the engine shows each in its own place. The
/// target question is the CR 601.2c half, so while it stands the 1/1 is still
/// untapped, still on the battlefield and the {G} is still in the pool — the
/// cost is CR 601.2h and comes last, which is what the assertions inside the
/// question are for. Answering it aims the destruction across the table while
/// this seat's own Sol Ring, named on the same menu, stays standing: "target
/// artifact" is one permanent and not a sweep, and the Elf beside it — the
/// only non-artifact permanent on the board — is never on the menu at all.
/// The Folk itself ends up in a graveyard, which is a sacrifice paid rather
/// than the destroy resolving against its own source.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn scavenger_folk_sacrifices_itself_to_destroy_the_artifact_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), scavenger_folk(), quiet_artifact()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let folk = on_battlefield(&engine, p0, scavenger_folk()).expect("the Folk is out");
    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // The Forest alone: the Sol Ring beside it is a permanent this test reads
    // the menu off, and `tap_all_mana` would have tapped it for {C}{C} with
    // the mana ability it prints (#159).
    tap_mana_except(&mut engine, p0, mine);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one Forest is one {{G}}, and that is the whole mana half of the price"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(folk, 0)),
        "{{G}}, {{T}} and itself are all payable, so its one line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, scavenger_folk(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("destroy asks what it destroys, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "the seat that activated chooses");
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert!(
        options.contains(&theirs),
        "\"target artifact\" reaches across the table: {options:?}"
    );
    assert!(
        options.contains(&mine),
        "and this seat's own artifact sits on the same menu: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the Elf is a creature and no artifact — the filter is read, not skipped: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    // CR 601.2c picks the target and CR 601.2h pays afterwards: while the
    // question stands, nothing has been tapped, eaten or spent.
    assert!(
        on_battlefield(&engine, p0, scavenger_folk()).is_some(),
        "the Folk has not paid its own price yet"
    );
    assert!(!is_tapped(&engine, folk), "so it is still untapped");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "and the {{G}} is still floating, unspent"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the artifact across the table was one of the options");
    assert!(matches!(drive_to_rest(&mut engine, p0), Rest::Reached));

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the artifact that was named is destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "a destroyed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "and only the artifact that was named: this seat's Sol Ring still stands"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "which is also true of the one non-artifact permanent on the board"
    );
    assert!(
        on_battlefield(&engine, p0, scavenger_folk()).is_none(),
        "the Folk left the battlefield to pay"
    );
    assert!(
        in_graveyard(&engine, p0, scavenger_folk()).is_some(),
        "and it is in its own graveyard — a sacrifice paid, not the destroy \
         resolving against its source"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{G}} was the mana half of the price and it is spent"
    );
}

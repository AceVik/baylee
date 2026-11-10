//! `cards/instants/mv_2/boomerang.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Boomerang — {U}{U} instant: "Return target permanent to its owner's hand."
///
/// The two words that carry the card are "permanent" and "owner's", so both
/// sides of the table are read: the offer has to name an Island this seat
/// controls as well as the Elf across it, the Elf is the one that is answered,
/// and it has to land in *its owner's* hand rather than in the hand of the seat
/// that aimed the spell. Casting it in the opponent's own main phase is the
/// other half — the card is an instant, and a bounce that could only be played
/// on its controller's turn would look identical on every board above.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn boomerang_returns_a_permanent_across_the_table_to_its_owners_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[boomerang()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    // The non-active seat holding priority in the active seat's own main phase
    // is the half of "instant" no board on p0's turn can show.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let mine = on_battlefield(&engine, p0, island()).expect("my Island is on the table");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is on the table");
    let permanents = engine.state().zones.list(ZoneLocation::Battlefield).len();

    // Mana before the claim: the offer is read off the pool, not off the two
    // untapped Islands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "the two Islands are exactly the {{U}}{{U}} the card costs"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(&engine, p0, boomerang()).expect("the Boomerang is in hand");
    assert!(
        legal.castable.contains(&spell),
        "{{U}}{{U}} is in the pool and an instant may be cast in an opponent's \
         main phase: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("the mana already floating pays for it");

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the seat casting the spell is the one aiming it"
    );
    assert_eq!((min, max), (1, 1), "one permanent, no more and no fewer");
    assert!(
        player_options.is_empty(),
        "\"target permanent\" is not \"any target\": no player may be named \
         (CR 115.4): {player_options:?}"
    );
    assert_eq!(
        options.len(),
        permanents,
        "\"target permanent\" is every permanent in the game: {options:?}"
    );
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "an Island of mine and the Elf across the table are both permanents \
         (CR 110.1): {options:?}"
    );
    // Still in **hand**, and that is this engine's announcement rather than a
    // bug: `cast_wizard` asks every question CR 601.2b–h poses and moves the
    // card to the stack last, at CR 601.2i, so the whole announcement is
    // atomic from the outside. Nobody can tell: no player gets priority
    // until 601.2i, and CR 115.5 makes a spell an illegal target for
    // itself, so there is no legal question whose answer differs.
    assert!(
        in_hand(&engine, p0, boomerang()).is_some(),
        "the card has not reached the stack yet (CR 601.2i comes last)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the permanent it is aimed at has not moved: the effect resolves \
         off the stack"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf was one of the permanents the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes to the seat that owns it, not to \
         the seat that aimed the spell"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "and nothing of the opponent's reached the caster's hand"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_none(),
        "a bounce is not a destroy: the Elf is in a hand and in no graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, boomerang()).is_some(),
        "and the instant that resolved is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{U}}{{U}} came out of the pool"
    );
}

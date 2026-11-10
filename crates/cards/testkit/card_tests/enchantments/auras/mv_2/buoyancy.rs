//! `cards/enchantments/auras/mv_2/buoyancy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Buoyancy — {1}{U} Aura: flash, "Enchant creature", "Enchanted creature
/// has flying."
///
/// The cast happens in the *opponent's* first main phase, so the only thing
/// that lets the engine offer it there is the printed flash (CR 702.8) — a
/// card with the same cost and no flash would be refused for timing, and
/// playing it on its controller's own turn could not tell the two apart.
/// "Enchant creature" is a restriction on what the Aura may hold and not on
/// who controls it, so the offer is read across the whole table before the
/// answer is given; and the Elf that answer declines is the live 1/1 that
/// shows the grant landing on the creature the Aura holds and nowhere else.
#[test]
fn buoyancy_flashes_in_on_the_opponents_turn_and_grants_flying_only_to_its_host() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), quiet_creature()])
        .hand(0, &[buoyancy()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);

    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "nothing is enchanted yet"
    );

    // The opponent's first main phase, and then priority handed to the seat
    // that is not taking the turn (CR 117.3a).
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert_eq!(
        engine.state().turn.active,
        p1,
        "the Aura is still being cast on the opponent's turn"
    );

    // Mana first: the offer is read off the pool and not off the untapped
    // lands. The Elf is kept standing so that the question below is about a
    // creature and not about a tapped one.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    let aura = in_hand(&engine, p0, buoyancy()).expect("the Aura is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&aura),
        "flash (CR 702.8) is the whole of why an enchantment is castable in \
         the opponent's main phase: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, buoyancy());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("an Aura asks what it enchants, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"Enchant creature\" reaches any creature in the game, on either \
         side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| on_battlefield(e, p0, buoyancy()).is_some());

    let on_table = on_battlefield(&engine, p0, buoyancy()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(on_table).and_then(|o| o.attached_to),
        Some(mine),
        "the Aura holds the creature it was aimed at"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FLYING),
        "enchanted creature has flying"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "the Elf that was offered and declined is still a printed 1/1"
    );
    assert!(
        !keywords(&engine, on_table).contains(KeywordSet::FLYING),
        "the static is filtered to a creature, so the Aura bestows the \
         keyword rather than keeping it"
    );
}

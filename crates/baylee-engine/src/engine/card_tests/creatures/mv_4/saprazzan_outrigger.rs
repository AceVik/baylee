//! `cards/creatures/mv_4/saprazzan_outrigger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "cd57638e-8444-4522-8638-4514dec8b3ed"
fn saprazzan_outrigger() -> CardIndex {
    card_index("cd57638e-8444-4522-8638-4514dec8b3ed")
}

/// `Saprazzan Outrigger` (`Coverage::Generated`):
/// "When this creature attacks or blocks, put it on top of its owner's library at end of combat."
///
/// Verifies that when `Saprazzan Outrigger` attacks, it deals combat damage and
/// is then put on top of its owner's library at end of combat.
#[test]
fn saprazzan_outrigger_puts_itself_on_top_of_owners_library_at_end_of_combat() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9615, island())
        .battlefield(0, &[saprazzan_outrigger()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let outrigger =
        on_battlefield(&engine, p0, saprazzan_outrigger()).expect("P0 Saprazzan Outrigger");
    let lib_before = library_size(&engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(outrigger, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Advance past combat into postcombat main phase.
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(
        engine.state().players[1].life,
        15,
        "Saprazzan Outrigger dealt 5 combat damage before end of combat"
    );
    assert!(
        on_battlefield(&engine, p0, saprazzan_outrigger()).is_none(),
        "Saprazzan Outrigger left the battlefield at end of combat"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_before + 1,
        "Saprazzan Outrigger was put on top of its owner's library"
    );
}

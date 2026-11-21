//! `cards/artifacts/mv_0/mishra_s_bauble.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn mishra_s_bauble() -> CardIndex {
    card_index("63afc3d1-7653-476e-838f-fc18d4a62a21")
}

/// Mishra's Bauble (63afc3d1-7653-476e-838f-fc18d4a62a21) — {0} Artifact:
/// "{T}, Sacrifice this artifact: Look at the top card of target player's
///  library. Draw a card at the beginning of the next turn's upkeep."
///
/// Casts for {0}, sacrifices to look at the top card of the opponent's library
/// (leaving it on top without drawing immediately), and draws 1 card at the
/// beginning of the next turn's upkeep.
#[test]
fn mishra_s_bauble_peeks_target_library_top_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[mishra_s_bauble()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast for {0} with no lands on board.
    let bauble_hand = in_hand(&engine, p0, mishra_s_bauble()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bauble_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let bauble = on_battlefield(&engine, p0, mishra_s_bauble()).unwrap();

    let their_top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .expect("opponent has a library");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, mishra_s_bauble(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(bauble).unwrap().zone,
        Zone::Graveyard,
        "Mishra's Bauble sacrificed as cost"
    );

    // Resolving shows the top card of target player's library via LookAtLibrary.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected LookAtLibrary choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, ChoicePrompt::LookAtLibrary);
    assert_eq!((min, max), (0, 0));
    assert_eq!(
        options,
        vec![their_top],
        "shows exact top card of p1's library"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Top card stays on top of p1's library, and p0 has NOT drawn yet.
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p1))
            .last()
            .copied(),
        Some(their_top)
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "draw waits until the next turn's upkeep"
    );

    // Next turn (p1's upkeep): delayed trigger draws 1 card for p0.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "p0 drew 1 card at the beginning of the next turn's upkeep"
    );
}

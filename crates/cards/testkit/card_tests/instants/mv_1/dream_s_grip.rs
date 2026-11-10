//! `cards/instants/mv_1/dream_s_grip.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn dream_s_grip() -> CardIndex {
    card_index("8b239262-c392-4ec6-9ae1-8b4a076e4bf0")
}

/// Dream's Grip — {U} Instant
/// "Choose one —
/// • Tap target permanent.
/// • Untap target permanent.
/// Entwine {1}"
#[test]
fn dream_s_grip_taps_or_untaps_and_entwines_both() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[dream_s_grip()])
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    // Tap Llanowar Elves for mana and also tap the two Islands so {1}{U} is floating.
    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, elf));
    assert!(!is_tapped(&engine, giant));

    // Cast Dream's Grip entwined (CastModeKind::Modes(3)) to tap P1's Hill Giant and untap P0's Llanowar Elves.
    cast_with_floating(&mut engine, p0, dream_s_grip());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode, got {:?}", engine.pending());
    };
    let entwine_idx = options
        .iter()
        .position(|o| o.kind == CastModeKind::Modes(3))
        .expect("entwine mode offered");
    engine
        .apply(p0, PlayerAction::ChooseMode(entwine_idx))
        .unwrap();

    // Target 1 (Tap target permanent): giant
    aim_at(&mut engine, p0, giant);
    // Target 2 (Untap target permanent): elf
    aim_at(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, giant));
    assert!(!is_tapped(&engine, elf));
}

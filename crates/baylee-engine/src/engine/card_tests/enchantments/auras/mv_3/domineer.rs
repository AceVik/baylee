//! `cards/enchantments/auras/mv_3/domineer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::ORNITHOPTER;

fn domineer() -> CardIndex {
    card_index("ca283839-864f-4252-8452-480e3ba29b20")
}

/// Domineer — {1}{U}{U} Enchantment — Aura
/// "Enchant artifact creature"
/// "You control enchanted artifact creature."
#[test]
fn domineer_gains_control_of_enchanted_artifact_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[domineer()])
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[ORNITHOPTER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let thopter = on_battlefield(&engine, p1, ORNITHOPTER).unwrap();

    cast_from_hand(&mut engine, p0, domineer());
    aim_at(&mut engine, p0, thopter);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(thopter).unwrap().controller, p0);
    assert!(on_battlefield(&engine, p0, ORNITHOPTER).is_some());
    assert!(on_battlefield(&engine, p1, ORNITHOPTER).is_none());
}
